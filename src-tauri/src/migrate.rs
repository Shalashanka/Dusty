//! One-time moves from the app's previous name (Dustling).

use std::path::Path;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

pub fn run(data_dir: &Path) {
    let db = data_dir.join("dusty.db");
    if !db.exists() {
        if let Some(old) = dirs::data_local_dir().map(|d| d.join("Dustling")) {
            for (from, to) in [
                ("dustling.db", "dusty.db"),
                ("dustling.db-wal", "dusty.db-wal"),
            ] {
                let src = old.join(from);
                if src.exists() {
                    let _ = std::fs::copy(&src, data_dir.join(to));
                }
            }
        }
    }
    // The old build registered itself to start at login under its old name.
    if let Ok(run) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        winreg::enums::KEY_ALL_ACCESS,
    ) {
        if let Ok(v) = run.get_value::<String, _>("Dustling") {
            if v.to_lowercase().contains("dustling.exe") {
                let _ = run.delete_value("Dustling");
            }
        }
    }
}
