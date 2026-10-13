/**
 * Browser preview mode (dev only): when the UI runs outside Tauri, fake the
 * backend so views can be designed in a normal browser.
 * Open http://127.0.0.1:1420/?view=triage (or /search for the search popup)
 */
import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Finding, HistoryRow, Plan, Settings, Snapshot, TreeNode } from "./api";

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
    app: null,
    ...p,
  };
}

let findings: Finding[] = [
  f({ kind: "folder_plan", title: "Downloads: 14 things to tidy", path: `${home}\\Downloads`, isDir: true, items: 14, bytes: 2.1e9, confidence: 0.8, suggested: "organize", actions: ["organize", "snooze"], related: ["run-server.bat", "invoice-2026-03.pdf", "holiday.jpg", "ffmpeg.zip"].map((n) => `${home}\\Downloads\\${n}`), reason: "You asked me to keep Downloads empty. I drew up a plan for where each item should live. Check it, tweak anything, and I'll do the moving." }),
  f({ kind: "installer", title: "UbisoftConnectInstaller.exe", path: `${home}\\Downloads\\UbisoftConnectInstaller.exe`, ext: "exe", bytes: 253e6, confidence: 0.92, origin: "https://static3.cdn.ubi.com/orbit/launcher_installer/UbisoftConnectInstaller.exe", reason: "Ubisoft Connect is already installed, so the installer can go.", lastTouched: now - 75 * DAY }),
  f({ kind: "system_junk", title: "pip download cache", app: "pip", path: `${home}\\AppData\\Local\\pip\\cache`, isDir: true, bytes: 28.8e9, items: 5955, confidence: 0.9, suggested: "delete", actions: ["delete", "keep", "snooze"], olderThanDays: 30, reason: "Packages pip re-downloads on demand. Usually safe to delete. I'll only remove files older than 4 weeks and skip anything in use.", lastTouched: now - 3 * DAY }),
  f({ kind: "stale_project", title: "AllTalkTTS", path: "C:\\dev\\AllTalkTTS", isDir: true, bytes: 9.1e9, items: 52210, confidence: 0.7, suggested: "hibernate", actions: ["hibernate", "archive", "recycle", "keep", "snooze"], related: ["C:\\dev\\AllTalkTTS\\alltalk_venv", "C:\\dev\\AllTalkTTS\\installer_files\\conda"], reason: "Nobody touched this project in 10 months. Might it be finished or abandoned? I can hibernate it: zip the code (git included) to cold storage without the rebuildable alltalk_venv and conda, and leave a note here so you can wake it up in one click.", lastTouched: now - 300 * DAY }),
  f({ kind: "unsure", title: "old stuff 2021", path: "D:\\old stuff 2021", isDir: true, bytes: 34e9, items: 812, confidence: 0.3, suggested: "archive", actions: ["archive", "recycle", "keep", "snooze"], reason: "I don't recognise this folder. It holds 34 GB and nothing inside changed in 3 years. What is it? Do you still need it?", lastTouched: now - 1100 * DAY }),
  f({ kind: "duplicate", title: "HDR 2026-10-02 10-39-54.tif", path: `${home}\\Pictures\\Fije Results\\HDR\\HDR 2026-10-02 10-39-54.tif`, ext: "tif", bytes: 201e6, confidence: 0.78, actions: ["recycle", "keep", "snooze"], related: [`${home}\\Pictures\\Fije Results\\HDR\\HDR 2026-10-02 10-37-28.tif`], reason: "Exact copy of “HDR 2026-10-02 10-37-28.tif” in Pictures\\Fije Results\\HDR." }),
  f({ kind: "new_startup", title: "Spotify", app: "Spotify", path: "startup:user-run|Spotify", confidence: 0.5, suggested: "disable", actions: ["disable", "keep"], related: ['"C:\\Users\\you\\AppData\\Roaming\\Spotify\\Spotify.exe" --autostart --minimized'], reason: "“Spotify” now starts automatically with Windows. Every startup app makes boot a little slower. Keep it, or let it start only when you open it?" }),
  f({ kind: "resource_hog", title: "node.exe", app: "node.exe", path: "process:node.exe", bytes: 1.4e9, items: 3, confidence: 0.7, suggested: "quit", actions: ["quit", "keep", "snooze"], related: ["1234", "5678", "9012"], reason: "3 copies of node.exe kept running after the app that started them closed. They're probably leftovers from a dev session, using 1.4 GB." }),
  f({ kind: "stale_download", title: "Non confermato 672995.crdownload", path: `${home}\\Downloads\\Non confermato 672995.crdownload`, ext: "crdownload", bytes: 1.2e9, confidence: 0.95, reason: "An unfinished download that never completed.", lastTouched: now - 20 * DAY }),
  f({ kind: "extracted_archive", title: "ffmpeg-7.1-full_build.zip", path: `${home}\\Downloads\\ffmpeg-7.1-full_build.zip`, ext: "zip", bytes: 180e6, confidence: 0.88, related: [`${home}\\Downloads\\ffmpeg-7.1-full_build`], origin: "https://www.gyan.dev/ffmpeg/builds/", reason: "Already extracted to the “ffmpeg-7.1-full_build” folder right next to it. The archive is a leftover." }),
  f({ kind: "similar_photo", title: "IMG_2041 (small).jpg", path: `${home}\\Pictures\\IMG_2041 (small).jpg`, ext: "jpg", bytes: 1.1e6, confidence: 0.55, actions: ["recycle", "keep", "snooze"], related: [`${home}\\Pictures\\IMG_2041.jpg`], reason: "Looks almost identical to “IMG_2041.jpg”, which is bigger (probably the better copy). A resized or re-saved version, or a burst shot?" }),
  f({ kind: "dev_junk", title: "TCLTransmitter", path: "C:\\dev\\TCLTransmitter", isDir: true, bytes: 378e6, items: 4210, confidence: 0.95, suggested: "delete", actions: ["delete", "keep", "snooze"], related: [".gradle", "build", "app\\build"].map((n) => `C:\\dev\\TCLTransmitter\\${n}`), reason: "No changes in 7 months. Its .gradle, build and 1 more can be rebuilt any time with an install or build. Your code stays untouched.", lastTouched: now - 210 * DAY }),
  f({ kind: "backup", title: "Fije-backup-2026-09-29b.bundle", path: "C:\\dev\\Fije-backup-2026-09-29b.bundle", ext: "bundle", bytes: 96e6, confidence: 0.75, related: ["C:\\dev\\Fije"], reason: "A backup made 2 weeks ago. The original “Fije” is still right next to it." }),
  f({ kind: "empty_folder", title: "New folder (3)", path: `${home}\\Downloads\\New folder (3)`, isDir: true, items: 0, confidence: 0.9, suggested: "delete", actions: ["delete", "keep", "snooze"], reason: "An empty folder, created 3 months ago and never filled." }),
  f({ kind: "big_dormant", title: "edited.zip", path: "D:\\Videos\\edited.zip", ext: "zip", bytes: 1.3e9, confidence: 0.5, suggested: "archive", actions: ["archive", "recycle", "keep", "snooze"], reason: "1.3 GB that hasn't been opened in 4 months. Archive it somewhere roomier, or let it go.", lastTouched: now - 130 * DAY }),
  ...["MSVCP140.dll", "vcruntime140.dll", "libcrypto-3-x64.dll", "Qt6Core.dll"].map((n, i) =>
    f({ kind: "stale_download", title: n, ext: "dll", path: `${home}\\Downloads\\old-tool\\${n}`, bytes: (i + 1) * 3.1e6, confidence: 0.6, reason: "Untouched for 8 months. Downloads you still need usually get opened or moved.", lastTouched: now - 240 * DAY }),
  ),
];

