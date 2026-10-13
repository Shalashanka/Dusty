<script lang="ts">
  import { fade, scale, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { X, WandSparkles, FolderOpen, ArrowRight, Trash2, Pause, LoaderCircle } from "@lucide/svelte";
  import Preview from "./Preview.svelte";
  import Mascot from "./Mascot.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type Plan, type PlanItem } from "$lib/api";
  import { bytes, shortPath, age } from "$lib/format";

  let plan = $state<Plan | null>(null);
  let original: PlanItem[] = [];
  let loading = $state(true);
  let applying = $state(false);
  let error = $state("");

  $effect(() => {
    const folder = app.planFolder;
    if (!folder) return;
    loading = true;
    plan = null;
    api
      .makePlan(folder)
      .then((p) => {
        plan = p;
        original = structuredClone($state.snapshot(p.items) as PlanItem[]);
      })
      .catch((e) => (error = String(e)))
      .finally(() => (loading = false));
  });

  const groups = $derived.by(() => {
    if (!plan) return [];
    const m = new Map<string, PlanItem[]>();
    for (const it of plan.items) m.set(it.category, [...(m.get(it.category) ?? []), it]);
    return [...m.entries()];
  });
  const counts = $derived({
    move: plan?.items.filter((i) => i.action === "move").length ?? 0,
    recycle: plan?.items.filter((i) => i.action === "recycle").length ?? 0,
    leave: plan?.items.filter((i) => i.action === "leave").length ?? 0,
  });

  const LABELS: Record<string, string> = {
    installer: "Installers",
    archive: "Archives",
    document: "Documents",
    spreadsheet: "Spreadsheets",
    presentation: "Presentations",
    ebook: "E-books",
    image: "Images",
    screenshot: "Screenshots",
    video: "Videos",
    audio: "Audio",
    script: "Scripts",
    code: "Code & projects",
    font: "Fonts",
    disk_image: "Disk images",
    design: "Design files",
    data: "Data",
    torrent: "Torrents",
    partial: "Unfinished downloads",
    shortcut: "Shortcuts",
    folder: "Folders",
    other: "Other",
  };

  async function pick(items: PlanItem[]) {
    const p = await api.pickFolder();
    if (!p) return;
    for (const it of items) {
      it.dest = p;
      it.action = "move";
    }
  }
  function setAction(items: PlanItem[], a: PlanItem["action"]) {
    for (const it of items) {
      if (a === "move" && !it.dest) {
        const o = original.find((x) => x.path === it.path);
        it.dest = o?.dest || "";
        if (!it.dest) continue;
      }
      it.action = a;
    }
  }

  async function apply() {
    if (!plan) return;
    applying = true;
    try {
      const out = await api.applyPlan(plan.folder, $state.snapshot(plan.items) as PlanItem[], original);
      app.planFolder = null;
      await app.refresh();
      app.refreshDisks();
      if (out.errors.length) app.toast(`Done with ${out.errors.length} hiccup(s): ${out.errors[0]}`, { tone: "error" });
      else
        app.toast(`Tidied! ${out.moved} moved${out.freed ? `, ${bytes(out.freed)} recycled` : ""}.`, {
          tone: "ok",
          undo: () => app.undoBatch(out.batch),
        });
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    } finally {
      applying = false;
    }
  }
  const name = (p: string) => p.slice(p.lastIndexOf("\\") + 1);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && app.planFolder && (app.planFolder = null)} />

