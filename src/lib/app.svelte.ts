import { listen } from "@tauri-apps/api/event";
import { api, type Action, type Drive, type Finding, type Kind, type Settings, type Snapshot } from "./api";
import { ACTIONS } from "./kinds";
import { bytes } from "./format";

export type View = "home" | "triage" | "space" | "history" | "rules";

export interface Snack {
  id: number;
  text: string;
  undo?: () => Promise<void>;
  tone?: "ok" | "error";
}

class AppStore {
  view = $state<View>("home");
  filter = $state<Kind | "all">("all");
  snap = $state<Snapshot | null>(null);
  settings = $state<Settings | null>(null);
  drives = $state<Drive[]>([]);
  home = $state("");
  phase = $state("");
  snack = $state<Snack | null>(null);
  paletteOpen = $state(false);
  /** Bytes freed since the window was opened, for the triage counter. */
  sessionFreed = $state(0);

  findings = $derived(this.snap?.findings ?? []);
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

  async init() {
    const [snap, settings, drives, home] = await Promise.all([
      api.snapshot(),
      api.getSettings(),
      api.drives(),
      api.homeDir(),
    ]);
    this.snap = snap;
    this.settings = settings;
    this.drives = drives;
    this.home = home;
    this.phase = snap.phase;

    await listen("changed", () => this.refresh());
    await listen<string>("phase", (e) => (this.phase = e.payload));
    await listen<string>("navigate", (e) => this.go(e.payload as View));
    await listen("scan-finished", async () => (this.drives = await api.drives()));
    // Keep relative times fresh.
    setInterval(() => this.refresh(), 60_000);
  }

  async refresh() {
    this.snap = await api.snapshot();
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

  /** Applies an action and offers undo. Returns history-ready outcome. */
  async act(items: Finding[], action: Action, quiet = false) {
    if (!items.length) return;
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
        const undoable = !["delete", "empty"].includes(action);
        this.toast(`${ACTIONS[action].verb} ${what}${extra}`, {
          tone: "ok",
          undo: undoable ? () => this.undoBatch(out.batch) : undefined,
        });
      }
      await this.refresh();
      return out;
    } catch (e) {
      this.toast(String(e), { tone: "error" });
      await this.refresh();
    }
  }

  async undoBatch(batch: string) {
    const rows = (await api.history(200)).filter((r) => r.batch === batch && !r.undone);
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
  }
}

export const app = new AppStore();
