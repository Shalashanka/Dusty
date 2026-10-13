//! System tune-ups: the "fresh install" feeling without reinstalling.

use crate::winsys::{powershell, quiet, run_elevated};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemFiles {
    pub hiberfil: u64,
    pub pagefile: u64,
    pub swapfile: u64,
    pub windows_old: u64,
    pub system_drive: String,
}

pub fn system_files() -> SystemFiles {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root = format!("{drive}\\");
    let mut sf = SystemFiles {
        hiberfil: 0,
        pagefile: 0,
        swapfile: 0,
        windows_old: 0,
        system_drive: drive.clone(),
    };
    // Directory listings report sizes even for locked system files.
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            let len = e.metadata().map(|m| m.len()).unwrap_or(0);
            match n.as_str() {
                "hiberfil.sys" => sf.hiberfil = len,
                "pagefile.sys" => sf.pagefile = len,
                "swapfile.sys" => sf.swapfile = len,
                "windows.old" => sf.windows_old = crate::walk::dir_stats(&e.path()).bytes.max(1),
                _ => {}
            }
        }
    }
    sf
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub active: bool,
}

/// `powercfg /list` (works in any display language: GUIDs and parentheses).
pub fn power_plans() -> Vec<PowerPlan> {
    let out = quiet("powercfg.exe", &["/list"]).unwrap_or_default();
    out.lines()
        .filter_map(|l| {
            let guid = l
                .split_whitespace()
                .find(|w| w.len() == 36 && w.chars().filter(|c| *c == '-').count() == 4)?;
            let name = l.split_once('(')?.1.split_once(')')?.0.to_string();
            Some(PowerPlan {
                guid: guid.to_string(),
                name,
                active: l.trim_end().ends_with('*'),
            })
        })
        .collect()
}

