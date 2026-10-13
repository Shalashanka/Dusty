//! Thin wrappers over the Win32 bits Dustling needs.

use std::path::Path;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN, THREAD_MODE_BACKGROUND_END,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Shell::{
    SHEmptyRecycleBinW, SHQueryRecycleBinW, SHQueryUserNotificationState, QUNS_ACCEPTS_NOTIFICATIONS,
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHERB_NOCONFIRMATION,
    SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHQUERYRBINFO,
};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

/// Seconds since the last keyboard/mouse input.
pub fn idle_secs() -> u64 {
    let mut lii = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        if !GetLastInputInfo(&mut lii).as_bool() {
            return 0;
        }
        let tick = GetTickCount64() as u32;
        (tick.wrapping_sub(lii.dwTime) / 1000) as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    /// Normal desktop use: notifications welcome.
    Available,
    /// Fullscreen game, presentation, fullscreen app.
    Busy,
    /// Locked, screensaver, Focus Assist, etc.
    Away,
}

pub fn user_state() -> UserState {
    match unsafe { SHQueryUserNotificationState() } {
        Ok(s) if s == QUNS_ACCEPTS_NOTIFICATIONS => UserState::Available,
        Ok(s) if s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE => {
            UserState::Busy
        }
        Ok(_) => UserState::Away,
        Err(_) => UserState::Available,
    }
}

/// True when running on battery below 30%.
pub fn low_battery() -> bool {
    let mut st = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut st) }.is_err() {
        return false;
    }
    st.ACLineStatus == 0 && st.BatteryLifePercent != 255 && st.BatteryLifePercent < 30
}

/// Background mode lowers the calling thread's CPU, I/O and memory priority.
pub fn set_background_mode(on: bool) {
    unsafe {
        let mode = if on { THREAD_MODE_BACKGROUND_BEGIN } else { THREAD_MODE_BACKGROUND_END };
        let _ = SetThreadPriority(GetCurrentThread(), mode);
    }
}

/// (bytes, item count) across all drives' recycle bins.
pub fn recycle_bin_info() -> (u64, u64) {
    let mut info = SHQUERYRBINFO { cbSize: size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
    match unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) } {
        Ok(()) => (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64),
        Err(_) => (0, 0),
    }
}

pub fn empty_recycle_bin() -> Result<(), String> {
    unsafe {
        SHEmptyRecycleBinW(None, PCWSTR::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND)
    }
    .map_err(|e| e.message())
}

/// The URL a file was downloaded from, from the `Zone.Identifier` stream
/// browsers attach to downloads.
pub fn download_origin(path: &Path) -> Option<String> {
    let mut ads = path.as_os_str().to_owned();
    ads.push(":Zone.Identifier");
    let text = std::fs::read_to_string(&ads).ok()?;
    let mut referrer = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("HostUrl=") {
            if v.starts_with("http") {
                return Some(v.trim().to_string());
            }
        }
        if let Some(v) = line.strip_prefix("ReferrerUrl=") {
            if v.starts_with("http") {
                referrer = Some(v.trim().to_string());
            }
        }
    }
    referrer
}

/// Display names of installed programs (Uninstall registry keys).
pub fn installed_programs() -> Vec<String> {
    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut out = Vec::new();
    for (hive, path) in [
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL32),
        (HKEY_CURRENT_USER, UNINSTALL),
    ] {
        let Ok(key) = RegKey::predef(hive).open_subkey(path) else { continue };
        for name in key.enum_keys().flatten() {
            if let Ok(sub) = key.open_subkey(&name) {
                if let Ok(dn) = sub.get_value::<String, _>("DisplayName") {
                    out.push(dn);
                }
            }
        }
    }
    out
}

/// Registers an AppUserModelID so toasts show "Dustling" with our icon even
/// when running unpackaged.
pub fn register_app_id(app_id: &str, display_name: &str, icon: &Path) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok((key, _)) = hkcu.create_subkey(format!(r"Software\Classes\AppUserModelId\{app_id}")) else {
        return false;
    };
    let ok = key.set_value("DisplayName", &display_name).is_ok()
        && key.set_value("IconUri", &icon.to_string_lossy().as_ref()).is_ok()
        && key.set_value("IconBackgroundColor", &"FF17151F").is_ok();
    unsafe {
        let _ = windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(w!("com.griz.dustling"));
    }
    ok
}

/// Opens Explorer with the item selected.
pub fn reveal(path: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn();
}
