//! Organizing plans: for each item of a folder, where should it go and why?

use crate::detect::dev::Listing;
use crate::settings::{PolicyMode, Settings};
use crate::timeutil::now;
use crate::walk::{self, ext_of, is_hidden, name_of};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub modified: i64,
    pub category: String,
    /// "move" | "recycle" | "leave"
    pub action: String,
    pub dest: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub folder: String,
    pub mode: PolicyMode,
    pub items: Vec<PlanItem>,
}

/// Category + a short explanation of how it was recognised.
pub fn classify(path: &Path, is_dir: bool) -> (&'static str, String) {
    let name = name_of(path).to_lowercase();
    let ext = ext_of(path);
    if is_dir {
        let l = Listing::of(path);
        if l.is_project() {
            return ("code", "Looks like a code project.".into());
        }
        // Folder full of one kind of thing?
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(path) {
            for e in rd.flatten().take(200) {
                let p = e.path();
                if p.is_file() {
                    n += 1;
                    *counts.entry(classify(&p, false).0).or_default() += 1;
                }
            }
        }
        if let Some((cat, c)) = counts.iter().max_by_key(|(_, c)| **c) {
            if n >= 3 && *c * 10 >= n * 8 && !matches!(*cat, "other" | "folder") {
                return (cat, format!("A folder of {c} {}.", plural(cat)));
            }
        }
        return ("folder", "A folder of mixed things.".into());
    }
    let cat: &str = match ext.as_str() {
        "exe" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" => "installer",
        "zip" | "7z" | "rar" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "zst" => "archive",
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "pages" => "document",
        "xls" | "xlsx" | "ods" | "numbers" => "spreadsheet",
        "ppt" | "pptx" | "odp" | "key" => "presentation",
        "epub" | "mobi" | "azw3" => "ebook",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "bmp" | "tif" | "tiff" | "avif"
        | "raw" | "cr2" | "nef" | "svg" => {
            if name.contains("screenshot")
                || name.contains("screen shot")
                || name.contains("schermata")
                || name.starts_with("snip")
            {
                "screenshot"
            } else {
                "image"
            }
        }
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v" => "video",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => "audio",
        "bat" | "cmd" | "ps1" | "psm1" | "sh" | "ahk" | "vbs" | "reg" => "script",
        "py" | "js" | "ts" | "rs" | "go" | "java" | "kt" | "c" | "cpp" | "h" | "cs" | "rb"
        | "php" | "ipynb" | "html" | "css" => "code",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "iso" | "img" | "vhd" | "vhdx" | "dmg" => "disk_image",
        "psd" | "ai" | "fig" | "sketch" | "xd" | "blend" | "afdesign" | "afphoto" | "fbx"
        | "obj" | "stl" => "design",
        "csv" | "json" | "xml" | "sql" | "db" | "sqlite" | "parquet" | "yaml" | "yml" => "data",
        "torrent" => "torrent",
        "crdownload" | "part" | "partial" | "download" | "opdownload" => "partial",
        "lnk" | "url" => "shortcut",
        _ => "other",
    };
    let why = match cat {
        "installer" => "A program installer.".to_string(),
        "script" => {
            format!("A .{ext} script, probably something you run. Scripts get a home of their own.")
        }
        "partial" => "A download that never finished.".into(),
        "torrent" => "A .torrent file; the download it points to is what matters.".into(),
        "screenshot" => "A screenshot.".into(),
        "shortcut" => "A shortcut; those can stay.".into(),
        "other" => format!("I don't recognise .{ext} files, so they go to Unsorted."),
        c => format!("{} file.", capital(c.replace('_', " ").as_str())),
    };
    (cat, why)
}

