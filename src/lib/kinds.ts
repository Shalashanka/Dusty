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
  Brush,
  Flame,
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
  threshold?: { key: string; label: string; min: number; max: number; step: number; unit: string };
}

export const KINDS: Record<Kind, KindMeta> = {
  stale_download: {
    label: "Stale downloads",
    short: "Stale",
    blurb: "Things in Downloads nobody has opened or moved in a while.",
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
    blurb: "Byte-for-byte identical copies across your personal folders.",
    color: "#ff7eb6",
    icon: Copy,
    bulk: false,
    threshold: { key: "dupMinKb", label: "Ignore files under", min: 16, max: 102400, step: 16, unit: "KB" },
  },
  dev_junk: {
    label: "Idle project leftovers",
    short: "Projects",
    blurb: "node_modules, target, venvs and build output of projects you haven't touched.",
    color: "#4fe3c1",
    icon: CodeXml,
    bulk: true,
    threshold: { key: "devIdleDays", label: "Project idle for", min: 7, max: 365, step: 1, unit: "days" },
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
  desktop_clutter: {
    label: "Desktop pile",
    short: "Desktop",
    blurb: "When the desktop gets crowded, sweep loose items into a dated folder.",
    color: "#7dcfff",
    icon: Monitor,
    bulk: false,
    threshold: { key: "desktopMaxItems", label: "More than", min: 5, max: 100, step: 1, unit: "items" },
  },
  system_junk: {
    label: "Temp & caches",
    short: "Junk",
    blurb: "Temp files, crash dumps, shader and package caches. All rebuilt on demand.",
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
};

export const KIND_ORDER: Kind[] = [
  "system_junk",
  "dev_junk",
  "installer",
  "stale_download",
  "duplicate",
  "extracted_archive",
  "backup",
  "big_dormant",
  "desktop_clutter",
  "empty_folder",
  "recycle_bin",
];

export interface ActionMeta {
  label: string;
  verb: string;
  icon: typeof Hourglass;
  keys: string[];
  tone: "danger" | "ok" | "neutral" | "accent";
}

export const ACTIONS: Record<Action, ActionMeta> = {
  recycle: { label: "Recycle", verb: "Recycled", icon: Trash2, keys: ["D", "←"], tone: "danger" },
  delete: { label: "Clear", verb: "Cleared", icon: Flame, keys: ["D", "←"], tone: "danger" },
  archive: { label: "Archive", verb: "Archived", icon: Archive, keys: ["A"], tone: "accent" },
  sweep: { label: "Sweep it", verb: "Swept", icon: Brush, keys: ["D", "←"], tone: "accent" },
  empty: { label: "Empty bin", verb: "Emptied", icon: Trash2, keys: ["D", "←"], tone: "danger" },
  keep: { label: "Keep", verb: "Kept", icon: Heart, keys: ["K", "→"], tone: "ok" },
  snooze: { label: "Later", verb: "Snoozed", icon: AlarmClock, keys: ["S", "↓"], tone: "neutral" },
};

/** Color + label for a file-type tile. */
export function extStyle(ext: string, isDir: boolean): { label: string; color: string } {
  if (isDir) return { label: "", color: "#8b9cff" };
  const e = ext.toLowerCase();
  const groups: [string[], string][] = [
    [["exe", "msi", "msix", "appx", "msixbundle", "bat", "cmd"], "#9d8cff"],
    [["zip", "7z", "rar", "tar", "gz", "tgz", "xz", "bz2", "zst", "iso", "bundle"], "#5ab0ff"],
    [["mp4", "mkv", "mov", "avi", "webm", "wmv"], "#c792ea"],
    [["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "heic", "raw", "psd", "svg"], "#ff7eb6"],
    [["mp3", "wav", "flac", "ogg", "m4a", "aac"], "#4fe3c1"],
    [["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "csv", "ppt", "pptx"], "#ffb547"],
    [["js", "ts", "py", "rs", "json", "html", "css", "java", "kt", "c", "cpp", "h", "cs", "go"], "#7dcfff"],
    [["crdownload", "part", "tmp", "partial"], "#f7768e"],
  ];
  const hit = groups.find(([list]) => list.includes(e));
  return { label: e.slice(0, 4).toUpperCase() || "FILE", color: hit ? hit[1] : "#8f8aa6" };
}
