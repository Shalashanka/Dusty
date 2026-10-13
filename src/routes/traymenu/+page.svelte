<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { House, RefreshCw, WandSparkles, Search, Mic, Moon, Sun, Layers, Power } from "@lucide/svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import { api, type Settings, type Snapshot } from "$lib/api";
  import { bytes, clock } from "$lib/format";

  let snap = $state<Snapshot | null>(null);
  let settings = $state<Settings | null>(null);
  let opened = $state(0);

  async function load() {
    [snap, settings] = await Promise.all([api.snapshot().catch(() => null), api.getSettings().catch(() => null)]);
    opened++;
  }
  onMount(async () => {
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) (await import("$lib/mock")).installMocks();
    await load();
    await listen("traymenu-open", load);
  });

  const paused = $derived((snap?.pausedUntil ?? 0) > Date.now() / 1000);
  const n = $derived(snap?.findings.length ?? 0);
  const pending = $derived(snap?.findings.reduce((a, f) => a + f.bytes, 0) ?? 0);
  const mood = $derived(snap?.scanning ? "sweeping" : paused ? "sleeping" : n ? "curious" : "happy");
  const status = $derived(
    snap?.scanning
      ? "Looking around…"
      : paused
        ? `Napping until ${clock(snap!.pausedUntil)}`
        : n
          ? `${n} thing${n === 1 ? "" : "s"} to review · ${bytes(pending)}`
          : "All tidy ✨",
  );

  async function run(fn: () => Promise<unknown>) {
    await api.hideTraymenu();
    await fn();
  }
  const items = $derived([
    { label: "Open Dusty", icon: House, run: () => api.barCommand("view:home") },
    ...(n ? [{ label: "Start tidying", icon: Layers, run: () => api.barCommand("view:triage") }] : []),
    { label: "Clean the sure ones", icon: WandSparkles, run: () => api.barCommand("clean_sure") },
    { label: "Look around now", icon: RefreshCw, run: () => api.barCommand("scan") },
    { label: "Search files", hint: settings?.searchHotkey, icon: Search, run: () => api.openBar("search") },
    { label: "Ask by voice", hint: settings?.voiceHotkey, icon: Mic, run: () => api.openBar("voice") },
    paused
      ? { label: "Wake up", icon: Sun, run: () => api.barCommand("resume") }
      : { label: "Pause for 3 hours", icon: Moon, run: () => api.barCommand("pause") },
  ]);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && api.hideTraymenu()} />

{#key opened}
  <div class="menu" in:fly={{ y: 10, duration: 220, easing: cubicOut }}>
    <header>
      <div class="m"><Mascot {mood} size={54} /></div>
      <div class="st">
        <b>Dusty</b>
        <span class="faint">{status}</span>
      </div>
    </header>
    <div class="items">
      {#each items as it, i (it.label)}
        {@const I = it.icon}
        <button class="item" style="animation-delay:{40 + i * 25}ms" onclick={() => run(it.run)}>
          <span class="ic"><I size={15} /></span>
          <span class="lbl">{it.label}</span>
          {#if "hint" in it && it.hint}<span class="hint">{it.hint}</span>{/if}
        </button>
      {/each}
    </div>
    <div class="sep"></div>
    <button class="item quit" onclick={() => run(() => api.quitApp())}>
      <span class="ic"><Power size={15} /></span>
      <span class="lbl">Quit</span>
    </button>
  </div>
{/key}

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
  }
  .menu {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    padding: 8px;
    background: color-mix(in srgb, var(--bg-solid) 60%, transparent);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px 12px;
    margin-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }
  .m {
    width: 54px;
    height: 54px;
    margin: -4px 0;
  }
  .st {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-size: 12px;
  }
  .st b {
    font-family: var(--font-display);
    font-size: 15px;
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    border-radius: 9px;
    font-size: 13px;
    text-align: left;
    animation: in 0.28s var(--ease) both;
    transition: background 0.12s;
  }
  .item:hover {
    background: var(--accent-soft);
  }
  .item:hover .ic {
    transform: scale(1.08) rotate(-4deg);
  }
  .ic {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    transition: transform 0.2s var(--ease);
  }
  .lbl {
    flex: 1;
  }
  .hint {
    font-size: 10.5px;
    color: var(--text-3);
    font-family: var(--mono);
  }
  .sep {
    height: 1px;
    margin: 6px 6px;
    background: var(--border);
  }
  .quit .ic {
    color: var(--text-2);
    background: var(--surface-2);
  }
  .quit:hover {
    background: var(--danger-soft);
  }
  .quit:hover .ic {
    color: var(--danger);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
</style>