fn plural(cat: &str) -> &'static str {
    match cat {
        "image" => "images",
        "screenshot" => "screenshots",
        "video" => "videos",
        "audio" => "audio files",
        "document" => "documents",
        "installer" => "installers",
        "archive" => "archives",
        "script" => "scripts",
        "code" => "code files",
        "design" => "design files",
        "font" => "fonts",
        _ => "files",
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Proposes where everything in `folder` should go.
pub fn make_plan(
    s: &Settings,
    folder: &str,
    installed: &[String],
    overrides: &std::collections::HashMap<String, String>,
) -> Plan {
    let policy = s
        .policies
        .iter()
        .find(|p| p.path.eq_ignore_ascii_case(folder))
        .cloned()
        .unwrap_or_default();
    let grace = policy.grace_hours as i64 * 3600;
    let sorted = policy.mode == PolicyMode::Sorted;
    let mut items = Vec::new();
    let dir_names: std::collections::HashSet<String> = walk::children(Path::new(folder))
        .iter()
        .filter(|(_, m)| m.is_dir())
        .map(|(p, _)| name_of(p).to_lowercase())
        .collect();
    let category_dirs: std::collections::HashSet<String> = s
        .sort_dest
        .keys()
        .map(|k| category_folder(k).to_lowercase())
        .collect();

    for (path, md) in walk::children(Path::new(folder)) {
        let name = name_of(&path);
        if is_hidden(&md)
            || name.eq_ignore_ascii_case("desktop.ini")
            || name == crate::detect::SWEPT_DIR
        {
            continue;
        }
        // In "sorted" mode the category folders themselves are the result.
        if sorted && md.is_dir() && category_dirs.contains(&name.to_lowercase()) {
            continue;
        }
        let modified = walk::touched(&md);
        let fresh = now() - modified < grace;
        let (cat, why) = classify(&path, md.is_dir());
        let ext = ext_of(&path);
        let bytes = if md.is_dir() {
            walk::dir_stats(&path).bytes
        } else {
            md.len()
        };
        let mut reason = why;
        let mut action = "move".to_string();
        let mut dest = if sorted {
            format!("{folder}\\{}", category_folder(cat))
        } else {
            overrides
                .get(&format!("ext:{ext}"))
                .cloned()
                .unwrap_or_else(|| s.dest_for(cat))
        };
        if dest == "recycle" {
            action = "recycle".into();
            dest.clear();
        } else if dest == "leave" {
            action = "leave".into();
            dest.clear();
        }
        if cat == "installer" {
            let stem = path
                .file_stem()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(app) = crate::detect::inbox::match_installed(&stem, installed) {
                action = "recycle".into();
                dest.clear();
                reason = format!("{app} is already installed, so this installer can go.");
            }
        }
        if cat == "archive" {
            let stem = name.to_lowercase();
            let stem = stem.rsplit_once('.').map(|(a, _)| a).unwrap_or(&stem);
            if dir_names.contains(stem) {
                action = "recycle".into();
                dest.clear();
                reason = "Already extracted right next to it.".into();
            }
        }
        if fresh {
            action = "leave".into();
            reason = format!(
                "Arrived recently, so I'll give it {} hours. {reason}",
                policy.grace_hours
            );
        }
        items.push(PlanItem {
            path: path.to_string_lossy().into_owned(),
            name,
            is_dir: md.is_dir(),
            bytes,
            modified,
            category: cat.into(),
            action,
            dest,
            reason,
        });
    }
    items.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Plan {
        folder: folder.to_string(),
        mode: policy.mode,
        items,
    }
}

pub fn category_folder(cat: &str) -> &'static str {
    match cat {
        "installer" => "Installers",
        "archive" => "Archives",
        "document" | "spreadsheet" | "presentation" => "Documents",
        "ebook" => "Books",
        "image" => "Images",
        "screenshot" => "Screenshots",
        "video" => "Videos",
        "audio" => "Audio",
        "script" => "Scripts",
        "code" => "Code",
        "font" => "Fonts",
        "disk_image" => "Disk Images",
        "design" => "Design",
        "data" => "Data",
        "folder" => "Folders",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_files() {
        assert_eq!(classify(Path::new("C:\\x\\launch.bat"), false).0, "script");
        assert_eq!(
            classify(Path::new("C:\\x\\Screenshot 2026-01-01.png"), false).0,
            "screenshot"
        );
        assert_eq!(classify(Path::new("C:\\x\\holiday.jpg"), false).0, "image");
        assert_eq!(
            classify(Path::new("C:\\x\\setup.exe"), false).0,
            "installer"
        );
        assert_eq!(
            classify(Path::new("C:\\x\\a.crdownload"), false).0,
            "partial"
        );
        assert_eq!(classify(Path::new("C:\\x\\font.ttf"), false).0, "font");
    }

    #[test]
    fn plans_a_folder() {
        let d = std::env::temp_dir().join(format!("dusty-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tool")).unwrap();
        std::fs::write(d.join("run.bat"), b"echo").unwrap();
        std::fs::write(d.join("tool.zip"), b"zip").unwrap();
        let mut s = Settings::default();
        s.policies = vec![crate::settings::FolderPolicy {
            path: d.to_string_lossy().into_owned(),
            mode: PolicyMode::Zero,
            grace_hours: 0,
            ask_every_days: 1,
        }];
        let plan = make_plan(&s, &d.to_string_lossy(), &[], &Default::default());
        let bat = plan.items.iter().find(|i| i.name == "run.bat").unwrap();
        assert_eq!(bat.action, "move");
        assert!(bat.dest.ends_with("Scripts"));
        let zip = plan.items.iter().find(|i| i.name == "tool.zip").unwrap();
        assert_eq!(zip.action, "recycle");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
