//! Placement and motion of the floating bar (search / commands / voice).
//!
//! The bar never "jumps": search and commands share one anchor (top edge fixed
//! at ~20 % of the screen, growing downwards), the voice pill sits low and
//! centred, and every change of size or anchor is a short native tween of the
//! window rectangle, so one surface visibly morphs into the other.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewWindow};

pub const SEARCH_W: f64 = 760.0;
pub const VOICE_W: f64 = 560.0;
pub const VOICE_H: f64 = 168.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Where the bar is right now (physical pixels), as last placed by us.
static CURRENT: Mutex<Option<Rect>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Work area of the monitor under the mouse (falls back to the primary one).
fn work_area(w: &WebviewWindow) -> Option<(Rect, f64)> {
    let pos = w.cursor_position().ok();
    let monitors = w.available_monitors().ok()?;
    let m = pos
        .and_then(|p| {
            monitors.iter().find(|m| {
                let (mp, ms) = (m.position(), m.size());
                p.x >= mp.x as f64 && p.y >= mp.y as f64 && p.x < (mp.x + ms.width as i32) as f64 && p.y < (mp.y + ms.height as i32) as f64
            })
        })
        .cloned()
        .or_else(|| w.primary_monitor().ok().flatten())?;
    let wa = m.work_area();
    Some((
        Rect { x: wa.position.x as f64, y: wa.position.y as f64, w: wa.size.width as f64, h: wa.size.height as f64 },
        m.scale_factor(),
    ))
}

/// Target rectangle for a mode, given the content height (logical px).
fn target(w: &WebviewWindow, mode: &str, height: f64) -> Option<Rect> {
    let (wa, s) = work_area(w)?;
    Some(if mode == "voice" {
        let (bw, bh) = (VOICE_W * s, VOICE_H * s);
        Rect { x: wa.x + (wa.w - bw) / 2.0, y: wa.y + wa.h - bh - 72.0 * s, w: bw, h: bh }
    } else {
        let bw = SEARCH_W * s;
        let bh = height.max(64.0) * s;
        Rect { x: wa.x + (wa.w - bw) / 2.0, y: wa.y + (wa.h * 0.2).round(), w: bw, h: bh }
    })
}

fn place(w: &WebviewWindow, r: Rect) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};
        if let Ok(h) = w.hwnd() {
            let hwnd = HWND(h.0 as *mut core::ffi::c_void);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.x.round() as i32,
                    r.y.round() as i32,
                    r.w.round() as i32,
                    r.h.round() as i32,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    *CURRENT.lock() = Some(r);
}

/// Puts the bar where `mode` belongs, without animation (used when it opens).
pub fn snap(app: &AppHandle, mode: &str, height: f64) {
    let Some(w) = app.get_webview_window("search") else { return };
    GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Some(r) = target(&w, mode, height) {
        place(&w, r);
    }
}

/// Moves/resizes the bar to fit `mode` and `height` with an eased tween.
/// Changing anchor (voice -> search) takes longer, so the morph reads clearly.
pub fn fit(app: &AppHandle, mode: &str, height: f64) {
    let Some(w) = app.get_webview_window("search") else { return };
    let Some(to) = target(&w, mode, height) else { return };
    let from = CURRENT.lock().unwrap_or(to);
    if from == to {
        return;
    }
    if !w.is_visible().unwrap_or(false) {
        place(&w, to);
        return;
    }
    let moving = (from.y - to.y).abs() > 40.0 || (from.w - to.w).abs() > 4.0;
    let ms = if moving { 520.0 } else { 200.0 };
    let id = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            if GENERATION.load(Ordering::SeqCst) != id {
                return;
            }
            let t = (start.elapsed().as_secs_f64() * 1000.0 / ms).min(1.0);
            // easeOutQuint for size, a softer easeInOutCubic for travel.
            let e = 1.0 - (1.0 - t).powi(5);
            let m = if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 };
            let k = if moving { m } else { e };
            let lerp = |a: f64, b: f64| a + (b - a) * k;
            place(&w, Rect { x: lerp(from.x, to.x), y: lerp(from.y, to.y), w: lerp(from.w, to.w), h: lerp(from.h, to.h) });
            if t >= 1.0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(7));
        }
    });
}
