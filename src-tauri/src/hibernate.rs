//! Project hibernation: zip a sleeping project into cold storage, verify the
//! archive byte-for-byte, then replace the folder with a small note.
//! Waking it up extracts everything back into place.

use crate::detect::dev::{Listing, is_junk};
use crate::walk::{self, is_link, name_of};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

pub const NOTE: &str = "DUSTY-HIBERNATED.txt";

pub struct Outcome {
    pub archive: PathBuf,
    pub files: u64,
    pub original_bytes: u64,
    pub archive_bytes: u64,
}

/// Every file to keep (relative path, absolute path), skipping rebuildable junk.
fn files_to_keep(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let listing = Listing::of(&dir);
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            if is_link(&md) {
                continue;
            }
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_lowercase();
            if md.is_dir() {
                if !is_junk(&name, &p, &listing) {
                    stack.push(p);
                }
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push((rel.to_string_lossy().replace('\\', "/"), p));
            }
        }
    }
    out
}

pub fn hibernate(project: &Path, cold_storage: &Path) -> Result<Outcome, String> {
    if !project.is_dir() {
        return Err("the project folder is gone".into());
    }
    if project.join(NOTE).exists() {
        return Err("this project is already hibernating".into());
    }
    fs::create_dir_all(cold_storage).map_err(|e| e.to_string())?;
    let archive = walk::unique_dest(
        cold_storage,
        &format!("{}-{}.zip", name_of(project), crate::timeutil::day_stamp()),
    );
    let files = files_to_keep(project);
    let original_bytes = walk::dir_stats(project).bytes;

    // 1. Write the archive.
    {
        let f = File::create(&archive).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(BufWriter::new(f));
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let mut buf = vec![0u8; 1 << 20];
        for (rel, abs) in &files {
            zip.start_file(rel.as_str(), opts)
                .map_err(|e| e.to_string())?;
            let mut src = BufReader::new(File::open(abs).map_err(|e| format!("{rel}: {e}"))?);
            loop {
                let n = src.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                zip.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }

    // 2. Verify: every entry decompresses with a matching CRC.
    if let Err(e) = verify(&archive, files.len()) {
        let _ = fs::remove_file(&archive);
        return Err(format!(
            "the archive didn't verify, nothing was removed ({e})"
        ));
    }

    // 3. Replace the folder with a note.
    walk::remove_tree(project).map_err(|e| e.to_string())?;
    fs::create_dir_all(project).map_err(|e| e.to_string())?;
    let note = format!(
        "This project is hibernating.\r\n\r\nDusty zipped it on {} to:\r\n{}\r\n\r\nTo wake it up, open Dusty > History and press Undo,\r\nor just extract the zip back into this folder.\r\n(Rebuildable folders like node_modules were left out; reinstall them as usual.)\r\n",
        crate::timeutil::day_stamp(),
        archive.display()
    );
    fs::write(project.join(NOTE), note).map_err(|e| e.to_string())?;
    let archive_bytes = fs::metadata(&archive).map(|m| m.len()).unwrap_or(0);
    Ok(Outcome {
        archive,
        files: files.len() as u64,
        original_bytes,
        archive_bytes,
    })
}

fn verify(archive: &Path, expected: usize) -> Result<(), String> {
    let f = File::open(archive).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(BufReader::new(f)).map_err(|e| e.to_string())?;
    if z.len() != expected {
        return Err(format!("{} of {expected} files in the archive", z.len()));
    }
    for i in 0..z.len() {
        let mut entry = z.by_index(i).map_err(|e| e.to_string())?;
        // Reading to the end checks the CRC.
        io::copy(&mut entry, &mut io::sink()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn wake(project: &Path, archive: &Path) -> Result<(), String> {
    if !archive.is_file() {
        return Err(format!("the archive is missing: {}", archive.display()));
    }
    fs::create_dir_all(project).map_err(|e| e.to_string())?;
    let leftovers: Vec<_> = fs::read_dir(project)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.file_name() != NOTE)
        .collect();
    if !leftovers.is_empty() {
        return Err("the project folder isn't empty anymore; move its contents away first".into());
    }
    let f = File::open(archive).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(BufReader::new(f)).map_err(|e| e.to_string())?;
    z.extract(project).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(project.join(NOTE));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hibernate_and_wake_roundtrip() {
        let d = std::env::temp_dir().join(format!("dusty-hib-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let proj = d.join("proj");
        fs::create_dir_all(proj.join("src")).unwrap();
        fs::create_dir_all(proj.join("node_modules/pkg")).unwrap();
        fs::write(proj.join("package.json"), b"{}").unwrap();
        fs::write(proj.join("src/main.js"), b"console.log(1)").unwrap();
        fs::write(proj.join("node_modules/pkg/index.js"), b"junk").unwrap();

        let out = hibernate(&proj, &d.join("cold")).unwrap();
        assert_eq!(out.files, 2);
        assert!(proj.join(NOTE).exists());
        assert!(!proj.join("src").exists());

        wake(&proj, &out.archive).unwrap();
        assert_eq!(
            fs::read_to_string(proj.join("src/main.js")).unwrap(),
            "console.log(1)"
        );
        assert!(!proj.join(NOTE).exists());
        assert!(!proj.join("node_modules").exists());
        fs::remove_dir_all(&d).unwrap();
    }
}
