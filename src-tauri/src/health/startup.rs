//! Startup apps: the same places (and enable/disable switches) Task Manager uses.

use serde::Serialize;
use std::path::PathBuf;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE};
use winreg::{RegKey, RegValue};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    /// "<source>|<name>", stable across runs.
    pub id: String,
    pub name: String,
    pub command: String,
    pub source: String,
    pub enabled: bool,
    /// Changing it requires administrator rights (machine-wide entries).
    pub machine: bool,
    pub exe: Option<String>,
}

struct Source {
    key: &'static str,
    machine: bool,
    reg: Option<&'static str>,
    folder: Option<PathBuf>,
    approved: &'static str,
}

fn sources() -> Vec<Source> {
    let user_folder =
        dirs::data_dir().map(|d| d.join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    let common_folder =
        PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()))
            .join(r"Microsoft\Windows\Start Menu\Programs\StartUp");
    vec![
        Source {
            key: "user-run",
            machine: false,
            reg: Some(RUN),
            folder: None,
            approved: "Run",
        },
        Source {
            key: "machine-run",
            machine: true,
            reg: Some(RUN),
            folder: None,
            approved: "Run",
        },
        Source {
            key: "machine-run32",
            machine: true,
            reg: Some(RUN32),
            folder: None,
            approved: "Run32",
        },
        Source {
            key: "user-folder",
            machine: false,
            reg: None,
            folder: user_folder,
            approved: "StartupFolder",
        },
        Source {
            key: "common-folder",
            machine: true,
            reg: None,
            folder: Some(common_folder),
            approved: "StartupFolder",
        },
    ]
}

fn hive(machine: bool) -> RegKey {
    RegKey::predef(if machine {
        HKEY_LOCAL_MACHINE
    } else {
        HKEY_CURRENT_USER
    })
}

fn is_enabled(machine: bool, approved: &str, name: &str) -> bool {
    hive(machine)
        .open_subkey_with_flags(format!(r"{APPROVED}\{approved}"), KEY_READ)
        .and_then(|k| k.get_raw_value(name))
        .map(|v| v.bytes.first().map(|b| b % 2 == 0).unwrap_or(true))
        .unwrap_or(true)
}

/// First token of a command line: `"C:\x y\a.exe" --flag` -> `C:\x y\a.exe`.
pub fn exe_of(cmd: &str) -> Option<String> {
    let c = cmd.trim();
    if let Some(rest) = c.strip_prefix('"') {
        return rest.split('"').next().map(|s| s.to_string());
    }
    let low = c.to_lowercase();
    if let Some(i) = low.find(".exe") {
        return Some(c[..i + 4].to_string());
    }
    c.split_whitespace().next().map(|s| s.to_string())
}

pub fn list() -> Vec<StartupItem> {
    let mut out = Vec::new();
    for src in sources() {
        if let Some(reg) = src.reg {
            let Ok(k) = hive(src.machine).open_subkey_with_flags(reg, KEY_READ) else {
                continue;
            };
            for (name, value) in k.enum_values().flatten() {
                let command = value.to_string().trim_matches('"').to_string();
                let command = if value.to_string().starts_with('"') {
                    value.to_string()
                } else {
                    command
                };
                out.push(StartupItem {
                    id: format!("{}|{name}", src.key),
                    enabled: is_enabled(src.machine, src.approved, &name),
                    exe: exe_of(&command),
                    name,
                    command,
                    source: src.key.into(),
                    machine: src.machine,
                });
            }
        }
        if let Some(folder) = &src.folder {
            let Ok(rd) = std::fs::read_dir(folder) else {
                continue;
            };
            for e in rd.flatten() {
                let file = e.file_name().to_string_lossy().into_owned();
                if file.eq_ignore_ascii_case("desktop.ini") {
                    continue;
                }
                let name = file
                    .rsplit_once('.')
                    .map(|(n, _)| n.to_string())
                    .unwrap_or(file.clone());
                out.push(StartupItem {
                    id: format!("{}|{file}", src.key),
                    enabled: is_enabled(src.machine, src.approved, &file),
                    exe: Some(e.path().to_string_lossy().into_owned()),
                    command: e.path().to_string_lossy().into_owned(),
                    name,
                    source: src.key.into(),
                    machine: src.machine,
                });
            }
        }
    }
    out
}

/// Flips Task Manager's enabled flag. Machine-wide entries need a UAC prompt.
pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    let (key, name) = id.split_once('|').ok_or("bad startup id")?;
    let src = sources()
        .into_iter()
        .find(|s| s.key == key)
        .ok_or("unknown startup location")?;
    let mut bytes = vec![if enabled { 2u8 } else { 3u8 }, 0, 0, 0];
    if enabled {
        bytes.extend([0u8; 8]);
    } else {
        let ft = (crate::timeutil::now() as u64 + 11_644_473_600) * 10_000_000;
        bytes.extend(ft.to_le_bytes());
    }
    let path = format!(r"{APPROVED}\{}", src.approved);
    if src.machine {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let args = format!(r#"add "HKLM\{path}" /v "{name}" /t REG_BINARY /d {hex} /f"#);
        let code = crate::winsys::run_elevated("reg.exe", &args)?;
        return if code == 0 {
            Ok(())
        } else {
            Err(format!("reg.exe failed ({code})"))
        };
    }
    let (k, _) = hive(false)
        .create_subkey(&path)
        .map_err(|e| e.to_string())?;
    let k = hive(false)
        .open_subkey_with_flags(&path, KEY_SET_VALUE)
        .unwrap_or(k);
    k.set_raw_value(
        name,
        &RegValue {
            bytes: bytes.into(),
            vtype: winreg::enums::RegType::REG_BINARY,
        },
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exe() {
        assert_eq!(
            exe_of(r#""C:\Program Files\A b\app.exe" --min"#).as_deref(),
            Some(r"C:\Program Files\A b\app.exe")
        );
        assert_eq!(
            exe_of(r"C:\Tools\x.exe /silent").as_deref(),
            Some(r"C:\Tools\x.exe")
        );
        // Reading never fails, even if empty.
        let _ = list();
    }
}