const settings: Settings = {
  inboxDirs: [`${home}\\Downloads`],
  desktopDir: `${home}\\Desktop`,
  devRoots: ["C:\\dev"],
  libraryDirs: [`${home}\\Documents`, `${home}\\Pictures`, `${home}\\Videos`],
  watchedDirs: ["D:\\Projects"],
  drives: [],
  archiveRoot: `${home}\\Documents\\Dusty Archive`,
  coldStorage: "D:\\Dusty Cold Storage",
  rules: { system_junk: "auto" },
  policies: [
    { path: `${home}\\Downloads`, mode: "zero", graceHours: 48, askEveryDays: 3 },
    { path: `${home}\\Desktop`, mode: "watch", graceHours: 48, askEveryDays: 3 },
  ],
  sortDest: {
    installer: `${home}\\Documents\\Installers`,
    archive: `${home}\\Documents\\Archives`,
    document: `${home}\\Documents\\Documents Inbox`,
    image: `${home}\\Pictures\\Inbox`,
    screenshot: `${home}\\Pictures\\Screenshots`,
    video: `${home}\\Videos\\Inbox`,
    script: `${home}\\Documents\\Scripts`,
    code: "C:\\dev",
    partial: "recycle",
    shortcut: "leave",
    other: `${home}\\Documents\\Unsorted`,
  },
  staleDays: 30,
  installerDays: 3,
  backupDays: 14,
  devIdleDays: 45,
  staleProjectDays: 180,
  bigMb: 500,
  bigDays: 120,
  dupMinKb: 256,
  desktopMaxItems: 15,
  junkMinMb: 64,
  recycleBinGb: 2,
  snoozeDays: 14,
  unsureMinGb: 1,
  unsureDays: 365,
  notify: true,
  quickActions: true,
  weeklyDigest: true,
  quietStart: 22,
  quietEnd: 9,
  nudgeGapHours: 6,
  scanFloorMin: 30,
  scanCapMin: 720,
  idleMin: 3,
  indexEnabled: true,
  searchHotkey: "Ctrl+Alt+Space",
  watchStartup: true,
  watchHogs: true,
  learn: true,
  startWithWindows: true,
  onboarded: new URLSearchParams(location.search).get("onboarding") === null,
  devMode: false,
  seenVersion: 2,
  mirrors: [{ a: "D:\Photos", b: "G:\Photos" }],
};

