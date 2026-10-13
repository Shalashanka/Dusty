<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { ArrowRight, RefreshCw, HardDrive, Sparkles, Flame, CalendarCheck, Leaf } from "@lucide/svelte";
  import Mascot, { type Mood } from "$lib/components/Mascot.svelte";
  import Counter from "$lib/components/Counter.svelte";
  import Ring from "$lib/components/Ring.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type Kind, type Finding } from "$lib/api";
  import { KINDS, KIND_ORDER } from "$lib/kinds";
  import { ago, bytes, clock, greeting, plural, until } from "$lib/format";

  const mood: Mood = $derived(
    app.scanning ? "sweeping" : app.paused ? "sleeping" : app.findings.length ? "curious" : "happy",
  );
  const minutes = $derived(Math.max(1, Math.ceil((app.findings.length * 3) / 60)));
  const groups = $derived(KIND_ORDER.filter((k) => app.byKind.get(k)?.length).map((k) => [k, app.byKind.get(k)!] as const));

  let confirming = $state<Kind | null>(null);
  let confirmTimer: ReturnType<typeof setTimeout>;

  async function cleanAll(kind: Kind, list: Finding[]) {
    if (confirming !== kind) {
      confirming = kind;
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(() => (confirming = null), 3000);
      return;
    }
    confirming = null;
    const byAction = Map.groupBy(list, (f) => f.suggested);
    for (const [action, items] of byAction) await app.act(items, action);
  }

  async function lookNow() {
    await api.scanNow();
    await app.refresh();
  }
</script>

