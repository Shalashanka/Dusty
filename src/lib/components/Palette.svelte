<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Search, House, Layers, ChartPie, History, SlidersHorizontal, RefreshCw, Pause, Play, FolderOpen, CornerDownLeft, Gauge, Database } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { app } from "$lib/app.svelte";
  import { api, type Kind } from "$lib/api";
  import { KINDS, KIND_ORDER } from "$lib/kinds";

  interface Cmd {
    id: string;
    label: string;
    hint?: string;
    icon: typeof House;
    run: () => void | Promise<void>;
  }

  let q = $state("");
  let sel = $state(0);
  let input = $state<HTMLInputElement>();

  const cmds = $derived.by((): Cmd[] => {
    const base: Cmd[] = [
      { id: "tidy", label: "Start tidying", hint: `${app.findings.length} waiting`, icon: Layers, run: () => app.go("triage") },
      { id: "scan", label: "Look around now", icon: RefreshCw, run: async () => { await api.scanNow(); await app.refresh(); } },
      { id: "home", label: "Go to Home", icon: House, run: () => app.go("home") },
      { id: "space", label: "Explore disk space", icon: ChartPie, run: () => app.go("space") },
      { id: "history", label: "Open history", icon: History, run: () => app.go("history") },
      { id: "speed", label: "Speed up my PC", icon: Gauge, run: () => app.go("speed") },
      { id: "rules", label: "Settings", icon: SlidersHorizontal, run: () => app.go("rules") },
      { id: "search", label: "Search all files", hint: app.settings?.searchHotkey, icon: Search, run: () => invoke("toggle_search") },
      { id: "reindex", label: "Rebuild the file index", icon: Database, run: () => api.indexRebuild() },
      app.paused
        ? { id: "wake", label: "Wake up", icon: Play, run: async () => { await api.pause(0); await app.refresh(); } }
        : { id: "pause", label: "Pause for 3 hours", icon: Pause, run: async () => { await api.pause(180); await app.refresh(); } },
      ...(app.settings?.inboxDirs ?? []).map((d) => ({ id: "open:" + d, label: `Open ${d.split("\\").pop()}`, hint: d, icon: FolderOpen, run: () => api.openPath(d) })),
    ];
    const kinds: Cmd[] = KIND_ORDER.filter((k) => app.byKind.get(k)?.length).map((k: Kind) => ({
      id: "kind:" + k,
      label: `Review ${KINDS[k].label.toLowerCase()}`,
      hint: `${app.byKind.get(k)!.length}`,
      icon: KINDS[k].icon,
      run: () => app.go("triage", k),
    }));
    const all = [...base, ...kinds];
    const needle = q.trim().toLowerCase();
    return needle ? all.filter((c) => (c.label + " " + (c.hint ?? "")).toLowerCase().includes(needle)) : all;
  });

  $effect(() => {
    q;
    sel = 0;
  });
  $effect(() => {
    if (app.paletteOpen) {
      q = "";
      setTimeout(() => input?.focus(), 10);
    }
  });

  function run(c: Cmd | undefined) {
    if (!c) return;
    app.paletteOpen = false;
    c.run();
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(cmds.length - 1, sel + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(0, sel - 1);
    } else if (e.key === "Enter") {
      run(cmds[sel]);
    } else if (e.key === "Escape") {
      app.paletteOpen = false;
    }
  }
</script>

{#if app.paletteOpen}
  <div class="veil" transition:fade={{ duration: 150 }} onclick={() => (app.paletteOpen = false)} role="presentation"></div>
  <div class="palette" transition:scale={{ start: 0.96, duration: 200, easing: cubicOut }} role="dialog" aria-label="Command palette">
    <div class="search">
      <Search size={17} class="faint" />
      <input bind:this={input} bind:value={q} placeholder="What should I do?" onkeydown={key} />
      <span class="kbd">Esc</span>
    </div>
    <div class="results scroll">
      {#each cmds as c, i (c.id)}
        {@const I = c.icon}
        <button class="cmd" class:sel={i === sel} onmouseenter={() => (sel = i)} onclick={() => run(c)}>
          <I size={16} />
          <span class="ellipsis">{c.label}</span>
          {#if c.hint}<span class="hint faint ellipsis">{c.hint}</span>{/if}
          {#if i === sel}<CornerDownLeft size={14} class="faint" />{/if}
        </button>
      {:else}
        <p class="faint none">No matching command.</p>
      {/each}
    </div>
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(2px);
    z-index: 200;
  }
  .palette {
    position: fixed;
    top: 14%;
    left: 50%;
    translate: -50% 0;
    width: min(560px, 90vw);
    z-index: 201;
    border-radius: var(--r-lg);
    background: color-mix(in srgb, var(--bg-solid) 94%, white 6%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 80px -20px rgba(0, 0, 0, 0.7);
    overflow: hidden;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  input {
    flex: 1;
    background: none;
    border: 0;
    outline: none;
    font-size: 15px;
  }
  .results {
    max-height: 360px;
    padding: 6px;
  }
  .cmd {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 10px;
    text-align: left;
    color: var(--text-2);
  }
  .cmd.sel {
    background: var(--accent-soft);
    color: var(--text);
  }
  .hint {
    margin-left: auto;
    font-size: 12px;
    max-width: 40%;
  }
  .none {
    padding: 14px;
    margin: 0;
  }
</style>
