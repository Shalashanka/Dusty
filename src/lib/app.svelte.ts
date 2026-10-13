import { listen } from "@tauri-apps/api/event";
import {
  api,
  type Action,
  type Drive,
  type Finding,
  type IndexStatus,
  type Kind,
  type Settings,
  type Snapshot,
} from "./api";
import { ACTIONS } from "./kinds";
import { bytes } from "./format";

export type View = "home" | "triage" | "space" | "speed" | "history" | "rules";

export interface Snack {
  id: number;
  text: string;
  undo?: () => Promise<void>;
  tone?: "ok" | "error";
}

/** "C:" from a path, or "" for non-file findings. */
export function driveOf(path: string): string {
  return /^[a-z]:/i.test(path) ? path.slice(0, 2).toUpperCase() : "";
}

class AppStore {
  view = $state<View>("home");
  filter = $state<Kind | "all">("all");
  /** "all" or a drive like "C:" — scopes the dashboard and the triage list. */
  drive = $state<string>("all");
  snap = $state<Snapshot | null>(null);
  settings = $state<Settings | null>(null);
  drives = $state<Drive[]>([]);
  index = $state<IndexStatus | null>(null);
  bin = $state<[number, number]>([0, 0]);
  home = $state("");
  phase = $state("");
  snack = $state<Snack | null>(null);
  paletteOpen = $state(false);
  /** Folder whose organizing plan is open, if any. */
  planFolder = $state<string | null>(null);
  /** Bytes freed since the window was opened, for the triage counter. */
  sessionFreed = $state(0);

  allFindings = $derived(this.snap?.findings ?? []);
  findings = $derived(
    this.drive === "all" ? this.allFindings : this.allFindings.filter((f) => driveOf(f.path) === this.drive),
  );
  scanning = $derived(this.snap?.scanning ?? false);
  pendingBytes = $derived(this.findings.reduce((a, f) => a + f.bytes, 0));
  paused = $derived((this.snap?.pausedUntil ?? 0) > Date.now() / 1000);

  byKind = $derived.by(() => {
    const m = new Map<Kind, Finding[]>();
    for (const f of this.findings) {
      const list = m.get(f.kind) ?? [];
      list.push(f);
      m.set(f.kind, list);
    }
    return m;
  });

  #snackTimer: ReturnType<typeof setTimeout> | undefined;
  #driveTimer: ReturnType<typeof setTimeout> | undefined;

  async init() {
    const [snap, settings, drives, home, index, bin] = await Promise.all([
      api.snapshot(),
      api.getSettings(),
      api.drives(),
      api.homeDir(),
      api.indexStatus(),
      api.recycleBin(),
    ]);
    this.snap = snap;
    this.settings = settings;
    this.drives = drives;
    this.home = home;
    this.index = index;
    this.bin = bin;
    this.phase = snap.phase;

    await listen("changed", () => {
      this.refresh();
      this.refreshDisks();
    });
    await listen<string>("phase", (e) => (this.phase = e.payload));
    await listen<string>("navigate", (e) => this.go(e.payload as View));
    await listen<IndexStatus>("index", (e) => (this.index = e.payload));
    // Keep relative times and free space fresh.
    setInterval(() => {
      this.refresh();
      this.refreshDisks();
    }, 30_000);
  }

  async refresh() {
    this.snap = await api.snapshot();
  }

  /** Free space changes after every action; debounce the disk query. */
  refreshDisks() {
    clearTimeout(this.#driveTimer);
    this.#driveTimer = setTimeout(async () => {
      [this.drives, this.bin] = await Promise.all([api.drives(), api.recycleBin()]);
    }, 400);
  }

  go(view: View, filter: Kind | "all" = "all") {
    this.filter = filter;
    this.view = view;
  }

  toast(text: string, opts: Partial<Snack> = {}) {
    clearTimeout(this.#snackTimer);
    this.snack = { id: Date.now(), text, ...opts };
    this.#snackTimer = setTimeout(() => (this.snack = null), opts.undo ? 7000 : 3500);
  }

  async saveSettings(s: Settings) {
    this.settings = s;
    await api.saveSettings($state.snapshot(s) as Settings);
  }

  /** Applies an action and offers undo. */
  async act(items: Finding[], action: Action, quiet = false) {
    if (!items.length) return;
    if (action === "organize") {
      this.planFolder = items[0].path;
      return;
    }
    // Optimistic: drop them from the list right away.
    const ids = new Set(items.map((f) => f.id));
    if (this.snap) this.snap.findings = this.snap.findings.filter((f) => !ids.has(f.id));
    try {
      const out = await api.act([...ids], action);
      this.sessionFreed += out.freed;
      if (out.errors.length) {
        this.toast(out.errors[0], { tone: "error" });
      } else if (!quiet) {
        const what = items.length === 1 ? `“${items[0].title}”` : `${items.length} things`;
        const extra = out.freed ? ` · ${bytes(out.freed)} freed` : "";
        const undoable = !["delete", "empty", "quit"].includes(action);
        this.toast(`${ACTIONS[action].verb} ${what}${extra}`, {
          tone: "ok",
          undo: undoable ? () => this.undoBatch(out.batch) : undefined,
        });
      }
      await this.refresh();
      this.refreshDisks();
      return out;
    } catch (e) {
      this.toast(String(e), { tone: "error" });
      await this.refresh();
    }
  }

  /** Remove each item with its own best removal action (recycle, delete for good, sweep...). */
  async removeSmart(items: Finding[]) {
    const groups = new Map<Action, Finding[]>();
    for (const f of items) {
      const a = (["recycle", "delete", "sweep", "empty", "hibernate", "disable", "quit", "archive"] as Action[]).find(
        (x) => f.suggested === x,
      ) ?? (f.actions.includes("recycle") ? "recycle" : f.actions[0]);
      groups.set(a, [...(groups.get(a) ?? []), f]);
    }
    let freed = 0;
    let done = 0;
    for (const [a, list] of groups) {
      const out = await this.act(list, a, true);
      freed += out?.freed ?? 0;
      done += out?.done ?? 0;
    }
    this.toast(`Handled ${done} things${freed ? ` · ${bytes(freed)} freed` : ""}`, { tone: "ok" });
  }

  async protect(path: string, days: number, label: string) {
    await api.protect(path, days);
    await this.refresh();
    const when = days === 0 ? "for good" : days === 1 ? "for a day" : days === 7 ? "for a week" : `for ${days} days`;
    this.toast(`Got it. I won't touch ${label} ${when}.`, {
      tone: "ok",
      undo: async () => {
        await api.unprotect(path);
        await api.scanNow();
        this.toast("Protection removed.", { tone: "ok" });
      },
    });
  }

  async undoBatch(batch: string) {
    const rows = (await api.history(500)).filter((r) => r.batch === batch && !r.undone);
    let freed = 0;
    for (const r of rows) {
      try {
        await api.undo(r.id);
        if (["recycle"].includes(r.action)) freed += r.bytes;
      } catch (e) {
        this.toast(String(e), { tone: "error" });
        return;
      }
    }
    this.sessionFreed = Math.max(0, this.sessionFreed - freed);
    this.toast("Undone. Everything is back where it was.", { tone: "ok" });
    await this.refresh();
    this.refreshDisks();
  }
}

export const app = new AppStore();
