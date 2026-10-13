import { invoke } from "@tauri-apps/api/core";

export type Kind =
  | "stale_download"
  | "installer"
  | "extracted_archive"
  | "duplicate"
  | "dev_junk"
  | "backup"
  | "big_dormant"
  | "desktop_clutter"
  | "system_junk"
  | "recycle_bin"
  | "empty_folder";

export type Action = "recycle" | "delete" | "archive" | "sweep" | "empty" | "keep" | "snooze";
export type Mode = "off" | "ask" | "auto";

export interface Finding {
  id: string;
  kind: Kind;
  title: string;
  path: string;
  related: string[];
  bytes: number;
  items: number;
  lastTouched: number;
  isDir: boolean;
  reason: string;
  origin: string | null;
  confidence: number;
  suggested: Action;
  actions: Action[];
  ext: string;
  olderThanDays: number | null;
}

export interface ScanSummary {
  at: number;
  durationMs: number;
  count: number;
  bytes: number;
  newCount: number;
  autoCount: number;
  autoFreed: number;
  background: boolean;
}

export interface Stats {
  freedTotal: number;
  freedWeek: number;
  tidiedTotal: number;
  actionsTotal: number;
  keptTotal: number;
  streakDays: number;
}

export interface Snapshot {
  findings: Finding[];
  scanning: boolean;
  phase: string;
  lastScan: ScanSummary | null;
  lastScanAt: number;
  nextScanAt: number;
  pausedUntil: number;
  stats: Stats;
  idleSecs: number;
}

export interface Settings {
  inboxDirs: string[];
  desktopDir: string;
  devRoots: string[];
  libraryDirs: string[];
  archiveRoot: string;
  rules: Partial<Record<Kind, Mode>>;
  staleDays: number;
  installerDays: number;
  backupDays: number;
  devIdleDays: number;
  bigMb: number;
  bigDays: number;
  dupMinKb: number;
  desktopMaxItems: number;
  junkMinMb: number;
  recycleBinGb: number;
  snoozeDays: number;
  notify: boolean;
  quietStart: number;
  quietEnd: number;
  nudgeGapHours: number;
  scanFloorMin: number;
  scanCapMin: number;
  idleMin: number;
  startWithWindows: boolean;
  onboarded: boolean;
}

export interface Outcome {
  done: number;
  freed: number;
  moved: number;
  errors: string[];
  batch: string;
}

export interface HistoryRow {
  id: number;
  at: number;
  action: string;
  kind: string;
  findingId: string;
  title: string;
  path: string;
  dest: string | null;
  bytes: number;
  batch: string;
  undoable: boolean;
  undone: boolean;
  auto: boolean;
}

export interface Drive {
  name: string;
  mount: string;
  total: number;
  free: number;
  kind: "ssd" | "hdd" | "unknown";
  fs: string;
}

export interface SpaceNode {
  name: string;
  path: string;
  bytes: number;
  files: number;
  isDir: boolean;
  ext: string;
  newest: number;
}

export interface SpaceResult {
  path: string;
  bytes: number;
  files: number;
  children: SpaceNode[];
  ms: number;
}

export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  ready: () => invoke<void>("ready"),
  scanNow: () => invoke<void>("scan_now"),
  act: (ids: string[], action: Action) => invoke<Outcome>("act", { ids, action }),
  undo: (historyId: number) => invoke<void>("undo", { historyId }),
  history: (limit?: number) => invoke<HistoryRow[]>("history", { limit }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  pause: (minutes: number) => invoke<void>("pause", { minutes }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  drives: () => invoke<Drive[]>("drives"),
  space: (path: string) => invoke<SpaceResult>("space", { path }),
  recyclePath: (path: string, bytes: number) => invoke<number>("recycle_path", { path, bytes }),
  homeDir: () => invoke<string>("home_dir"),
};