pub fn set_power_plan(guid: &str) -> Result<(), String> {
    quiet("powercfg.exe", &["/setactive", guid]).map(|_| ())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeInfo {
    pub scheduled: bool,
    pub last_run: String,
}

/// Is Windows' own weekly "Optimize drives" task on?
pub fn optimize_schedule() -> OptimizeInfo {
    let out = powershell(
        "$t = Get-ScheduledTask -TaskPath '\\Microsoft\\Windows\\Defrag\\' -TaskName ScheduledDefrag -ErrorAction SilentlyContinue; \
         if ($t) { \"$($t.State)|$((Get-ScheduledTaskInfo $t).LastRunTime.ToString('o'))\" }",
    )
    .unwrap_or_default();
    let (state, last) = out.trim().split_once('|').unwrap_or(("", ""));
    OptimizeInfo {
        scheduled: !state.is_empty() && state != "Disabled",
        last_run: last.to_string(),
    }
}

/// Defragments HDDs / re-trims SSDs (Windows picks the right one). Needs admin.
pub fn optimize_drive(letter: &str) -> Result<(), String> {
    let l = letter.trim_end_matches(['\\', ':']);
    let code = run_elevated("defrag.exe", &format!("{l}: /O /H"))?;
    if code == 0 {
        Ok(())
    } else {
        Err(format!("defrag finished with code {code}"))
    }
}

/// Fragmentation percentage of a volume (admin, language independent via WMI).
pub fn analyze_drive(letter: &str) -> Result<u32, String> {
    let l = letter.trim_end_matches(['\\', ':']);
    let out = std::env::temp_dir().join(format!("dusty-frag-{l}.txt"));
    let _ = std::fs::remove_file(&out);
    let script = format!(
        "$v = Get-CimInstance Win32_Volume -Filter \\\"DriveLetter='{l}:'\\\"; \
         $r = Invoke-CimMethod -InputObject $v -MethodName DefragAnalysis; \
         $r.DefragAnalysis.FilePercentFragmentation | Out-File -Encoding ascii '{}'",
        out.display()
    );
    run_elevated(
        "powershell.exe",
        &format!("-NoProfile -ExecutionPolicy Bypass -Command \"{script}\""),
    )?;
    let s = std::fs::read_to_string(&out)
        .map_err(|_| "the analysis didn't produce a result".to_string())?;
    s.trim()
        .parse::<u32>()
        .map_err(|_| format!("unexpected result: {}", s.trim()))
}

pub fn set_hibernation(on: bool) -> Result<(), String> {
    let code = run_elevated(
        "powercfg.exe",
        if on {
            "/hibernate on"
        } else {
            "/hibernate off"
        },
    )?;
    if code == 0 {
        Ok(())
    } else {
        Err(format!("powercfg failed ({code})"))
    }
}

pub fn clear_delivery_optimization() -> Result<(), String> {
    run_elevated(
        "powershell.exe",
        "-NoProfile -Command \"Delete-DeliveryOptimizationCache -Force\"",
    )
    .map(|_| ())
}

/// Removes superseded Windows components (can take several minutes).
pub fn component_cleanup() -> Result<(), String> {
    let code = run_elevated("dism.exe", "/Online /Cleanup-Image /StartComponentCleanup")?;
    if code == 0 {
        Ok(())
    } else {
        Err(format!("DISM finished with code {code}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppxApp {
    pub name: String,
    pub full_name: String,
    pub friendly: String,
    pub note: String,
}

/// Preinstalled apps most people never open.
const BLOAT: &[(&str, &str, &str)] = &[
    ("king.com.CandyCrush", "Candy Crush", "Preinstalled game."),
    ("Microsoft.BingNews", "Microsoft News", "News feed app."),
    ("Microsoft.BingWeather", "Weather", "Weather app."),
    (
        "Microsoft.BingSearch",
        "Bing Search",
        "Web search integration.",
    ),
    ("Microsoft.GetHelp", "Get Help", "Support app."),
    ("Microsoft.Getstarted", "Tips", "Windows tips."),
    (
        "Microsoft.MicrosoftSolitaireCollection",
        "Solitaire Collection",
        "Ad-supported card games.",
    ),
    (
        "Microsoft.MicrosoftOfficeHub",
        "Microsoft 365 (Office) hub",
        "Launcher for Office web apps.",
    ),
    ("Microsoft.People", "People", "Contacts app."),
    (
        "Microsoft.PowerAutomateDesktop",
        "Power Automate",
        "Automation tool most people never use.",
    ),
    ("Microsoft.Todos", "Microsoft To Do", "Task app."),
    (
        "Microsoft.WindowsFeedbackHub",
        "Feedback Hub",
        "Send feedback to Microsoft.",
    ),
    ("Microsoft.WindowsMaps", "Maps", "Offline maps app."),
    (
        "Microsoft.ZuneMusic",
        "Media Player (legacy Groove)",
        "Music player.",
    ),
    ("Microsoft.ZuneVideo", "Movies & TV", "Video store app."),
    ("Clipchamp.Clipchamp", "Clipchamp", "Video editor."),
    ("Microsoft.549981C3F5F10", "Cortana", "Retired assistant."),
    (
        "MicrosoftCorporationII.QuickAssist",
        "Quick Assist",
        "Remote help tool.",
    ),
    ("Microsoft.YourPhone", "Phone Link", "Phone companion."),
    (
        "Microsoft.MixedReality.Portal",
        "Mixed Reality Portal",
        "VR headset setup.",
    ),
    ("Microsoft.SkypeApp", "Skype", "Discontinued messenger."),
    (
        "SpotifyAB.SpotifyMusic",
        "Spotify (Store)",
        "Remove only if you don't use it.",
    ),
    (
        "Disney.37853FC22B2CE",
        "Disney+",
        "Preinstalled streaming app.",
    ),
    ("BytedancePte.Ltd.TikTok", "TikTok", "Preinstalled app."),
    ("Facebook.Facebook", "Facebook", "Preinstalled app."),
    ("Facebook.Instagram", "Instagram", "Preinstalled app."),
    ("AmazonVideo.PrimeVideo", "Prime Video", "Preinstalled app."),
    (
        "Microsoft.XboxApp",
        "Xbox Console Companion",
        "Legacy Xbox app.",
    ),
    (
        "Microsoft.OutlookForWindows",
        "Outlook (new)",
        "Remove only if you use another mail app.",
    ),
    ("MicrosoftTeams", "Teams (personal)", "Consumer Teams chat."),
];

pub fn preinstalled_apps() -> Vec<AppxApp> {
    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "PackageFullName")]
        full: String,
    }
    let out = powershell(
        "Get-AppxPackage | Select-Object Name, PackageFullName | ConvertTo-Json -Compress",
    )
    .unwrap_or_default();
    let raw: Vec<Raw> = serde_json::from_str(out.trim()).unwrap_or_default();
    raw.into_iter()
        .filter_map(|r| {
            let (_, friendly, note) = BLOAT.iter().find(|(id, _, _)| r.name.starts_with(id))?;
            Some(AppxApp {
                name: r.name,
                full_name: r.full,
                friendly: friendly.to_string(),
                note: note.to_string(),
            })
        })
        .collect()
}

pub fn remove_app(full_name: &str) -> Result<(), String> {
    if full_name.contains(['\'', '"', ';']) {
        return Err("bad package name".into());
    }
    powershell(&format!("Remove-AppxPackage -Package '{full_name}'")).map(|_| ())
}
