<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Trash2, Flame, Archive, Broom, Heart, AlarmClock, Undo2, Bot, Check, Snowflake, PowerOff, CircleStop, Sun, FolderOpen } from "@lucide/svelte";
  import Counter from "$lib/components/Counter.svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type Hibernated, type HistoryRow, type Kind } from "$lib/api";
  import { KINDS } from "$lib/kinds";
  import { ago, bytes, clock, dayLabel, shortPath } from "$lib/format";

  let rows = $state<HistoryRow[]>([]);
  let busy = $state<number | null>(null);

  const ICONS: Record<string, typeof Trash2> = {
    recycle: Trash2,
    delete: Flame,
    empty: Trash2,
    archive: Archive,
    sweep: Broom,
    keep: Heart,
    snooze: AlarmClock,
    hibernate: Snowflake,
    disable: PowerOff,
    quit: CircleStop,
  };
  const VERBS: Record<string, string> = {
    recycle: "Recycled",
    delete: "Deleted for good",
    empty: "Emptied",
    archive: "Archived",
    sweep: "Swept",
    keep: "Kept",
    snooze: "Snoozed",
    hibernate: "Hibernated",
    disable: "Stopped autostart of",
    quit: "Ended",
  };
  let sleeping = $state<Hibernated[]>([]);

  async function load() {
    [rows, sleeping] = await Promise.all([api.history(500), api.hibernated()]);
  }
  async function wake(h: Hibernated) {
    try {
      await api.wakeProject(h.project, h.archive);
      app.toast(`${h.project.split("\\").pop()} is awake again`, { tone: "ok" });
      await load();
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    }
  }
  $effect(() => {
    // Reload when anything changes elsewhere.
    app.snap?.stats.actionsTotal;
    load();
  });

  /** Bulk actions produce one row per item: fold each batch into one entry. */
  const entries = $derived.by(() => {
    const out: (HistoryRow & { count: number })[] = [];
    for (const r of rows) {
      const prev = out.at(-1);
      if (prev && prev.action === r.action && prev.batch === r.batch && r.batch) {
        prev.count++;
        continue;
      }
      out.push({ ...r, count: 1 });
    }
    return out;
  });
  const days = $derived(Map.groupBy(entries, (r) => dayLabel(r.at)));

  async function undo(r: HistoryRow & { count: number }) {
    busy = r.id;
    try {
      if (r.count > 1) await app.undoBatch(r.batch);
      else {
        await api.undo(r.id);
        app.toast(`Brought back “${r.title}”`, { tone: "ok" });
      }
      await load();
      await app.refresh();
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    } finally {
      busy = null;
    }
  }
</script>

<div class="history scroll">
  <h1>History</h1>
  <p class="muted lead">Everything I've done, and everything you decided. Most of it can be undone.</p>

  <div class="stats">
    <div class="card s"><span class="faint">Freed all-time</span><b><Counter value={app.snap?.stats.freedTotal ?? 0} kind="bytes" /></b></div>
    <div class="card s"><span class="faint">This week</span><b><Counter value={app.snap?.stats.freedWeek ?? 0} kind="bytes" /></b></div>
    <div class="card s"><span class="faint">Decisions</span><b><Counter value={app.snap?.stats.actionsTotal ?? 0} /></b></div>
    <div class="card s"><span class="faint">Things kept</span><b><Counter value={app.snap?.stats.keptTotal ?? 0} /></b></div>
  </div>

  {#if sleeping.some((h) => !h.restored)}
    <h3>Hibernating projects</h3>
    <div class="timeline">
      {#each sleeping.filter((h) => !h.restored) as h (h.id)}
        <div class="row hib">
          <span class="ico archive"><Snowflake size={15} /></span>
          <div class="what">
            <span class="ellipsis"><b>{h.project.split("\\").pop()}</b> · {h.files.toLocaleString()} files, asleep since {ago(h.at)}</span>
            <span class="faint ellipsis mono small">{shortPath(h.archive, app.home)}</span>
          </div>
          <span></span>
          <span class="size tabular">{bytes(h.bytes)}</span>
          <span></span>
          <span class="act">
            <button class="btn sm ghost" onclick={() => api.reveal(h.archive)} title="Show the zip"><FolderOpen size={13} /></button>
            <button class="btn sm" onclick={() => wake(h)}><Sun size={13} /> Wake up</button>
          </span>
        </div>
      {/each}
    </div>
  {/if}

  {#if !entries.length}
    <div class="empty">
      <Mascot mood="idle" size={130} />
      <p class="muted">Nothing here yet. Your first tidy will show up here.</p>
    </div>
  {/if}

  {#each [...days] as [day, list], di (day)}
    <section in:fly={{ y: 12, duration: 400, delay: di * 60, easing: cubicOut }}>
      <h3>{day}</h3>
      <div class="timeline">
        {#each list as r (r.id)}
          {@const Icon = ICONS[r.action] ?? Check}
          {@const meta = KINDS[r.kind as Kind]}
          <div class="row" class:undone={r.undone}>
            <span class="ico {r.action}"><Icon size={15} /></span>
            <div class="what">
              <span class="ellipsis">
                <b>{VERBS[r.action] ?? r.action}</b>
                {r.count > 1 ? `${r.count} items` : r.title}
                {#if r.auto}<span class="auto" title="Done on autopilot"><Bot size={12} /> autopilot</span>{/if}
              </span>
              <span class="faint ellipsis mono small">{shortPath(r.dest ?? r.path, app.home)}</span>
            </div>
            <span class="kind faint">{meta?.short ?? ""}</span>
            <span class="size tabular">{r.bytes && ["recycle", "delete", "empty"].includes(r.action) ? bytes(r.bytes) : ""}</span>
            <span class="time faint tabular">{clock(r.at)}</span>
            <span class="act">
              {#if r.undone}
                <span class="faint small">undone</span>
              {:else if r.undoable}
                <button class="btn sm ghost" disabled={busy === r.id} onclick={() => undo(r)}><Undo2 size={13} /> Undo</button>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .history {
    height: 100%;
    padding: 34px 40px 60px;
  }
  h1 {
    font-size: 28px;
  }
  .lead {
    margin: 6px 0 20px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 26px;
  }
  .s {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
  }
  .s b {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 600;
  }
  h3 {
    font-size: 13px;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 18px 0 8px;
  }
  .timeline {
    display: flex;
    flex-direction: column;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 90px 80px 54px 90px;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    transition: background 0.15s, opacity 0.3s;
  }
  .row:last-child {
    border-bottom: 0;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.undone {
    opacity: 0.45;
  }
  .ico {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text-2);
  }
  .ico.recycle,
  .ico.delete,
  .ico.empty {
    color: var(--danger);
    background: var(--danger-soft);
  }
  .ico.keep {
    color: var(--ok);
    background: var(--ok-soft);
  }
  .ico.archive,
  .ico.sweep {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .what b {
    font-weight: 600;
  }
  .auto {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    margin-left: 6px;
    padding: 1px 7px;
    border-radius: 99px;
    font-size: 11px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .small {
    font-size: 11.5px;
  }
  .kind,
  .time {
    font-size: 12.5px;
  }
  .size {
    text-align: right;
    font-weight: 600;
  }
  .act {
    display: flex;
    justify-content: flex-end;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px;
  }
  .row.hib {
    grid-template-columns: auto minmax(0, 1fr) 0 90px 0 auto;
  }
  .act {
    gap: 6px;
  }
</style>
