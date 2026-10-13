<script lang="ts">
  import { fly, fade, scale } from "svelte/transition";
  import { cubicOut, backOut } from "svelte/easing";
  import { flip } from "svelte/animate";
  import { LayoutGrid, List, Undo2, ArrowLeft } from "@lucide/svelte";
  import FindingCard from "$lib/components/FindingCard.svelte";
  import FileGlyph from "$lib/components/FileGlyph.svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import Confetti from "$lib/components/Confetti.svelte";
  import Counter from "$lib/components/Counter.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type Action, type Finding, type Kind } from "$lib/api";
  import { ACTIONS, KINDS, KIND_ORDER } from "$lib/kinds";
  import { age, bytes, shortPath } from "$lib/format";

  const DESTRUCTIVE: Action[] = ["recycle", "delete", "sweep", "empty", "archive"];

  let mode = $state<"cards" | "list">(load("dustling.triage.mode", "cards"));
  let done = $state(0);
  let lastBatches = $state<string[]>([]);
  let startCount = $state(0);

  const list = $derived(app.filter === "all" ? app.findings : app.findings.filter((f) => f.kind === app.filter));
  const top = $derived(list[0]);
  const kindsPresent = $derived(KIND_ORDER.filter((k) => app.byKind.get(k)?.length));
  const progress = $derived(startCount ? Math.min(1, done / Math.max(startCount, done + list.length)) : 0);

  $effect(() => {
    // Remember the size of the pile when a session starts or the filter changes.
    app.filter;
    startCount = list.length + done;
  });

  function load<T>(k: string, d: T): T {
    try {
      return (localStorage.getItem(k) as T) ?? d;
    } catch {
      return d;
    }
  }
  function setMode(m: "cards" | "list") {
    mode = m;
    try {
      localStorage.setItem("dustling.triage.mode", m);
    } catch {}
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
    if (!f || busy || !f.actions.includes(action)) return;
    busy = true;
    exitDir = action === "keep" ? 1 : action === "snooze" ? 0 : -1;
    const out = await app.act([f], action, true);
    dx = 0;
    dy = 0;
    busy = false;
    if (out && out.done) {
      done++;
      if (!["delete", "empty"].includes(action)) lastBatches = [...lastBatches.slice(-30), out.batch];
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
    if ((e.target as HTMLElement).closest("button")) return;
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
    if (app.view !== "triage" || app.paletteOpen) return;
    if ((e.target as HTMLElement)?.closest("input, select, textarea")) return;
    const k = e.key.toLowerCase();
    if (k === "z" || (e.ctrlKey && k === "z")) {
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
    if (k === "o" || k === " ") {
      e.preventDefault();
      api.reveal(f.path);
      return;
    }
    const a = map[k];
    if (a) {
      e.preventDefault();
      decide(f, a);
    }
  }

  // ---- list mode ---------------------------------------------------------
  let selected = $state<Set<string>>(new Set());
  const selItems = $derived(list.filter((f) => selected.has(f.id)));
  const common = $derived.by(() => {
    if (!selItems.length) return [] as Action[];
    return (Object.keys(ACTIONS) as Action[]).filter((a) => selItems.every((f) => f.actions.includes(a)));
  });
  function toggle(id: string) {
    const s = new Set(selected);
    s.has(id) ? s.delete(id) : s.add(id);
    selected = s;
  }
  function toggleAll() {
    selected = selected.size === list.length ? new Set() : new Set(list.map((f) => f.id));
  }
  async function bulk(a: Action) {
    const items = selItems;
    selected = new Set();
    const out = await app.act(items, a);
    if (out?.done) done += out.done;
  }
  function chipFilter(k: Kind | "all") {
    app.filter = k;
    selected = new Set();
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
    <div class="modes">
      <button class:on={mode === "cards"} onclick={() => setMode("cards")} title="One at a time (L)"><LayoutGrid size={16} /></button>
      <button class:on={mode === "list"} onclick={() => setMode("list")} title="List (L)"><List size={16} /></button>
    </div>
  </div>

  {#if !list.length}
    <div class="empty" in:fade={{ duration: 300 }}>
      {#if done > 0}<Confetti />{/if}
      <div in:scale={{ start: 0.7, duration: 600, easing: backOut }}>
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
        {#each list.slice(0, 3).reverse() as f, ri (f.id)}
          {@const i = Math.min(list.length, 3) - 1 - ri}
          <div
            class="slot"
            role="group"
            class:top={i === 0}
            class:dragging={i === 0 && dragging}
            style="--i:{i}; {i === 0 ? `transform: translate(${dx}px, ${dy * 0.15}px) rotate(${dx / 22}deg)` : ''}"
            onpointerdown={i === 0 ? down : undefined}
            onpointermove={i === 0 ? move : undefined}
            onpointerup={i === 0 ? up : undefined}
            onpointercancel={i === 0 ? up : undefined}
            out:flyAway
            in:fly={{ y: 40, duration: 350, easing: cubicOut }}
          >
            <FindingCard {f} primary={primaryOf(f)} secondary={secondaryOf(f)} dx={i === 0 ? dx : 0} onact={(a) => decide(f, a)} />
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
          <span><span class="kbd">O</span> show in Explorer</span>
        </div>
      </div>
    </div>
  {:else}
    <div class="list scroll">
      <div class="lhead">
        <input type="checkbox" checked={selected.size === list.length && list.length > 0} onchange={toggleAll} />
        <span>{selected.size ? `${selected.size} selected · ${bytes(selItems.reduce((a, f) => a + f.bytes, 0))}` : `${list.length} items`}</span>
      </div>
      {#each list as f (f.id)}
        <div class="lrow" class:sel={selected.has(f.id)} animate:flip={{ duration: 300 }} out:fly={{ x: -60, duration: 250 }}>
          <input type="checkbox" checked={selected.has(f.id)} onchange={() => toggle(f.id)} />
          <FileGlyph ext={f.ext} isDir={f.isDir} size={34} color={f.kind === "system_junk" || f.kind === "dev_junk" ? KINDS[f.kind].color : ""} />
          <button class="lname" onclick={() => toggle(f.id)}>
            <b class="ellipsis">{f.title}</b>
            <span class="ellipsis faint mono">{shortPath(f.path, app.home)}</span>
          </button>
          <span class="lkind" style="--c:{KINDS[f.kind].color}"><i class="dot"></i>{KINDS[f.kind].short}</span>
          <span class="lage faint">{f.lastTouched ? age(f.lastTouched) : ""}</span>
          <span class="lsize tabular">{bytes(f.bytes)}</span>
          <span class="lacts">
            {#each [primaryOf(f), secondaryOf(f)].filter(Boolean) as a (a)}
              {@const I = ACTIONS[a!].icon}
              <button class="icon-btn {ACTIONS[a!].tone}" title={ACTIONS[a!].label} onclick={() => app.act([f], a!)}><I size={15} /></button>
            {/each}
          </span>
        </div>
      {/each}
    </div>
    {#if selItems.length}
      <div class="bulk" transition:fly={{ y: 40, duration: 300, easing: cubicOut }}>
        <span><b>{selItems.length}</b> selected</span>
        {#each common as a (a)}
          {@const I = ACTIONS[a].icon}
          <button class="btn sm {ACTIONS[a].tone === 'danger' ? 'danger' : ''}" onclick={() => bulk(a)}><I size={14} /> {ACTIONS[a].label}</button>
        {/each}
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
    padding: 22px 32px 18px;
    position: relative;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
    padding-right: 120px;
  }
  .filters {
    flex: 1;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 2px;
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
  .modes {
    display: flex;
    padding: 3px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .modes button {
    width: 34px;
    height: 28px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .modes button.on {
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
    width: min(680px, 100%);
    flex: 1;
    max-height: 450px;
    min-height: 360px;
    margin-top: 8px;
  }
  .slot {
    position: absolute;
    inset: 0;
    transition: transform 0.45s var(--spring), opacity 0.3s, filter 0.3s;
    transform: translateY(calc(var(--i) * 16px)) scale(calc(1 - var(--i) * 0.045));
    transform-origin: 50% 0;
    opacity: calc(1 - var(--i) * 0.3);
    filter: blur(calc(var(--i) * 0.6px));
    pointer-events: none;
  }
  .slot.top {
    pointer-events: auto;
    cursor: grab;
    touch-action: none;
  }
  .slot.dragging {
    transition: none;
    cursor: grabbing;
  }
  .progress {
    width: min(680px, 100%);
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
    gap: 18px;
    justify-content: center;
    font-size: 12px;
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

  .list {
    flex: 1;
    margin: 0 -8px;
    padding: 0 8px 80px;
  }
  .lhead {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    color: var(--text-2);
    font-size: 12.5px;
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--bg-solid);
    border-radius: var(--r);
  }
  .lrow {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr) 110px 90px 80px auto;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
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
  }
  .bulk {
    position: absolute;
    bottom: 22px;
    left: 50%;
    translate: -50% 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 8px 8px 18px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-solid) 92%, white 8%);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
  }
</style>
