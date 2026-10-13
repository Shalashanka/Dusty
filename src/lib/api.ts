import { invoke } from "@tauri-apps/api/core";

export type Kind =
  | "stale_download"
  | "installer"
  | "extracted_archive"
  | "duplicate"
  | "similar_photo"
  | "dev_junk"
  | "stale_project"
  | "backup"
  | "big_dormant"
  | "unsure"
  | "desktop_clutter"
  | "folder_plan"
  | "system_junk"
  | "recycle_bin"
  | "empty_folder"
  | "new_startup"
  | "resource_hog";

export type Action =
  | "recycle"
  | "delete"
  | "archive"
  | "sweep"
  | "empty"
  | "hibernate"
  | "organize"
  | "disable"
  | "quit"
  | "keep"
  | "snooze";
export type Mode = "off" | "ask" | "auto";
export type PolicyMode = "watch" | "zero" | "sorted";

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
  app: string | null;
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
  scanStartedAt: number;
}

export interface FolderPolicy {
  path: string;
  mode: PolicyMode;
  graceHours: number;
  askEveryDays: number;
}

export interface Settings {
  inboxDirs: string[];
  desktopDir: string;
  devRoots: string[];
  libraryDirs: string[];
  watchedDirs: string[];
  drives: string[];
  archiveRoot: string;
  coldStorage: string;
  rules: Partial<Record<Kind, Mode>>;
  policies: FolderPolicy[];
  sortDest: Record<string, string>;
  staleDays: number;
  installerDays: number;
  backupDays: number;
  devIdleDays: number;
  staleProjectDays: number;
  bigMb: number;
  bigDays: number;
  dupMinKb: number;
  desktopMaxItems: number;
  junkMinMb: number;
  recycleBinGb: number;
  snoozeDays: number;
  unsureMinGb: number;
  unsureDays: number;
  notify: boolean;
  quickActions: boolean;
  weeklyDigest: boolean;
  quietStart: number;
  quietEnd: number;
  nudgeGapHours: number;
  scanFloorMin: number;
  scanCapMin: number;
  idleMin: number;
  indexEnabled: boolean;
  searchHotkey: string;
  watchStartup: boolean;
  watchHogs: boolean;
  learn: boolean;
  startWithWindows: boolean;
  onboarded: boolean;
  seenVersion: number;
  devMode: boolean;
  mirrors: { a: string; b: string }[];
  updateSource: string;
  autoInstall: boolean;
  voiceEnabled: boolean;
  voiceModel: string;
  voiceReply: boolean;
  voiceHotkey: string;
  paletteHotkey: string;
  wander: boolean;
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

export interface TreeNode {
  name: string;
  path: string;
  bytes: number;
  files: number;
  isDir: boolean;
  newest: number;
  children: TreeNode[];
}

export interface IndexStatus {
  ready: boolean;
  building: boolean;
  entries: number;
  progress: number;
  builtAt: number;
  buildMs: number;
  memoryMb: number;
  drives: string[];
}

export interface Query {
  terms: string[];
  exts: string[];
  dirsOnly: boolean;
  filesOnly: boolean;
  minSize: number | null;
  maxSize: number | null;
  after: number | null;
  before: number | null;
  location: string[];
  explain: string[];
}

export interface Hit {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  mtime: number;
  score: number;
}

export interface SearchResult {
  query: Query;
  hits: Hit[];
  ms: number;
  indexed: boolean;
}

export interface KbHit {
  app: string;
  cat: string;
  safety: "safe" | "caution" | "never";
  days: number;
  note: string;
  path: string;
}

export interface Protected {
  path: string;
  until: number;
  at: number;
}

export interface PlanItem {
  path: string;
  name: string;
  isDir: boolean;
  bytes: number;
  modified: number;
  category: string;
  action: "move" | "recycle" | "leave";
  dest: string;
  reason: string;
}

export interface Plan {
  folder: string;
  mode: PolicyMode;
  items: PlanItem[];
}

export interface Hibernated {
  id: number;
  project: string;
  archive: string;
  bytes: number;
  files: number;
  at: number;
  restored: boolean;
}

export interface StartupItem {
  id: string;
  name: string;
  command: string;
  source: string;
  enabled: boolean;
  machine: boolean;
  exe: string | null;
}

export interface ProcGroup {
  name: string;
  pids: number[];
  count: number;
  memory: number;
  cpu: number;
  diskRate: number;
  exe: string | null;
  hasWindow: boolean;
  idleCpu: number;
  avgMemory: number;
  flags: string[];
  leftoverPids: number[];
  oldestSecs: number;
}

export interface ProcSnapshot {
  groups: ProcGroup[];
  totalMemory: number;
  usedMemory: number;
}

export interface SystemFiles {
  hiberfil: number;
  pagefile: number;
  swapfile: number;
  windowsOld: number;
  systemDrive: string;
}

export interface PowerPlan {
  guid: string;
  name: string;
  active: boolean;
}

export interface OptimizeInfo {
  scheduled: boolean;
  lastRun: string;
}

export interface AppxApp {
  name: string;
  fullName: string;
  friendly: string;
  note: string;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  debug: boolean;
}

export interface UpdateInfo {
  current: string;
  available: string | null;
  notes: string;
  date: string;
  source: string;
}

export interface VoiceModel {
  id: string;
  name: string;
  sizeMb: number;
  languages: string;
  installed: boolean;
}

export interface VoiceStatus {
  model: string;
  installed: boolean;
  sizeMb: number;
  downloading: boolean;
  listening: boolean;
  hasMic: boolean;
  models: VoiceModel[];
}

export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  ready: () => invoke<void>("ready"),
  scanNow: () => invoke<void>("scan_now"),
  addMirror: (a: string, b: string) => invoke<void>("add_mirror", { a, b }),
  checkUpdate: () => invoke<UpdateInfo>("check_update"),
  installUpdate: (quiet: boolean) => invoke<void>("install_update", { quiet }),
  voiceStatus: () => invoke<VoiceStatus>("voice_status"),
  voiceDownload: (model: string) => invoke<void>("voice_download", { model }),
  voiceListen: () => invoke<void>("voice_listen"),
  voiceStop: () => invoke<void>("voice_stop"),
  openBar: (mode: "search" | "commands" | "voice") => invoke<void>("open_bar", { mode }),
  barCommand: (cmd: string) => invoke<string>("bar_command", { cmd }),
  buddyDone: () => invoke<void>("buddy_done"),
  buddyTest: (message?: string) => invoke<void>("buddy_test", { message }),
  hideTraymenu: () => invoke<void>("hide_traymenu"),
  quitApp: () => invoke<void>("quit_app"),
  act: (ids: string[], action: Action) => invoke<Outcome>("act", { ids, action }),
  undo: (historyId: number) => invoke<void>("undo", { historyId }),
  history: (limit?: number) => invoke<HistoryRow[]>("history", { limit }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  pause: (minutes: number) => invoke<void>("pause", { minutes }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openFile: (path: string) => invoke<void>("open_file", { path }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  drives: () => invoke<Drive[]>("drives"),
  recyclePath: (path: string, bytes: number) => invoke<number>("recycle_path", { path, bytes }),
  deletePath: (path: string, bytes: number) => invoke<void>("delete_path", { path, bytes }),
  homeDir: () => invoke<string>("home_dir"),
  indexStatus: () => invoke<IndexStatus>("index_status"),
  indexRebuild: () => invoke<void>("index_rebuild"),
  search: (query: string, limit?: number) => invoke<SearchResult>("search", { query, limit }),
  spaceTree: (path: string, depth?: number) => invoke<TreeNode>("space_tree", { path, depth }),
  kbLookup: (paths: string[]) => invoke<(KbHit | null)[]>("kb_lookup", { paths }),
  thumbnail: (path: string, size?: number) => invoke<string | null>("thumbnail", { path, size }),
  readText: (path: string, max?: number) => invoke<string>("read_text", { path, max }),
  recycleBin: () => invoke<[number, number]>("recycle_bin"),
  emptyBin: () => invoke<number>("empty_bin"),
  protect: (path: string, days: number) => invoke<void>("protect", { path, days }),
  unprotect: (path: string) => invoke<void>("unprotect", { path }),
  protectedList: () => invoke<Protected[]>("protected_list"),
  makePlan: (folder: string) => invoke<Plan>("make_plan", { folder }),
  applyPlan: (folder: string, items: PlanItem[], original: PlanItem[]) =>
    invoke<Outcome>("apply_plan", { folder, items, original }),
  hibernated: () => invoke<Hibernated[]>("hibernated"),
  wakeProject: (project: string, archive: string) => invoke<void>("wake_project", { project, archive }),
  startupList: () => invoke<StartupItem[]>("startup_list"),
  startupSet: (id: string, enabled: boolean) => invoke<void>("startup_set", { id, enabled }),
  processes: () => invoke<ProcSnapshot>("processes"),
  setLiveProcs: (live: boolean) => invoke<void>("set_live_procs", { live }),
  killProcesses: (pids: number[]) => invoke<number>("kill_processes", { pids }),
  systemFiles: () => invoke<SystemFiles>("system_files"),
  powerPlans: () => invoke<PowerPlan[]>("power_plans"),
  setPowerPlan: (guid: string) => invoke<void>("set_power_plan", { guid }),
  optimizeInfo: () => invoke<OptimizeInfo>("optimize_info"),
  optimizeDrive: (letter: string) => invoke<void>("optimize_drive", { letter }),
  analyzeDrive: (letter: string) => invoke<number>("analyze_drive", { letter }),
  tune: (action: string, arg?: string) => invoke<void>("tune", { action, arg }),
  preinstalledApps: () => invoke<AppxApp[]>("preinstalled_apps"),
  hideSearch: () => invoke<void>("hide_search"),
  openInDusty: (view: string) => invoke<void>("open_in_dusty", { view }),
  devAction: (action: string) => invoke<string>("dev_action", { action }),
  appInfo: () => invoke<AppInfo>("app_info"),
};
