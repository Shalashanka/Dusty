<script lang="ts">
  import { fly, fade, scale } from "svelte/transition";
  import { cubicOut, backOut } from "svelte/easing";
  import { LayoutGrid, List, Undo2, ArrowLeft, ChevronDown, Trash2, Flame, Heart, AlarmClock, Sparkles, FolderOpen, ExternalLink } from "@lucide/svelte";
  import FindingCard from "$lib/components/FindingCard.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import Confetti from "$lib/components/Confetti.svelte";
  import Counter from "$lib/components/Counter.svelte";
  import DriveChips from "$lib/components/DriveChips.svelte";
  import { app, driveOf } from "$lib/app.svelte";
  import { api, type Action, type Finding, type Kind } from "$lib/api";
  import { ACTIONS, DESTRUCTIVE, KINDS, KIND_ORDER, categoryOf } from "$lib/kinds";
  import { age, bytes, shortPath, parent } from "$lib/format";
  import { menu } from "$lib/menu.svelte";

  type GroupBy = "none" | "type" | "kind" | "folder" | "drive";

  let mode = $state<"cards" | "list">(load("dusty.triage.mode", new URLSearchParams(location.search).get("mode") ?? "cards") as "cards" | "list");
  let groupBy = $state<GroupBy>(load("dusty.triage.group", "type") as GroupBy);
  let done = $state(0);
  let lastBatches = $state<string[]>([]);
  let collapsed = $state<Set<string>>(new Set());

  const list = $derived(app.filter === "all" ? app.findings : app.findings.filter((f) => f.kind === app.filter));
  const top = $derived(list[0]);
  const kindsPresent = $derived(KIND_ORDER.filter((k) => app.byKind.get(k)?.length));
  const total = $derived(list.length + done);
  const progress = $derived(total ? done / total : 0);

  function load(k: string, d: string): string {
    try {
      return localStorage.getItem(k) ?? d;
    } catch {
      return d;
    }
  }
  function save(k: string, v: string) {
    try {
      localStorage.setItem(k, v);
    } catch {}
  }
  function setMode(m: "cards" | "list") {
    mode = m;
    save("dusty.triage.mode", m);
  }
  function setGroup(g: GroupBy) {
    groupBy = g;
    collapsed = new Set();
    save("dusty.triage.group", g);
  }

  function primaryOf(f: Finding): Action {
    return DESTRUCTIVE.includes(f.suggested) ? f.suggested : (f.actions.find((a) => DESTRUCTIVE.includes(a)) ?? f.suggested);
  }
  function secondaryOf(f: Finding): Action | null {
    return f.actions.includes("keep") ? "keep" : f.actions.includes("snooze") ? "snooze" : null;
  }

  // ---- card motion -------------------------------------------------------
  let dx = $state(0);
  let dy = $state(0);
  let dragging = $state(false);
  let exitDir = 0;
  let startX = 0;
  let startY = 0;
  let busy = false;
  let puffs = $state<number[]>([]);
  function puff() {
    const id = Date.now();
    puffs = [...puffs, id];
    setTimeout(() => (puffs = puffs.filter((p) => p !== id)), 900);
  }

  function flyAway(_node: Element) {
    const dir = exitDir;
    const fromX = dx;
    return {
      duration: 420,
      easing: cubicOut,
      css: (t: number, u: number) =>
        dir === 0
          ? `transform: translateY(${u * 60}px) scale(${1 - u * 0.1}); opacity: ${t}`
          : `transform: translateX(${fromX + dir * u * 900}px) translateY(${u * 60}px) rotate(${dir * (6 + u * 22)}deg); opacity: ${t}`,
    };
  }

  async function decide(f: Finding | undefined, action: Action) {
    if (!f || busy) return;
    if (action === "organize") {
      app.planFolder = f.path;
      return;
    }
    busy = true;
    exitDir = action === "keep" ? 1 : action === "snooze" ? 0 : -1;
    if (exitDir === -1) puff();
    const out = await app.act([f], action, true);
    dx = 0;
    dy = 0;
    busy = false;
    if (out && out.done) {
      done++;
      if (!["delete", "empty", "quit"].includes(action)) lastBatches = [...lastBatches.slice(-30), out.batch];
    }
  }

  async function undoLast() {
    const b = lastBatches.at(-1);
    if (!b) return;
    lastBatches = lastBatches.slice(0, -1);
    done = Math.max(0, done - 1);
    await app.undoBatch(b);
  }

  function down(e: PointerEvent) {
    if ((e.target as HTMLElement).closest("button, video, audio, iframe, pre")) return;
    dragging = true;
    startX = e.clientX;
    startY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!dragging) return;
    dx = e.clientX - startX;
    dy = e.clientY - startY;
  }
  function up() {
    if (!dragging) return;
    dragging = false;
    const f = top;
    if (f && dx < -130) decide(f, primaryOf(f));
    else if (f && dx > 130 && secondaryOf(f)) decide(f, secondaryOf(f)!);
    else {
      dx = 0;
      dy = 0;
    }
  }

  function onkey(e: KeyboardEvent) {
    if (app.view !== "triage" || app.paletteOpen || app.planFolder || menu.open) return;
    if ((e.target as HTMLElement)?.closest("input, select, textarea")) return;
    const k = e.key.toLowerCase();
    if (k === "z") {
      e.preventDefault();
      undoLast();
      return;
    }
    if (k === "l") return setMode(mode === "cards" ? "list" : "cards");
    if (mode !== "cards" || !top) return;
    const f = top;
    const map: Record<string, Action | null> = {
      arrowleft: primaryOf(f),
      d: primaryOf(f),
      delete: primaryOf(f),
      arrowright: secondaryOf(f),
      k: f.actions.includes("keep") ? "keep" : null,
      arrowdown: "snooze",
      s: "snooze",
      a: f.actions.includes("archive") ? "archive" : null,
    };
    if (k === "o") {
      e.preventDefault();
      api.reveal(f.path);
      return;
    }
    if (k === "enter" && !f.isDir) {
      e.preventDefault();
      api.openFile(f.path);
      return;
    }
    const a = map[k];
    if (a && (f.actions.includes(a) || a === "snooze")) {
      e.preventDefault();
      decide(f, a);
    }
  }

  // ---- list mode ---------------------------------------------------------
  let selected = $state<Set<string>>(new Set());
  const selItems = $derived(list.filter((f) => selected.has(f.id)));
  const selBytes = $derived(selItems.reduce((a, f) => a + f.bytes, 0));
  const canRecycle = $derived(selItems.length > 0 && selItems.every((f) => f.actions.includes("recycle") || f.actions.includes("delete")) && selItems.every((f) => !["system_junk", "folder_plan", "desktop_clutter", "recycle_bin", "new_startup", "resource_hog"].includes(f.kind)));
  const canDelete = $derived(selItems.length > 0 && selItems.every((f) => f.actions.includes("delete") || f.actions.includes("recycle")) && selItems.every((f) => !["folder_plan", "desktop_clutter", "recycle_bin", "new_startup", "resource_hog"].includes(f.kind)));

  function keyOf(f: Finding): string {
    switch (groupBy) {
      case "type":
        if (f.kind === "system_junk") return "Caches & temp";
        if (f.kind === "new_startup" || f.kind === "resource_hog" || f.kind === "recycle_bin") return KINDS[f.kind].label;
        return categoryOf(f.ext, f.isDir);
      case "kind":
        return KINDS[f.kind].label;
      case "folder":
        return shortPath(parent(f.path), app.home);
      case "drive":
        return driveOf(f.path) || "System";
      default:
        return "";
    }
  }
  const groups = $derived.by(() => {
    if (groupBy === "none") return [["", list]] as [string, Finding[]][];
    const m = new Map<string, Finding[]>();
    for (const f of list) {
      const k = keyOf(f);
      m.set(k, [...(m.get(k) ?? []), f]);
    }
    return [...m.entries()].sort((a, b) => b[1].reduce((s, f) => s + f.bytes, 0) - a[1].reduce((s, f) => s + f.bytes, 0));
  });

  function toggle(id: string) {
    const s = new Set(selected);
    s.has(id) ? s.delete(id) : s.add(id);
    selected = s;
  }
  function toggleMany(items: Finding[]) {
    const all = items.every((f) => selected.has(f.id));
    const s = new Set(selected);
    for (const f of items) all ? s.delete(f.id) : s.add(f.id);
    selected = s;
  }
  function toggleCollapse(k: string) {
    const s = new Set(collapsed);
    s.has(k) ? s.delete(k) : s.add(k);
    collapsed = s;
  }
  async function bulk(a: Action | "smart") {
    const items = selItems;
    selected = new Set();
    if (a === "smart") {
      await app.removeSmart(items);
      done += items.length;
      return;
    }
    const out = await app.act(items, a);
    if (out?.done) done += out.done;
  }
  function chipFilter(k: Kind | "all") {
    app.filter = k;
    selected = new Set();
  }
  function rowMenu(e: MouseEvent, f: Finding) {
    menu.show(e, [
      { label: "Open", icon: ExternalLink, run: () => api.openFile(f.path), disabled: f.isDir && f.kind !== "unsure" },
      { label: "Show in folder", icon: FolderOpen, run: () => api.reveal(f.path) },
      { divider: true },
      ...f.actions.map((a) => ({ label: ACTIONS[a].label, icon: ACTIONS[a].icon, danger: ACTIONS[a].tone === "danger", run: () => decideList(f, a) })),
      { divider: true },
      ...((f.kind === "duplicate" || f.kind === "similar_photo") && f.related[0]
        ? [{ label: "These folders are backups of each other…", run: () => { app.mirrorPair = { a: f.path, b: f.related[0] }; } }]
        : []),
      { label: "Don't touch for a month", run: () => app.protect(f.path, 30, `“${f.title}”`) },
      { label: "Never touch this", run: () => app.protect(f.path, 0, `“${f.title}”`) },
    ]);
  }
  function decideList(f: Finding, a: Action) {
    if (a === "organize") app.planFolder = f.path;
    else app.act([f], a);
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="triage">
  <div class="toolbar">
    <div class="filters">
      <button class="chip f" class:on={app.filter === "all"} onclick={() => chipFilter("all")}>
        All <b>{app.findings.length}</b>
      </button>
      {#each kindsPresent as k (k)}
        <button class="chip f" class:on={app.filter === k} style="--c:{KINDS[k].color}" onclick={() => chipFilter(k)}>
          <i class="dot"></i>
          {KINDS[k].short} <b>{app.byKind.get(k)?.length}</b>
        </button>
      {/each}
    </div>
    <DriveChips />
    <div class="modes">
      <button class:on={mode === "cards"} onclick={() => setMode("cards")} title="One at a time (L)"><LayoutGrid size={16} /></button>
      <button class:on={mode === "list"} onclick={() => setMode("list")} title="List (L)"><List size={16} /></button>
    </div>
  </div>

  {#if !list.length}
    <div class="empty" in:fade={{ duration: 300 }}>
      {#if done > 0}<Confetti />{/if}
      <div in:scale={{ start: 0.8, duration: 600, easing: backOut }}>
        <Mascot mood="happy" size={170} />
      </div>
      <h1>{done > 0 ? "Squeaky clean!" : "Nothing to tidy here"}</h1>
      <p class="muted">
        {#if app.sessionFreed > 0}
          You freed <b>{bytes(app.sessionFreed)}</b> and made {done} decisions. Your future self says thanks.
        {:else if done > 0}
          {done} decisions made. Your future self says thanks.
        {:else}
          I'll let you know when something starts gathering dust.
        {/if}
      </p>
      <div class="row">
        {#if app.filter !== "all" && app.findings.length}
          <button class="btn primary" onclick={() => chipFilter("all")}>See everything else ({app.findings.length})</button>
        {/if}
        <button class="btn" onclick={() => app.go("home")}><ArrowLeft size={16} /> Back home</button>
      </div>
    </div>
  {:else if mode === "cards"}
    <div class="stage">
      <div class="stack">
        {#each puffs as p (p)}
          <div class="puff" aria-hidden="true">
            {#each Array(10) as _, k}<i style="--a:{k * 36}deg;--d:{60 + (k % 3) * 25}px"></i>{/each}
          </div>
        {/each}
        {#each list.slice(0, 3).reverse() as f, ri (f.id)}
          {@const i = Math.min(list.length, 3) - 1 - ri}
          <div
            class="slot"
            role="group"
            class:front={i === 0}
            class:dragging={i === 0 && dragging}
            style="--i:{i}; {i === 0 ? `transform: translate(${dx}px, ${dy * 0.15}px) rotate(${dx / 22}deg)` : ''}"
            onpointerdown={i === 0 ? down : undefined}
            onpointermove={i === 0 ? move : undefined}
            onpointerup={i === 0 ? up : undefined}
            onpointercancel={i === 0 ? up : undefined}
            out:flyAway
            in:fly={{ y: 40, duration: 350, easing: cubicOut }}
          >
            <FindingCard {f} primary={primaryOf(f)} secondary={secondaryOf(f)} dx={i === 0 ? dx : 0} active={i === 0} onact={(a) => decide(f, a)} />
          </div>
        {/each}
      </div>

      <div class="progress">
        <div class="bar"><span style="width:{progress * 100}%"></span></div>
        <div class="progress-row">
          <span class="muted tabular">{done} done · {list.length} to go</span>
          <span class="freed"><Counter value={app.sessionFreed} kind="bytes" /> <span class="faint">freed this session</span></span>
          <button class="btn sm ghost" disabled={!lastBatches.length} onclick={undoLast}><Undo2 size={14} /> Undo <span class="kbd">Z</span></button>
        </div>
        <div class="hints faint">
          <span><span class="kbd">←</span> / drag left to remove</span>
          <span><span class="kbd">→</span> keep</span>
          <span><span class="kbd">↓</span> later</span>
          <span><span class="kbd">Enter</span> open</span>
          <span><span class="kbd">O</span> show in folder</span>
        </div>
      </div>
    </div>
  {:else}
    <div class="list-head">
      <input type="checkbox" checked={selected.size === list.length && list.length > 0} onchange={() => toggleMany(list)} />
      <span class="muted">{selected.size ? `${selected.size} selected · ${bytes(selBytes)}` : `${list.length} items · ${bytes(list.reduce((a, f) => a + f.bytes, 0))}`}</span>
      <span class="sp"></span>
      <span class="faint">Group by</span>
      <div class="seg">
        {#each [["type", "Type"], ["kind", "Reason"], ["folder", "Folder"], ["drive", "Drive"], ["none", "None"]] as [g, label] (g)}
          <button class:on={groupBy === g} onclick={() => setGroup(g as GroupBy)}>{label}</button>
        {/each}
      </div>
    </div>
    <div class="list scroll">
      {#each groups as [g, items] (g)}
        {#if g}
          {@const all = items.every((f) => selected.has(f.id))}
          <div class="ghead">
            <input type="checkbox" checked={all} onchange={() => toggleMany(items)} />
            <button class="gtitle" onclick={() => toggleCollapse(g)}>
              <span class="chev" class:closed={collapsed.has(g)}><ChevronDown size={15} /></span>
              <b class="ellipsis">{g}</b>
              <span class="faint">{items.length} · {bytes(items.reduce((a, f) => a + f.bytes, 0))}</span>
            </button>
          </div>
        {/if}
        {#if !collapsed.has(g)}
          {#each items as f (f.id)}
            <div class="lrow" class:sel={selected.has(f.id)} oncontextmenu={(e) => rowMenu(e, f)} role="row" tabindex="-1" out:fly={{ x: -60, duration: 250 }}>
              <input type="checkbox" checked={selected.has(f.id)} onchange={() => toggle(f.id)} />
              <Preview path={f.path} ext={f.ext} isDir={f.isDir} size={34} color={f.kind === "system_junk" || f.kind === "dev_junk" ? KINDS[f.kind].color : ""} />
              <button class="lname" onclick={() => toggle(f.id)} ondblclick={() => (f.isDir ? api.reveal(f.path) : api.openFile(f.path))}>
                <b class="ellipsis">{f.title}</b>
                <span class="ellipsis faint mono">{shortPath(f.path, app.home)}</span>
              </button>
              <span class="lkind" style="--c:{KINDS[f.kind].color}"><i class="dot"></i>{KINDS[f.kind].short}</span>
              <span class="lage faint">{f.lastTouched ? age(f.lastTouched) : ""}</span>
              <span class="lsize tabular">{bytes(f.bytes)}</span>
              <span class="lacts">
                {#each [primaryOf(f), secondaryOf(f)].filter(Boolean) as a (a)}
                  {@const I = ACTIONS[a!].icon}
                  <button class="icon-btn {ACTIONS[a!].tone}" title={ACTIONS[a!].label} onclick={() => decideList(f, a!)}><I size={15} /></button>
                {/each}
              </span>
            </div>
          {/each}
        {/if}
      {/each}
    </div>
    {#if selItems.length}
      <div class="bulk" transition:fly={{ y: 40, duration: 300, easing: cubicOut }}>
        <span><b>{selItems.length}</b> · {bytes(selBytes)}</span>
        <button class="btn sm primary" onclick={() => bulk("smart")} title="Each item gets its own best removal: recycle, delete for good (caches), sweep..."><Sparkles size={14} /> Remove</button>
        {#if canRecycle}<button class="btn sm" onclick={() => bulk("recycle")} title={ACTIONS.recycle.hint}><Trash2 size={14} /> Recycle</button>{/if}
        {#if canDelete}<button class="btn sm danger" onclick={() => bulk("delete")} title={ACTIONS.delete.hint}><Flame size={14} /> Delete for good</button>{/if}
        <button class="btn sm" onclick={() => bulk("keep")}><Heart size={14} /> Keep</button>
        <button class="btn sm" onclick={() => bulk("snooze")}><AlarmClock size={14} /> Later</button>
        <button class="btn sm ghost" onclick={() => (selected = new Set())}>Clear</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .triage {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 42px 32px 18px;
    position: relative;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .filters {
    flex: 1;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 2px;
    mask-image: linear-gradient(to right, black 92%, transparent);
  }
  .filters::-webkit-scrollbar {
    height: 0;
  }
  .f {
    cursor: pointer;
    transition: all 0.2s;
  }
  .f b {
    color: var(--text);
    font-weight: 600;
  }
  .f:hover {
    border-color: var(--border-strong);
  }
  .f.on {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    color: var(--text);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--c, var(--accent));
  }
  .modes,
  .seg {
    display: flex;
    padding: 3px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    flex: none;
  }
  .modes button {
    width: 34px;
    height: 28px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .seg button {
    height: 26px;
    padding: 0 11px;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
  }
  .modes button.on,
  .seg button.on {
    background: var(--surface-3);
    color: var(--text);
  }

  .stage {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-height: 0;
  }
  .stack {
    position: relative;
    width: min(720px, 100%);
    flex: 1;
    max-height: 560px;
    min-height: 380px;
    margin-top: 6px;
  }
  .puff {
    position: absolute;
    left: 18%;
    top: 45%;
    z-index: 10;
    pointer-events: none;
  }
  .puff i {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(215, 207, 255, 0.85), rgba(215, 207, 255, 0));
    animation: burst 0.85s cubic-bezier(0.2, 0.8, 0.3, 1) both;
  }
  @keyframes burst {
    from {
      transform: rotate(var(--a)) translateX(0) scale(0.6);
      opacity: 1;
    }
    to {
      transform: rotate(var(--a)) translateX(var(--d)) scale(2);
      opacity: 0;
    }
  }
  .slot {
    position: absolute;
    inset: 0;
    transition:
      transform 0.45s var(--ease),
      opacity 0.3s,
      filter 0.3s;
    transform: translateY(calc(var(--i) * 16px)) scale(calc(1 - var(--i) * 0.045));
    transform-origin: 50% 0;
    opacity: calc(1 - var(--i) * 0.3);
    filter: blur(calc(var(--i) * 0.6px));
    pointer-events: none;
  }
  .slot.front {
    pointer-events: auto;
    cursor: grab;
    touch-action: none;
  }
  .slot.dragging {
    transition: none;
    cursor: grabbing;
  }
  .progress {
    width: min(720px, 100%);
    margin-top: 40px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .bar {
    height: 5px;
    border-radius: 99px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--grad);
    border-radius: 99px;
    transition: width 0.6s var(--ease);
  }
  .progress-row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .freed {
    margin-left: auto;
    font-weight: 600;
  }
  .freed .faint {
    font-weight: 400;
  }
  .hints {
    display: flex;
    gap: 16px;
    justify-content: center;
    font-size: 12px;
    flex-wrap: wrap;
  }
  .hints span {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }

  .empty {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 8px;
  }
  .empty h1 {
    font-size: 28px;
    margin-top: 10px;
  }
  .empty p {
    max-width: 440px;
    margin: 0 0 14px;
  }
  .row {
    display: flex;
    gap: 8px;
  }

  .list-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px 10px;
    font-size: 12.5px;
  }
  .sp {
    flex: 1;
  }
  .list {
    flex: 1;
    margin: 0 -8px;
    padding: 0 8px 90px;
  }
  .ghead {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    margin-top: 8px;
    border-radius: var(--r);
    background: var(--bg-solid);
    border: 1px solid var(--border);
  }
  .gtitle {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    text-align: left;
  }
  .gtitle .faint {
    margin-left: auto;
    font-size: 12.5px;
  }
  .chev {
    display: grid;
    transition: transform 0.2s;
  }
  .chev.closed {
    transform: rotate(-90deg);
  }
  .lrow {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr) 110px 90px 80px auto;
    align-items: center;
    gap: 12px;
    padding: 7px 12px;
    border-radius: var(--r);
    transition: background 0.15s;
  }
  .lrow:hover {
    background: var(--surface);
  }
  .lrow.sel {
    background: var(--accent-soft);
  }
  .lname {
    display: flex;
    flex-direction: column;
    text-align: left;
    min-width: 0;
  }
  .lname b {
    font-weight: 600;
  }
  .lkind {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .lage {
    font-size: 12.5px;
  }
  .lsize {
    text-align: right;
    font-weight: 600;
  }
  .lacts {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .lrow:hover .lacts {
    opacity: 1;
  }
  .icon-btn {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
  }
  .icon-btn.danger:hover {
    background: var(--danger);
    color: #fff;
  }
  .icon-btn.ok:hover {
    background: var(--ok);
    color: #0b2a22;
  }
  .icon-btn.neutral:hover,
  .icon-btn.accent:hover {
    background: var(--accent);
    color: #fff;
  }
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
    flex: none;
  }
  .bulk {
    position: absolute;
    bottom: 22px;
    left: 50%;
    translate: -50% 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px 8px 18px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-solid) 92%, white 8%);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    white-space: nowrap;
    z-index: 5;
  }
</style>
