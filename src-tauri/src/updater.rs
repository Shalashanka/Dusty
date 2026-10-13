//! Self-updates from a folder or a web address.
//!
//! The source holds a Tauri-style `latest.json` next to the signed installer:
//! ```json
//! { "version": "0.3.0", "notes": "...", "pub_date": "...",
//!   "platforms": { "windows-x86_64": { "url": "Dusty_0.3.0_x64-setup.exe", "signature": "<.sig contents>" } } }
//! ```
//! `url` may be relative to the source. Installers are verified with the
//! public key below before anything runs; a tampered file is refused.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Where updates come from unless the user picks something else.
/// Change to the public URL of `latest.json` before shipping to others.
pub const DEFAULT_SOURCE: &str = "C:\\dev\\Dusty\\releases";

/// minisign public key (base64 of the .pub file) matching ~/.tauri/dusty.key.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDRCRTZCRDk4MEI2MDA4NkQKUldSdENHQUxtTDNtUzF5YlNVN1h1bTlLYzEvQU5Ybk52NWFZS1lMbTJBbnVCU0J6ZkJsT3U2czYK";

#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    pub_date: String,
    platforms: HashMap<String, Platform>,
}

#[derive(Debug, Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub available: Option<String>,
    pub notes: String,
    pub date: String,
    pub source: String,
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn read_text(src: &str) -> Result<String, String> {
    if is_url(src) {
        ureq::get(src).call().map_err(|e| e.to_string())?.into_body().read_to_string().map_err(|e| e.to_string())
    } else {
        std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))
    }
}

fn read_bytes(src: &str) -> Result<Vec<u8>, String> {
    if is_url(src) {
        let mut v = Vec::new();
        ureq::get(src)
            .call()
            .map_err(|e| e.to_string())?
            .into_body()
            .into_with_config()
            .limit(1 << 30)
            .reader()
            .read_to_end(&mut v)
            .map_err(|e| e.to_string())?;
        Ok(v)
    } else {
        std::fs::read(src).map_err(|e| format!("{src}: {e}"))
    }
}

/// `source` is a folder, a path to latest.json, or a URL to latest.json.
fn manifest_location(source: &str) -> String {
    let s = source.trim();
    if is_url(s) || s.to_lowercase().ends_with(".json") {
        s.to_string()
    } else {
        Path::new(s).join("latest.json").to_string_lossy().into_owned()
    }
}

fn resolve(manifest_at: &str, url: &str) -> String {
    if is_url(url) || Path::new(url).is_absolute() {
        return url.to_string();
    }
    if is_url(manifest_at) {
        let base = manifest_at.rsplit_once('/').map(|(b, _)| b).unwrap_or(manifest_at);
        format!("{base}/{url}")
    } else {
        Path::new(manifest_at).parent().unwrap_or(Path::new(".")).join(url).to_string_lossy().into_owned()
    }
}

/// "0.2.10" > "0.2.9"
pub fn newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v').split(['.', '-', '+']).take(3).map(|p| p.parse().unwrap_or(0)).collect()
    };
    parse(candidate) > parse(current)
}

fn load(source: &str) -> Result<(String, Manifest), String> {
    let at = manifest_location(source);
    let m: Manifest = serde_json::from_str(&read_text(&at)?).map_err(|e| format!("bad latest.json: {e}"))?;
    Ok((at, m))
}

pub fn check(source: &str, current: &str) -> Result<UpdateInfo, String> {
    let (_, m) = load(source)?;
    Ok(UpdateInfo {
        current: current.to_string(),
        available: newer(&m.version, current).then(|| m.version.clone()),
        notes: m.notes,
        date: m.pub_date,
        source: source.to_string(),
    })
}

fn verify(bytes: &[u8], signature_b64: &str) -> Result<(), String> {
    let decode = |s: &str| -> Result<String, String> {
        let raw = base64::engine::general_purpose::STANDARD.decode(s.trim()).map_err(|e| e.to_string())?;
        String::from_utf8(raw).map_err(|e| e.to_string())
    };
    let pk = minisign_verify::PublicKey::decode(&decode(PUBLIC_KEY)?).map_err(|e| e.to_string())?;
    let sig = minisign_verify::Signature::decode(&decode(signature_b64)?).map_err(|e| e.to_string())?;
    pk.verify(bytes, &sig, false).map_err(|_| "the update's signature doesn't match. Refusing to install it.".to_string())
}

/// Fetches + verifies the installer and returns where it was saved.
pub fn download(source: &str) -> Result<(String, PathBuf), String> {
    let (at, m) = load(source)?;
    let p = m.platforms.get("windows-x86_64").ok_or("no Windows build in this update")?;
    let bytes = read_bytes(&resolve(&at, &p.url))?;
    verify(&bytes, &p.signature)?;
    let out = std::env::temp_dir().join(format!("Dusty_{}_setup.exe", m.version));
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    Ok((m.version, out))
}

/// Runs the installer in update mode and lets it relaunch Dusty.
/// `quiet` = no window at all; otherwise a small progress window.
pub fn run_installer(path: &Path, quiet: bool) -> Result<(), String> {
    let mode = if quiet { "/S" } else { "/P" };
    std::process::Command::new(path).args([mode, "/UPDATE", "/R"]).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare() {
        assert!(newer("0.2.10", "0.2.9"));
        assert!(newer("1.0.0", "0.9.9"));
        assert!(!newer("0.2.1", "0.2.1"));
        assert!(!newer("0.2.0", "0.2.1"));
    }

    #[test]
    fn rejects_bad_signature() {
        // A well-formed but wrong signature must fail.
        assert!(verify(b"hello", "bm90IGEgc2lnbmF0dXJl").is_err());
    }

    #[test]
    fn resolves_relative_urls() {
        assert_eq!(resolve("https://x.dev/u/latest.json", "a.exe"), "https://x.dev/u/a.exe");
        assert!(resolve("C:\\r\\latest.json", "a.exe").ends_with("r\\a.exe"));
    }

    /// Checks the last `scripts/release.ps1` output end to end (manifest + signature):
    /// `cargo test real_release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_release() {
        let info = check(DEFAULT_SOURCE, "0.0.1").unwrap();
        println!("{info:?}");
        let (version, path) = download(DEFAULT_SOURCE).unwrap();
        println!("verified {version} at {}", path.display());
    }
}
