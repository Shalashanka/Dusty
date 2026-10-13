import {
  Hourglass,
  PackageOpen,
  FileArchive,
  Copy,
  CodeXml,
  ArchiveRestore,
  Weight,
  Monitor,
  Wind,
  Trash2,
  FolderX,
  Heart,
  AlarmClock,
  Archive,
  Broom,
  Flame,
  Images,
  BedDouble,
  CircleHelp,
  ListChecks,
  Rocket,
  Activity,
  Snowflake,
  WandSparkles,
  PowerOff,
  CircleStop,
} from "@lucide/svelte";
import type { Action, Kind } from "./api";

export interface KindMeta {
  label: string;
  short: string;
  blurb: string;
  color: string;
  icon: typeof Hourglass;
  /** Safe enough to offer "clean all" from the dashboard. */
  bulk: boolean;
  /** Shown with a question ("What's this?") rather than a verdict. */
  question?: boolean;
  threshold?: { key: string; label: string; min: number; max: number; step: number; unit: string };
}

export const KINDS: Record<Kind, KindMeta> = {
  stale_download: {
    label: "Stale downloads",
    short: "Stale",
    blurb: "Things in Downloads (and folders you watch) nobody has opened or moved in a while.",
    color: "#ffb547",
    icon: Hourglass,
    bulk: false,
    threshold: { key: "staleDays", label: "Untouched for", min: 7, max: 365, step: 1, unit: "days" },
  },
  installer: {
    label: "Old installers",
    short: "Installers",
    blurb: "Setup files for programs that are probably installed already.",
    color: "#9d8cff",
    icon: PackageOpen,
    bulk: false,
    threshold: { key: "installerDays", label: "Older than", min: 1, max: 90, step: 1, unit: "days" },
  },
  extracted_archive: {
    label: "Extracted archives",
    short: "Archives",
    blurb: "Zips sitting next to the folder they were already unpacked into.",
    color: "#5ab0ff",
    icon: FileArchive,
    bulk: true,
  },
  duplicate: {
    label: "Duplicates",
    short: "Duplicates",
    blurb: "Byte-for-byte identical copies across your folders and data drives.",
    color: "#ff7eb6",
    icon: Copy,
    bulk: false,
    threshold: { key: "dupMinKb", label: "Ignore files under", min: 16, max: 102400, step: 16, unit: "KB" },
  },
  similar_photo: {
    label: "Near-identical photos",
    short: "Photos",
    blurb: "Resized, re-saved or burst shots of the same picture.",
    color: "#f78fb3",
    icon: Images,
    bulk: false,
  },
  dev_junk: {
    label: "Idle project leftovers",
    short: "Build junk",
    blurb: "node_modules, target, venvs and build output of projects you haven't touched.",
    color: "#4fe3c1",
    icon: CodeXml,
    bulk: true,
    threshold: { key: "devIdleDays", label: "Project idle for", min: 7, max: 365, step: 1, unit: "days" },
  },
  stale_project: {
    label: "Sleepy projects",
    short: "Projects",
    blurb: "Projects untouched for months. Hibernate them: zipped to cold storage, one click to wake.",
    color: "#64d2ff",
    icon: BedDouble,
    bulk: false,
    threshold: { key: "staleProjectDays", label: "Asleep for", min: 30, max: 1095, step: 1, unit: "days" },
  },
  backup: {
    label: "Forgotten backups",
    short: "Backups",
    blurb: "“_backup”, “- Copy”, .bak files and friends that outlived their purpose.",
    color: "#c792ea",
    icon: ArchiveRestore,
    bulk: false,
    threshold: { key: "backupDays", label: "Older than", min: 1, max: 365, step: 1, unit: "days" },
  },
  big_dormant: {
    label: "Big & dormant",
    short: "Big files",
    blurb: "Very large files nobody opened in months.",
    color: "#ff9e64",
    icon: Weight,
    bulk: false,
    threshold: { key: "bigMb", label: "Bigger than", min: 100, max: 10240, step: 100, unit: "MB" },
  },
  unsure: {
    label: "What's this?",
    short: "Mysteries",
    blurb: "Big folders in odd places nobody touched in ages. Dusty doesn't know them, so it asks.",
    color: "#e0af68",
    icon: CircleHelp,
    bulk: false,
    question: true,
    threshold: { key: "unsureDays", label: "Untouched for", min: 90, max: 1825, step: 15, unit: "days" },
  },
  desktop_clutter: {
    label: "Desktop pile",
    short: "Desktop",
    blurb: "When the desktop gets crowded, sweep loose items into a dated folder.",
    color: "#7dcfff",
    icon: Monitor,
    bulk: false,
    threshold: { key: "desktopMaxItems", label: "More than", min: 5, max: 100, step: 1, unit: "items" },
  },
  folder_plan: {
    label: "Folder plans",
    short: "Plans",
    blurb: "Folders with a policy (like “Downloads stays empty”) get an organizing plan you can edit.",
    color: "#bb9af7",
    icon: ListChecks,
    bulk: false,
  },
  system_junk: {
    label: "Temp & caches",
    short: "Junk",
    blurb: "Temp files, crash dumps, logs and app caches, from a built-in knowledge base of 90+ locations.",
    color: "#9ece6a",
    icon: Wind,
    bulk: true,
    threshold: { key: "junkMinMb", label: "Report from", min: 16, max: 4096, step: 16, unit: "MB" },
  },
  recycle_bin: {
    label: "Recycle Bin",
    short: "Bin",
    blurb: "Remind me when the bin itself is hoarding space.",
    color: "#f7768e",
    icon: Trash2,
    bulk: false,
    threshold: { key: "recycleBinGb", label: "Fuller than", min: 1, max: 100, step: 1, unit: "GB" },
  },
  empty_folder: {
    label: "Empty folders",
    short: "Empty",
    blurb: "Folders in Downloads or on the desktop with nothing inside.",
    color: "#a9b1d6",
    icon: FolderX,
    bulk: true,
  },
  new_startup: {
    label: "New startup apps",
    short: "Startup",
    blurb: "Apps that quietly added themselves to Windows startup.",
    color: "#ff9e64",
    icon: Rocket,
    bulk: false,
  },
  resource_hog: {
    label: "Background hogs",
    short: "Hogs",
    blurb: "Leftover dev processes and apps that burn CPU or memory while you're away.",
    color: "#f7768e",
    icon: Activity,
    bulk: false,
  },
};