const history: HistoryRow[] = [
  { id: 4, at: now - 300, action: "archive", kind: "folder_plan", findingId: "", title: "run.bat", path: `${home}\\Downloads\\run.bat`, dest: `${home}\\Documents\\Scripts\\run.bat`, bytes: 1200, batch: "p1", undoable: true, undone: false, auto: false },
  { id: 5, at: now - 300, action: "archive", kind: "folder_plan", findingId: "", title: "notes.pdf", path: `${home}\\Downloads\\notes.pdf`, dest: `${home}\\Documents\\Documents Inbox\\notes.pdf`, bytes: 120000, batch: "p1", undoable: true, undone: false, auto: false },
  { id: 3, at: now - 600, action: "recycle", kind: "installer", findingId: "a", title: "node-v25.8.1-x64.msi", path: `${home}\\Downloads\\node-v25.8.1-x64.msi`, dest: null, bytes: 31.8e6, batch: "b3", undoable: true, undone: false, auto: false },
  { id: 2, at: now - 3600, action: "delete", kind: "system_junk", findingId: "b", title: "NVIDIA shader cache", path: `${home}\\AppData\\Local\\NVIDIA\\DXCache`, dest: null, bytes: 8.3e9, batch: "b2", undoable: false, undone: false, auto: true },
  { id: 1, at: now - DAY - 7200, action: "hibernate", kind: "stale_project", findingId: "c", title: "OldGame", path: "C:\\dev\\OldGame", dest: "D:\\Dusty Cold Storage\\OldGame-2026-10-12.zip", bytes: 2.4e9, batch: "b1", undoable: true, undone: false, auto: false },
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
    scanStartedAt: 0,
  };
}