{#if app.planFolder}
  <div class="veil" transition:fade={{ duration: 180 }} role="presentation" onclick={() => (app.planFolder = null)}></div>
  <div class="modal" transition:scale={{ start: 0.96, duration: 240, easing: cubicOut }} role="dialog" aria-label="Organizing plan">
    <header>
      <span class="ic"><WandSparkles size={18} /></span>
      <div>
        <h2>Tidy plan for {name(app.planFolder)}</h2>
        <span class="faint">{plan?.mode === "sorted" ? "Filing loose items into category folders inside it." : "Finding a proper home for everything in it."} Change anything you like.</span>
      </div>
      <button class="close" onclick={() => (app.planFolder = null)} aria-label="Close"><X size={18} /></button>
    </header>

    <div class="body scroll">
      {#if loading}
        <div class="center"><Mascot mood="sweeping" size={110} /><p class="muted">Drawing up a plan…</p></div>
      {:else if error}
        <p class="muted center">{error}</p>
      {:else if plan && !plan.items.length}
        <div class="center"><Mascot mood="happy" size={110} /><p class="muted">Already spotless. Nothing to plan!</p></div>
      {:else if plan}
        {#each groups as [cat, items], gi (cat)}
          <section in:fly={{ y: 10, duration: 300, delay: gi * 40, easing: cubicOut }}>
            <div class="ghead">
              <b>{LABELS[cat] ?? cat}</b>
              <span class="faint">{items.length}</span>
              <span class="sp"></span>
              <span class="faint small">All:</span>
              <button class="mini" onclick={() => pick(items)}><FolderOpen size={12} /> Move to…</button>
              <button class="mini" onclick={() => setAction(items, "recycle")}><Trash2 size={12} /> Recycle</button>
              <button class="mini" onclick={() => setAction(items, "leave")}><Pause size={12} /> Leave</button>
            </div>
            {#each items as it (it.path)}
              <div class="item" class:leave={it.action === "leave"} class:recycle={it.action === "recycle"}>
                <Preview path={it.path} ext={it.name.includes(".") ? it.name.split(".").pop() : ""} isDir={it.isDir} size={32} />
                <div class="iname">
                  <b class="ellipsis" title={it.path}>{it.name}</b>
                  <span class="faint ellipsis">{bytes(it.bytes)} · {age(it.modified)} old · {it.reason}</span>
                </div>
                <select value={it.action} onchange={(e) => setAction([it], e.currentTarget.value as PlanItem["action"])}>
                  <option value="move">Move</option>
                  <option value="recycle">Recycle</option>
                  <option value="leave">Leave</option>
                </select>
                {#if it.action === "move"}
                  <button class="dest" onclick={() => pick([it])} title={it.dest}>
                    <ArrowRight size={13} /> <span class="ellipsis mono">{shortPath(it.dest, app.home)}</span>
                  </button>
                {:else}
                  <span class="dest faint">{it.action === "recycle" ? "→ Recycle Bin" : "stays here"}</span>
                {/if}
              </div>
            {/each}
          </section>
        {/each}
      {/if}
    </div>

    {#if plan && plan.items.length}
      <footer>
        <span class="muted">{counts.move} to move · {counts.recycle} to recycle · {counts.leave} stay</span>
        <span class="sp"></span>
        <button class="btn ghost" onclick={() => (app.planFolder = null)}>Not now</button>
        <button class="btn primary" disabled={applying || counts.move + counts.recycle === 0} onclick={apply}>
          {#if applying}<LoaderCircle size={15} class="spin" />{:else}<WandSparkles size={15} />{/if} Do it
        </button>
      </footer>
    {/if}
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(3px);
    z-index: 300;
  }
  .modal {
    position: fixed;
    inset: 6% 8%;
    z-index: 301;
    display: flex;
    flex-direction: column;
    border-radius: var(--r-xl);
    background: color-mix(in srgb, var(--bg-solid) 96%, white 4%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 100px -20px rgba(0, 0, 0, 0.8);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 18px 22px;
    border-bottom: 1px solid var(--border);
  }
  header > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }
  h2 {
    font-size: 19px;
  }
  .ic {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: var(--accent);
    background: var(--accent-soft);
  }
  .close {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .close:hover {
    background: var(--surface-2);
  }
  .body {
    flex: 1;
    padding: 8px 22px 16px;
  }
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 60px 0;
    gap: 8px;
  }
  section {
    margin-top: 14px;
  }
  .ghead {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    position: sticky;
    top: 0;
    background: color-mix(in srgb, var(--bg-solid) 96%, white 4%);
    z-index: 1;
  }
  .sp {
    flex: 1;
  }
  .small {
    font-size: 12px;
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 9px;
    border-radius: 99px;
    font-size: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-2);
  }
  .mini:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 110px minmax(0, 0.8fr);
    gap: 12px;
    align-items: center;
    padding: 7px 8px;
    border-radius: var(--r);
    transition:
      background 0.15s,
      opacity 0.2s;
  }
  .item:hover {
    background: var(--surface);
  }
  .item.leave {
    opacity: 0.55;
  }
  .item.recycle .dest {
    color: var(--danger);
  }
  .iname {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-size: 12px;
  }
  .iname b {
    font-size: 13.5px;
    font-weight: 600;
  }
  select {
    height: 30px;
    padding: 0 8px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 12.5px;
  }
  select option {
    background: var(--bg-solid);
  }
  .dest {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 12px;
    text-align: left;
    color: var(--text-2);
  }
  button.dest:hover {
    color: var(--accent);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 22px;
    border-top: 1px solid var(--border);
  }
</style>