export const KIND_ORDER: Kind[] = [
  "folder_plan",
  "system_junk",
  "dev_junk",
  "stale_project",
  "installer",
  "stale_download",
  "duplicate",
  "similar_photo",
  "extracted_archive",
  "backup",
  "big_dormant",
  "unsure",
  "desktop_clutter",
  "empty_folder",
  "recycle_bin",
  "new_startup",
  "resource_hog",
];

export interface ActionMeta {
  label: string;
  verb: string;
  icon: typeof Hourglass;
  tone: "danger" | "ok" | "neutral" | "accent";
  hint: string;
}

export const ACTIONS: Record<Action, ActionMeta> = {
  recycle: { label: "Recycle", verb: "Recycled", icon: Trash2, tone: "danger", hint: "Moves it to the Recycle Bin. You can undo, and it frees space once the bin is emptied." },
  delete: { label: "Delete for good", verb: "Deleted", icon: Flame, tone: "danger", hint: "Permanent: skips the Recycle Bin and frees space right away. Can't be undone." },
  archive: { label: "Archive", verb: "Archived", icon: Archive, tone: "accent", hint: "Moves it into your archive folder (by month). Undoable." },
  sweep: { label: "Sweep it", verb: "Swept", icon: Broom, tone: "accent", hint: "Moves loose desktop items into a dated folder. Nothing is deleted." },
  empty: { label: "Empty bin", verb: "Emptied", icon: Trash2, tone: "danger", hint: "Permanently deletes everything in the Recycle Bin." },
  hibernate: { label: "Hibernate", verb: "Hibernated", icon: Snowflake, tone: "accent", hint: "Zips the project to cold storage, verifies it, leaves a note. Undo wakes it up." },
  organize: { label: "See the plan", verb: "Organized", icon: WandSparkles, tone: "accent", hint: "Opens the organizing plan so you can review and edit it." },
  disable: { label: "Don't autostart", verb: "Disabled", icon: PowerOff, tone: "danger", hint: "Stops it from starting with Windows (like Task Manager). Undoable." },
  quit: { label: "End it", verb: "Ended", icon: CircleStop, tone: "danger", hint: "Closes the process. Unsaved work in it would be lost." },
  keep: { label: "Keep", verb: "Kept", icon: Heart, tone: "ok", hint: "Keep it; I won't ask about it again." },
  snooze: { label: "Later", verb: "Snoozed", icon: AlarmClock, tone: "neutral", hint: "Ask me again later." },
};