function tree(path: string, depth = 3): TreeNode {
  const names = ["AppData", "Videos", "Downloads", "Pictures", "Documents", ".cargo", ".vscode", "Music", "Desktop", "OneDrive", ".npm", "Saved Games"];
  const mk = (p: string, d: number, scale: number): TreeNode[] =>
    names.slice(0, d === 1 ? 12 : d === 2 ? 7 : 4).map((n, i) => {
      const bytes = Math.round(scale / (i + 1.4) ** 1.3);
      const cp = `${p}\\${n}`;
      return { name: n, path: cp, bytes, files: 1000, isDir: true, newest: now - i * DAY, children: d < depth ? mk(cp, d + 1, bytes) : [] };
    });
  const children = mk(path.replace(/\\$/, ""), 1, 9e10);
  children.push({ name: "pagefile-ish.bin", path: `${path}\\big.bin`, bytes: 4e9, files: 1, isDir: false, newest: now - 90 * DAY, children: [] });
  children.sort((a, b) => b.bytes - a.bytes);
  return { name: path, path, bytes: children.reduce((a, c) => a + c.bytes, 0), files: 100000, isDir: true, newest: now, children };
}

const plan: Plan = {
  folder: `${home}\\Downloads`,
  mode: "zero",
  items: [
    { path: `${home}\\Downloads\\run-server.bat`, name: "run-server.bat", isDir: false, bytes: 820, modified: now - 40 * DAY, category: "script", action: "move", dest: `${home}\\Documents\\Scripts`, reason: "A .bat script, probably something you run. Scripts get a home of their own." },
    { path: `${home}\\Downloads\\invoice-2026-03.pdf`, name: "invoice-2026-03.pdf", isDir: false, bytes: 240000, modified: now - 150 * DAY, category: "document", action: "move", dest: `${home}\\Documents\\Documents Inbox`, reason: "Document file." },
    { path: `${home}\\Downloads\\holiday.jpg`, name: "holiday.jpg", isDir: false, bytes: 3.2e6, modified: now - 20 * DAY, category: "image", action: "move", dest: `${home}\\Pictures\\Inbox`, reason: "Image file." },
    { path: `${home}\\Downloads\\ffmpeg.zip`, name: "ffmpeg.zip", isDir: false, bytes: 180e6, modified: now - 60 * DAY, category: "archive", action: "recycle", dest: "", reason: "Already extracted right next to it." },
    { path: `${home}\\Downloads\\DiscordSetup.exe`, name: "DiscordSetup.exe", isDir: false, bytes: 98e6, modified: now - 10 * DAY, category: "installer", action: "recycle", dest: "", reason: "Discord is already installed, so this installer can go." },
    { path: `${home}\\Downloads\\new-video.mp4`, name: "new-video.mp4", isDir: false, bytes: 820e6, modified: now - 3600, category: "video", action: "leave", dest: `${home}\\Videos\\Inbox`, reason: "Arrived recently, so I'll give it 48 hours. Video file." },
  ],
};

