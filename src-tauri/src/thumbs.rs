//! Windows shell thumbnails: the same previews Explorer shows (photos,
//! videos, PDFs, PSDs, folders...), straight from the thumbnail cache.

use base64::Engine;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::OnceLock;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HGDIOBJ,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF, SIIGBF_BIGGERSIZEOK,
    SIIGBF_RESIZETOFIT, SIIGBF_THUMBNAILONLY,
};
use windows::core::HSTRING;

/// BGRA pixels, top-down.
pub struct Pixels {
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

fn com_init() {
    thread_local!(static INIT: () = unsafe { let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); });
    INIT.with(|_| ());
}

pub fn pixels(path: &str, size: i32, thumbnail_only: bool) -> Option<Pixels> {
    com_init();
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
        let mut flags = SIIGBF(SIIGBF_RESIZETOFIT.0 | SIIGBF_BIGGERSIZEOK.0);
        if thumbnail_only {
            flags = SIIGBF(flags.0 | SIIGBF_THUMBNAILONLY.0);
        }
        let hbmp = factory.GetImage(SIZE { cx: size, cy: size }, flags).ok()?;
        let mut bm = BITMAP::default();
        let got = GetObjectW(
            HGDIOBJ(hbmp.0),
            size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );
        let result = if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
            None
        } else {
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut buf = vec![0u8; (w * h * 4) as usize];
            let dc = CreateCompatibleDC(None);
            let lines = GetDIBits(
                dc,
                hbmp,
                0,
                h as u32,
                Some(buf.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(dc);
            (lines > 0).then_some(Pixels {
                w: w as u32,
                h: h as u32,
                bgra: buf,
            })
        };
        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        result
    }
}

fn to_png(p: &Pixels) -> Option<Vec<u8>> {
    // Shell bitmaps are premultiplied BGRA; some have no alpha at all.
    let has_alpha = p.bgra.chunks_exact(4).any(|c| c[3] != 0);
    let mut rgba = Vec::with_capacity(p.bgra.len());
    for c in p.bgra.chunks_exact(4) {
        let a = if has_alpha { c[3] } else { 255 };
        let un = |v: u8| {
            if a == 0 || a == 255 {
                v
            } else {
                ((v as u32 * 255) / a as u32).min(255) as u8
            }
        };
        rgba.extend_from_slice(&[un(c[2]), un(c[1]), un(c[0]), a]);
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, p.w, p.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().ok()?;
        w.write_image_data(&rgba).ok()?;
    }
    Some(out)
}

fn cache() -> &'static Mutex<HashMap<(String, i32), String>> {
    static C: OnceLock<Mutex<HashMap<(String, i32), String>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A `data:image/png;base64,...` URL, or None when Windows has no preview.
pub fn data_url(path: &str, size: i32) -> Option<String> {
    let key = (path.to_lowercase(), size);
    if let Some(v) = cache().lock().get(&key) {
        return Some(v.clone());
    }
    let px = pixels(path, size, false)?;
    let png = to_png(&px)?;
    let url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let mut c = cache().lock();
    if c.len() > 400 {
        c.clear();
    }
    c.insert(key, url.clone());
    Some(url)
}

/// 64-bit difference hash of an image's thumbnail (for near-duplicate photos).
pub fn dhash(path: &str) -> Option<u64> {
    let px = pixels(path, 64, true)?;
    // Downsample to 9x8 grayscale by box averaging.
    let (w, h) = (px.w as usize, px.h as usize);
    let mut g = [[0f32; 9]; 8];
    for (y, row) in g.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let (x0, x1) = (x * w / 9, ((x + 1) * w / 9).max(x * w / 9 + 1));
            let (y0, y1) = (y * h / 8, ((y + 1) * h / 8).max(y * h / 8 + 1));
            let mut sum = 0f32;
            let mut n = 0f32;
            for yy in y0..y1.min(h) {
                for xx in x0..x1.min(w) {
                    let i = (yy * w + xx) * 4;
                    sum += 0.114 * px.bgra[i] as f32
                        + 0.587 * px.bgra[i + 1] as f32
                        + 0.299 * px.bgra[i + 2] as f32;
                    n += 1.0;
                }
            }
            *cell = if n > 0.0 { sum / n } else { 0.0 };
        }
    }
    let mut hash = 0u64;
    for (y, row) in g.iter().enumerate() {
        for x in 0..8 {
            if row[x] > row[x + 1] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    Some(hash)
}
