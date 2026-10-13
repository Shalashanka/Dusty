/**
 * Browser preview mode (dev only): when the UI runs outside Tauri, fake the
 * backend so views can be designed in a normal browser.
 * Open http://127.0.0.1:1420/?view=triage
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Finding, HistoryRow, Settings, Snapshot, SpaceResult } from "./api";

const now = Math.floor(Date.now() / 1000);
const DAY = 86400;
const home = "C:\\Users\\you";

function f(p: Partial<Finding> & Pick<Finding, "kind" | "title" | "path">): Finding {
  return {
    id: Math.random().toString(36).slice(2),
    related: [],
    bytes: 0,
    items: 1,
    lastTouched: now - 40 * DAY,
    isDir: false,
    reason: "",
    origin: null,
    confidence: 0.8,
    suggested: "recycle",
    actions: ["recycle", "archive", "keep", "snooze"],
    ext: "",
    olderThanDays: null,
    ...p,
  };
}

let findings: Finding[] = [
  f({ kind: "installer", title: "UbisoftConnectInstaller.exe", path: `${home}\\Downloads\\UbisoftConnectInstaller.exe`, ext: "exe", bytes: 253e6, confidence: 0.92, origin: "https://static3.cdn.ubi.com/orbit/launcher_installer/UbisoftConnectInstaller.exe", reason: "Ubisoft Connect is already installed, so the installer can go.", lastTouched: now - 75 * DAY }),
  f({ kind: "system_junk", title: "pip cache", path: `${home}\\AppData\\Local\\pip\\cache`, isDir: true, bytes: 28.8e9, items: 5955, confidence: 0.9, suggested: "delete", actions: ["delete", "keep", "snooze"], olderThanDays: 30, reason: "Packages pip re-downloads on demand. I'll only remove files older than 4 weeks and skip anything in use.", lastTouched: now - 3 * DAY }),
  f({ kind: "dev_junk", title: "AllTalkTTS", path: "C:\\dev\\AllTalkTTS", isDir: true, bytes: 8.3e9, items: 41210, confidence: 0.95, suggested: "delete", actions: ["delete", "keep", "snooze"], related: ["C:\\dev\\AllTalkTTS\\alltalk_venv", "C:\\dev\\AllTalkTTS\\installer_files\\conda", "C:\\dev\\AllTalkTTS\\__pycache__"], reason: "No changes in 10 months. Its alltalk_venv, conda and 1 more can be rebuilt any time with an install or build. Your code stays untouched.", lastTouched: now - 300 * DAY }),
  f({ kind: "stale_download", title: "Non confermato 672995.crdownload", path: `${home}\\Downloads\\Non confermato 672995.crdownload`, ext: "crdownload", bytes: 1.2e9, confidence: 0.95, reason: "An unfinished download that never completed.", lastTouched: now - 20 * DAY }),
  f({ kind: "duplicate", title: "HDR 2026-10-02 10-39-54.tif", path: `${home}\\Pictures\\Fije Results\\HDR\\HDR 2026-10-02 10-39-54.tif`, ext: "tif", bytes: 201e6, confidence: 0.78, actions: ["recycle", "keep", "snooze"], related: [`${home}\\Pictures\\Fije Results\\HDR\\HDR 2026-10-02 10-37-28.tif`], reason: "Exact copy of “HDR 2026-10-02 10-37-28.tif” in Pictures\\Fije Results\\HDR." }),
  f({ kind: "extracted_archive", title: "ffmpeg-7.1-full_build.zip", path: `${home}\\Downloads\\ffmpeg-7.1-full_build.zip`, ext: "zip", bytes: 180e6, confidence: 0.88, related: [`${home}\\Downloads\\ffmpeg-7.1-full_build`], origin: "https://www.gyan.dev/ffmpeg/builds/", reason: "Already extracted to the “ffmpeg-7.1-full_build” folder right next to it. The archive is a leftover." }),
  f({ kind: "backup", title: "Fije-backup-2026-09-29b.bundle", path: "C:\\dev\\Fije-backup-2026-09-29b.bundle", ext: "bundle", bytes: 96e6, confidence: 0.75, related: ["C:\\dev\\Fije"], reason: "A backup made 2 weeks ago. The original “Fije” is still right next to it." }),
  f({ kind: "big_dormant", title: "edited.zip", path: `${home}\\Videos\\edited.zip`, ext: "zip", bytes: 1.3e9, confidence: 0.5, suggested: "archive", actions: ["archive", "recycle", "keep", "snooze"], reason: "1.3 GB that hasn't been opened in 4 months. Archive it somewhere roomier, or let it go.", lastTouched: now - 130 * DAY }),
  f({ kind: "desktop_clutter", title: "23 loose things on your desktop", path: `${home}\\Desktop`, isDir: true, items: 23, bytes: 340e6, confidence: 0.7, suggested: "sweep", actions: ["sweep", "snooze"], related: ["notes.txt", "Screenshot 2026-09-12.png", "invoice.pdf", "test.py", "mockup-v3.fig", "Untitled.png"].map((n) => `${home}\\Desktop\\${n}`), reason: "A clear desktop is a clear head. I'll move everything except shortcuts into “Swept by Dustling\\2026-10-13”. Nothing gets deleted." }),
  f({ kind: "empty_folder", title: "New folder (3)", path: `${home}\\Downloads\\New folder (3)`, isDir: true, items: 0, confidence: 0.9, suggested: "delete", actions: ["delete", "keep", "snooze"], reason: "An empty folder, created 3 months ago and never filled." }),
];

const settings: Settings = {
  inboxDirs: [`${home}\\Downloads`],
  desktopDir: `${home}\\Desktop`,
  devRoots: ["C:\\dev"],
  libraryDirs: [`${home}\\Documents`, `${home}\\Pictures`, `${home}\\Videos`],
  archiveRoot: `${home}\\Documents\\Dustling Archive`,
  rules: { system_junk: "auto" },
  staleDays: 30,
  installerDays: 3,
  backupDays: 14,
  devIdleDays: 45,
  bigMb: 500,
  bigDays: 120,
  dupMinKb: 256,
  desktopMaxItems: 15,
  junkMinMb: 64,
  recycleBinGb: 2,
  snoozeDays: 14,
  notify: true,
  quietStart: 22,
  quietEnd: 9,
  nudgeGapHours: 6,
  scanFloorMin: 30,
  scanCapMin: 720,
  idleMin: 3,
  startWithWindows: true,
  onboarded: true,
};

const history: HistoryRow[] = [
  { id: 3, at: now - 600, action: "recycle", kind: "installer", findingId: "a", title: "node-v25.8.1-x64.msi", path: `${home}\\Downloads\\node-v25.8.1-x64.msi`, dest: null, bytes: 31.8e6, batch: "b3", undoable: true, undone: false, auto: false },
  { id: 2, at: now - 3600, action: "delete", kind: "system_junk", findingId: "b", title: "NVIDIA shader cache", path: `${home}\\AppData\\Local\\NVIDIA\\DXCache`, dest: null, bytes: 8.3e9, batch: "b2", undoable: false, undone: false, auto: true },
  { id: 1, at: now - DAY - 7200, action: "keep", kind: "backup", findingId: "c", title: "OpusStudio_backup", path: "C:\\dev\\OpusStudio_backup", dest: null, bytes: 0, batch: "b1", undoable: true, undone: false, auto: false },
];

function snapshot(): Snapshot {
  return {
    findings,
    scanning: false,
    phase: "",
    lastScan: null,
    lastScanAt: now - 120,
    nextScanAt: now + 46 * 60,
    pausedUntil: 0,
    stats: { freedTotal: 12.4e9, freedWeek: 8.6e9, tidiedTotal: 23, actionsTotal: 57, keptTotal: 9, streakDays: 3 },
    idleSecs: 0,
  };
}

function space(path: string): SpaceResult {
  const names = ["AppData", "Videos", "Downloads", "Pictures", "Documents", ".cargo", ".vscode", "Music", "Desktop", "OneDrive", ".npm", "Saved Games", "go", "Contacts"];
  const children = names.map((n, i) => ({ name: n, path: `${path}\\${n}`, bytes: Math.round(9e10 / (i + 1.4) ** 1.3), files: 1000 * (14 - i), isDir: true, ext: "", newest: now - i * DAY }));
  children.push({ name: "pagefile-ish.bin", path: `${path}\\big.bin`, bytes: 4e9, files: 1, isDir: false, ext: "bin", newest: now - 90 * DAY });
  children.sort((a, b) => b.bytes - a.bytes);
  return { path, bytes: children.reduce((a, c) => a + c.bytes, 0), files: children.reduce((a, c) => a + c.files, 0), children, ms: 1840 };
}

export function installMocks() {
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = args as Record<string, unknown>;
      switch (cmd) {
        case "snapshot":
          return snapshot();
        case "get_settings":
          return settings;
        case "home_dir":
          return home;
        case "drives":
          return [
            { name: "", mount: "C:\\", total: 952e9, free: 64e9, kind: "ssd", fs: "NTFS" },
            { name: "Loto Bianco", mount: "D:\\", total: 466e9, free: 158e9, kind: "hdd", fs: "NTFS" },
          ];
        case "history":
          return history;
        case "space":
          return space(a.path as string);
        case "act": {
          const ids = a.ids as string[];
          const freed = findings.filter((x) => ids.includes(x.id)).reduce((s, x) => s + x.bytes, 0);
          findings = findings.filter((x) => !ids.includes(x.id));
          return { done: ids.length, freed, moved: 0, errors: [], batch: "mock" };
        }
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