/** Removal-ish actions, the "left swipe". */
export const DESTRUCTIVE: Action[] = ["recycle", "delete", "sweep", "empty", "archive", "hibernate", "organize", "disable", "quit"];

/** Coarse file category, for grouping lists. */
export function categoryOf(ext: string, isDir: boolean): string {
  if (isDir) return "Folders";
  const e = ext.toLowerCase();
  const groups: [string[], string][] = [
    [["exe", "msi", "msix", "appx", "msixbundle"], "Installers & programs"],
    [["dll", "sys", "ocx", "lib", "so", "pdb"], "Libraries (.dll…)"],
    [["zip", "7z", "rar", "tar", "gz", "tgz", "xz", "bz2", "zst", "bundle"], "Archives"],
    [["iso", "img", "vhd", "vhdx"], "Disk images"],
    [["mp4", "mkv", "mov", "avi", "webm", "wmv", "m4v"], "Videos"],
    [["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "heic", "raw", "psd", "svg"], "Images"],
    [["mp3", "wav", "flac", "ogg", "m4a", "aac"], "Audio"],
    [["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "csv", "ppt", "pptx"], "Documents"],
    [["bat", "cmd", "ps1", "sh", "ahk", "vbs", "reg"], "Scripts"],
    [["js", "ts", "py", "rs", "json", "html", "css", "java", "kt", "c", "cpp", "h", "cs", "go"], "Code"],
    [["crdownload", "part", "tmp", "partial"], "Unfinished downloads"],
  ];
  return groups.find(([list]) => list.includes(e))?.[1] ?? (e ? `.${e} files` : "Other");
}

/** Color + label for a file-type tile. */
export function extStyle(ext: string, isDir: boolean): { label: string; color: string } {
  if (isDir) return { label: "", color: "#8b9cff" };
  const e = ext.toLowerCase();
  const groups: [string[], string][] = [
    [["exe", "msi", "msix", "appx", "msixbundle", "bat", "cmd", "ps1"], "#9d8cff"],
    [["zip", "7z", "rar", "tar", "gz", "tgz", "xz", "bz2", "zst", "iso", "bundle"], "#5ab0ff"],
    [["mp4", "mkv", "mov", "avi", "webm", "wmv"], "#c792ea"],
    [["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "heic", "raw", "psd", "svg"], "#ff7eb6"],
    [["mp3", "wav", "flac", "ogg", "m4a", "aac"], "#4fe3c1"],
    [["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "csv", "ppt", "pptx"], "#ffb547"],
    [["js", "ts", "py", "rs", "json", "html", "css", "java", "kt", "c", "cpp", "h", "cs", "go", "dll"], "#7dcfff"],
    [["crdownload", "part", "tmp", "partial"], "#f7768e"],
  ];
  const hit = groups.find(([list]) => list.includes(e));
  return { label: e.slice(0, 4).toUpperCase() || "FILE", color: hit ? hit[1] : "#8f8aa6" };
}

/** What kind of preview a file can have in the webview. */
export function previewKind(ext: string): "image" | "video" | "audio" | "pdf" | "text" | null {
  const e = ext.toLowerCase();
  if (["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico"].includes(e)) return "image";
  if (["mp4", "webm", "mov", "m4v", "mkv"].includes(e)) return "video";
  if (["mp3", "wav", "ogg", "m4a", "flac", "aac", "opus"].includes(e)) return "audio";
  if (e === "pdf") return "pdf";
  if (["txt", "md", "log", "json", "csv", "xml", "yml", "yaml", "ini", "cfg", "bat", "cmd", "ps1", "sh", "py", "js", "ts", "rs", "html", "css", "toml"].includes(e)) return "text";
  return null;
}