<div class="home scroll">
  <section class="hero" in:fade={{ duration: 300 }}>
    <div class="mascot-wrap">
      <Mascot {mood} size={176} />
    </div>
    <div class="copy">
      <p class="hello">{greeting()}.</p>
      {#key mood + app.phase}
        <h1 in:fly={{ y: 10, duration: 400, easing: cubicOut }}>
          {#if app.scanning}
            {app.phase || "Looking around"}<span class="dots"><i>.</i><i>.</i><i>.</i></span>
          {:else if app.paused}
            Napping until {clock(app.snap!.pausedUntil)}.
          {:else if app.findings.length}
            I found {plural(app.findings.length, "thing")} you probably don't need.
          {:else}
            Everything looks tidy.
          {/if}
        </h1>
      {/key}
      <p class="sub">
        {#if app.findings.length}
          <b class="grad-text">{bytes(app.pendingBytes)}</b> could be freed. A quick pass takes about {minutes}
          {minutes === 1 ? "minute" : "minutes"}.
        {:else if !app.scanning}
          I'll keep an eye out and ping you when something starts gathering dust.
        {:else}
          I work at low priority, so you won't notice me.
        {/if}
      </p>
      <div class="cta">
        <button class="btn primary lg" disabled={!app.findings.length} onclick={() => app.go("triage")}>
          Start tidying <ArrowRight size={18} />
        </button>
        <button class="btn lg" disabled={app.scanning} onclick={lookNow}>
          <span class="spin" class:on={app.scanning}><RefreshCw size={16} /></span>
          {app.scanning ? "Looking…" : "Look around now"}
        </button>
      </div>
      <p class="meta faint">
        Last look {ago(app.snap?.lastScanAt ?? 0)}
        {#if !app.paused && !app.scanning && app.snap?.nextScanAt}
          · next one {until(app.snap.nextScanAt)}, once you step away
        {/if}
      </p>
    </div>
  </section>

  <section class="stats">
    {#each [
      { label: "Ready to free", value: app.pendingBytes, icon: Sparkles, kind: "bytes" as const },
      { label: "Freed this week", value: app.snap?.stats.freedWeek ?? 0, icon: Leaf, kind: "bytes" as const },
      { label: "Freed all-time", value: app.snap?.stats.freedTotal ?? 0, icon: Flame, kind: "bytes" as const },
      { label: "Tidy streak", value: app.snap?.stats.streakDays ?? 0, icon: CalendarCheck, kind: "number" as const, unit: "days" },
    ] as s, i (s.label)}
      {@const Icon = s.icon}
      <div class="stat card" in:fly={{ y: 14, duration: 450, delay: 60 + i * 50, easing: cubicOut }}>
        <span class="stat-icon"><Icon size={16} /></span>
        <span class="stat-value"><Counter value={s.value} kind={s.kind} />{#if s.unit}<small> {s.unit}</small>{/if}</span>
        <span class="stat-label faint">{s.label}</span>
      </div>
    {/each}
  </section>

  {#if app.drives.length}
    <section class="drives">
      {#each app.drives as d, i (d.mount)}
        {@const used = 1 - d.free / d.total}
        <div class="drive card" in:fly={{ y: 14, duration: 450, delay: 120 + i * 50, easing: cubicOut }}>
          <div class="ring">
            <Ring value={used} size={58} stroke={6} color={used > 0.9 ? "var(--danger)" : used > 0.75 ? "var(--warn)" : "var(--accent)"} />
            <span class="pct tabular">{Math.round(used * 100)}%</span>
          </div>
          <div class="drive-info">
            <b><HardDrive size={14} /> {d.mount.replace(/\\$/, "")} {d.name ? `· ${d.name}` : ""}</b>
            <span class="muted">{bytes(d.free)} free of {bytes(d.total)}</span>
            <span class="faint small">{d.kind === "unknown" ? "" : d.kind.toUpperCase()} {d.fs}</span>
          </div>
        </div>
      {/each}
    </section>
  {/if}

  {#if groups.length}
    <h2 class="section-title">What I found</h2>
    <section class="kinds">
      {#each groups as [kind, list], i (kind)}
        {@const meta = KINDS[kind]}
        {@const Icon = meta.icon}
        {@const total = list.reduce((a, f) => a + f.bytes, 0)}
        <div
          class="kind card"
          style="--c:{meta.color}"
          in:fly={{ y: 18, duration: 500, delay: 160 + i * 45, easing: cubicOut }}
          role="button"
          tabindex="0"
          onclick={() => app.go("triage", kind)}
          onkeydown={(e) => e.key === "Enter" && app.go("triage", kind)}
        >
          <div class="kind-head">
            <span class="kind-icon"><Icon size={18} /></span>
            <span class="kind-count chip">{list.length}</span>
          </div>
          <div class="kind-size"><Counter value={total} kind="bytes" /></div>
          <div class="kind-label">{meta.label}</div>
          <div class="kind-preview faint">
            {#each list.slice(0, 2) as f (f.id)}<span class="ellipsis">{f.title}</span>{/each}
          </div>
          <div class="kind-foot">
            <button class="btn sm ghost" onclick={(e) => { e.stopPropagation(); app.go("triage", kind); }}>Review</button>
            {#if meta.bulk}
              <button
                class="btn sm {confirming === kind ? 'danger' : ''}"
                onclick={(e) => { e.stopPropagation(); cleanAll(kind, list); }}
              >
                {confirming === kind ? "Sure? Click again" : `Clean all`}
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .home {
    height: 100%;
    padding: 28px 40px 60px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 32px;
    padding: 10px 0 26px;
  }
  .mascot-wrap {
    flex: none;
  }
  .hello {
    margin: 0 0 4px;
    color: var(--text-2);
    font-size: 15px;
  }
  h1 {
    font-size: 30px;
    line-height: 1.15;
    max-width: 640px;
  }
  .sub {
    margin: 10px 0 20px;
    font-size: 15px;
    color: var(--text-2);
    max-width: 560px;
  }
  .grad-text {
    background: var(--grad);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    font-weight: 700;
  }
  .cta {
    display: flex;
    gap: 10px;
  }
  .meta {
    margin: 14px 0 0;
    font-size: 12.5px;
  }
  .spin {
    display: grid;
  }
  .spin.on {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .dots i {
    font-style: normal;
    animation: blink 1.4s infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.2s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes blink {
    0%,
    100% {
      opacity: 0.2;
    }
    50% {
      opacity: 1;
    }
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 12px;
  }
  .stat {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stat-icon {
    color: var(--accent);
    margin-bottom: 6px;
  }
  .stat-value {
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .stat-value small {
    font-size: 14px;
    color: var(--text-2);
    font-weight: 500;
  }
  .stat-label {
    font-size: 12.5px;
  }

  .drives {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }
  .drive {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
  }
  .ring {
    position: relative;
    display: grid;
    place-items: center;
  }
  .pct {
    position: absolute;
    font-size: 12px;
    font-weight: 700;
  }
  .drive-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .drive-info b {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .small {
    font-size: 11.5px;
  }

  .section-title {
    font-size: 17px;
    margin: 30px 0 12px;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 12px;
  }
  .kind {
    position: relative;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: pointer;
    overflow: hidden;
    transition:
      transform 0.25s var(--ease),
      border-color 0.25s,
      background 0.25s;
  }
  .kind::before {
    content: "";
    position: absolute;
    inset: -40% -40% auto auto;
    width: 160px;
    height: 160px;
    background: radial-gradient(circle, color-mix(in srgb, var(--c) 30%, transparent), transparent 70%);
    opacity: 0.5;
    transition: opacity 0.3s;
    pointer-events: none;
  }
  .kind:hover {
    transform: translateY(-3px);
    border-color: color-mix(in srgb, var(--c) 40%, transparent);
    background: var(--surface-2);
  }
  .kind:hover::before {
    opacity: 1;
  }
  .kind-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }
  .kind-icon {
    width: 36px;
    height: 36px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
  }
  .kind-size {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .kind-label {
    font-weight: 600;
  }
  .kind-preview {
    display: flex;
    flex-direction: column;
    font-size: 12px;
    margin: 6px 0 10px;
    min-height: 36px;
  }
  .kind-foot {
    display: flex;
    gap: 6px;
    margin-top: auto;
  }
  @media (max-width: 1100px) {
    .stats {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