export function installMocks() {
  mockWindows("main");
  mockConvertFileSrc("windows");
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
            { name: "", mount: "C:\\", total: 952e9, free: 124e9, kind: "ssd", fs: "NTFS" },
            { name: "Loto Bianco", mount: "D:\\", total: 466e9, free: 158e9, kind: "hdd", fs: "NTFS" },
            { name: "BASTION", mount: "G:\\", total: 1.9e12, free: 114e9, kind: "ssd", fs: "NTFS" },
          ];
        case "history":
          return history;
        case "hibernated":
          return [{ id: 1, project: "C:\\dev\\OldGame", archive: "D:\\Dusty Cold Storage\\OldGame-2026-10-12.zip", bytes: 2.4e9, files: 1820, at: now - DAY, restored: false }];
        case "space_tree":
          return tree(a.path as string);
        case "kb_lookup":
          return (a.paths as string[]).map((p) =>
            p.endsWith("AppData") ? null : p.endsWith(".cargo") ? { app: "Cargo", cat: "build", safety: "safe", days: 60, note: "Crate archives Cargo re-downloads on demand.", path: p } : null,
          );
        case "index_status":
          return { ready: true, building: false, entries: 3112132, progress: 0, builtAt: now - 3600, buildMs: 27980, memoryMb: 135, drives: ["C:\\", "D:\\", "G:\\"] };
        case "recycle_bin":
          return [6.4e9, 214];
        case "make_plan":
          return plan;
        case "protected_list":
          return [{ path: "c:\\users\\you\\documents\\taxes", until: 0, at: now - 5 * DAY }];
        case "startup_list":
          return [
            { id: "user-run|Spotify", name: "Spotify", command: '"C:\\Users\\you\\AppData\\Roaming\\Spotify\\Spotify.exe" --autostart', source: "user-run", enabled: true, machine: false, exe: "C:\\Users\\you\\AppData\\Roaming\\Spotify\\Spotify.exe" },
            { id: "user-run|Discord", name: "Discord", command: "C:\\Users\\you\\AppData\\Local\\Discord\\Update.exe --processStart Discord.exe", source: "user-run", enabled: true, machine: false, exe: null },
            { id: "machine-run|SecurityHealth", name: "SecurityHealth", command: "%windir%\\system32\\SecurityHealthSystray.exe", source: "machine-run", enabled: true, machine: true, exe: null },
            { id: "user-run|Steam", name: "Steam", command: '"C:\\Program Files (x86)\\Steam\\steam.exe" -silent', source: "user-run", enabled: false, machine: false, exe: null },
          ];
        case "processes":
          return {
            totalMemory: 32e9,
            usedMemory: 21e9,
            groups: [
              { name: "chrome.exe", pids: [1, 2], count: 34, memory: 4.1e9, cpu: 3.2, diskRate: 120000, exe: null, hasWindow: true, idleCpu: 1, avgMemory: 4e9, flags: [], leftoverPids: [], oldestSecs: 9000 },
              { name: "node.exe", pids: [3, 4, 5], count: 3, memory: 1.4e9, cpu: 0.4, diskRate: 0, exe: null, hasWindow: false, idleCpu: 0.3, avgMemory: 1.4e9, flags: ["leftover"], leftoverPids: [3, 4, 5], oldestSecs: 90000 },
              { name: "Code.exe", pids: [6], count: 12, memory: 2.2e9, cpu: 1.1, diskRate: 0, exe: null, hasWindow: true, idleCpu: 0.5, avgMemory: 2e9, flags: [], leftoverPids: [], oldestSecs: 9000 },
              { name: "OneDrive.exe", pids: [7], count: 1, memory: 180e6, cpu: 9.4, diskRate: 4e6, exe: null, hasWindow: false, idleCpu: 11.2, avgMemory: 1.7e8, flags: ["busy_idle"], leftoverPids: [], oldestSecs: 90000 },
            ],
          };
        case "system_files":
          return { hiberfil: 12.7e9, pagefile: 8e9, swapfile: 256e6, windowsOld: 0, systemDrive: "C:" };
        case "power_plans":
          return [
            { guid: "381b4222-f694-41f0-9685-ff5bb260df2e", name: "Balanced", active: true },
            { guid: "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c", name: "High performance", active: false },
          ];
        case "optimize_info":
          return { scheduled: true, lastRun: new Date(Date.now() - 3 * DAY * 1000).toISOString() };
        case "app_info":
          return { version: "0.2.0", dataDir: "C:\\Users\\you\\AppData\\Local\\com.griz.dusty", debug: true };
        case "search":
          return {
            ms: 7,
            indexed: true,
            query: { terms: ["invoice"], exts: ["pdf"], dirsOnly: false, filesOnly: true, minSize: null, maxSize: null, after: null, before: null, location: [], explain: ["“invoice”", "PDFs", "~5 months ago"] },
            hits: ["invoice-2026-03.pdf", "Invoice_ACME_May.pdf", "invoice-template.pdf", "invoices-2025.pdf"].map((n, i) => ({ path: `${home}\\Documents\\Taxes\\${n}`, name: n, isDir: false, size: (i + 1) * 120000, mtime: now - (140 + i * 10) * DAY, score: 30 - i })),
          };
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
