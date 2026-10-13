<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { ChevronRight, House, HardDrive, ExternalLink, Trash2, FolderOpen, ArrowUp } from "@lucide/svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import FileGlyph from "$lib/components/FileGlyph.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type SpaceNode, type SpaceResult } from "$lib/api";
  import { squarify } from "$lib/treemap";
  import { age, bytes, plural } from "$lib/format";
  import { extStyle } from "$lib/kinds";

  let path = $state("");
  let result = $state<SpaceResult | null>(null);
  let loading = $state(false);
  let error = $state("");
  let hovered = $state<SpaceNode | null>(null);
  let picked = $state<SpaceNode | null>(null);
  let confirm = $state(false);
  let w = $state(0);
  let h = $state(0);
  const cache = new Map<string, SpaceResult>();

  onMount(() => {
    go(app.home || "C:\\");
  });

  async function go(p: string, fresh = false) {
    path = p;
    picked = null;
    confirm = false;
    error = "";
    if (!fresh && cache.has(p)) {
      result = cache.get(p)!;
      return;
    }
    loading = true;
    try {
      const r = await api.space(p);
      cache.set(p, r);
      if (path === p) result = r;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const crumbs = $derived.by(() => {
    const parts = path.replace(/\\$/, "").split("\\");
    return parts.map((name, i) => ({ name: i === 0 ? name + "\\" : name, path: parts.slice(0, i + 1).join("\\") + (i === 0 ? "\\" : "") }));
  });
  const parentPath = $derived(crumbs.length > 1 ? crumbs[crumbs.length - 2].path : null);

  const tiles = $derived.by(() => {
    if (!result || !w || !h) return [];
    const kids = result.children.filter((c) => c.bytes > 0);
    const total = result.bytes || 1;
    // Fold tiny items into one "other" tile so the map stays readable.
    const big = kids.filter((c) => c.bytes / total >= 0.004).slice(0, 120);
    const rest = kids.slice(big.length);
    const restBytes = rest.reduce((a, c) => a + c.bytes, 0);
    const items: SpaceNode[] = restBytes
      ? [...big, { name: `${rest.length} smaller items`, path: "", bytes: restBytes, files: rest.reduce((a, c) => a + c.files, 0), isDir: false, ext: "", newest: 0 }]
      : big;
    return squarify(items, (n) => n.bytes, { x: 0, y: 0, w, h });
  });

  function colorOf(n: SpaceNode): string {
    if (!n.path) return "#6f6b85";
    if (n.isDir) {
      let hsh = 0;
      for (const ch of n.name) hsh = (hsh * 31 + ch.charCodeAt(0)) | 0;
      const hues = [250, 265, 280, 220, 200, 170, 300, 235];
      return `hsl(${hues[Math.abs(hsh) % hues.length]} 70% 68%)`;
    }
    return extStyle(n.ext, false).color;
  }

  function click(n: SpaceNode) {
    if (!n.path) return;
    if (n.isDir) go(n.path);
    else {
      picked = n;
      confirm = false;
    }
  }

  async function recycle(n: SpaceNode) {
    if (!confirm) {
      confirm = true;
      return;
    }
    try {
      const id = await api.recyclePath(n.path, n.bytes);
      cache.clear();
      app.toast(`Recycled “${n.name}” · ${bytes(n.bytes)} freed`, {
        tone: "ok",
        undo: async () => {
          await api.undo(id);
          cache.clear();
          go(path, true);
          app.toast("Restored.", { tone: "ok" });
        },
      });
      picked = null;
      go(path, true);
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    }
  }

  const info = $derived(hovered ?? picked);
</script>

<div class="space">
  <div class="top">
    <div class="roots">
      <button class="chip r" class:on={path === app.home} onclick={() => go(app.home)}><House size={13} /> Home</button>
      {#each app.drives as d (d.mount)}
        <button class="chip r" class:on={path === d.mount} onclick={() => go(d.mount)}><HardDrive size={13} /> {d.mount.replace(/\\$/, "")}</button>
      {/each}
    </div>
    <nav class="crumbs">
      <button class="up" disabled={!parentPath} onclick={() => parentPath && go(parentPath)} title="Up one level"><ArrowUp size={15} /></button>
      {#each crumbs as c, i (c.path)}
        {#if i}<ChevronRight size={14} class="faint" />{/if}
        <button class="crumb" class:last={i === crumbs.length - 1} onclick={() => go(c.path)}>{c.name}</button>
      {/each}
    </nav>
  </div>

  <div class="body">
    <div class="map" bind:clientWidth={w} bind:clientHeight={h}>
      {#if loading && !result}
        <div class="loading" in:fade><Mascot mood="sweeping" size={130} /><p class="muted">Measuring every folder…</p></div>
      {:else if error}
        <div class="loading"><p class="muted">{error}</p></div>
      {:else}
        {#key result?.path}
          <div class="tiles" in:fade={{ duration: 250 }} class:dim={loading}>
            {#each tiles as t, i (t.item.path || "rest")}
              {@const c = colorOf(t.item)}
              <button
                class="tile"
                class:dir={t.item.isDir}
                class:sel={picked?.path === t.item.path}
                style="left:{t.x}px;top:{t.y}px;width:{t.w}px;height:{t.h}px;--c:{c};animation-delay:{Math.min(i * 12, 400)}ms"
                onmouseenter={() => (hovered = t.item)}
                onmouseleave={() => (hovered = null)}
                onclick={() => click(t.item)}
                ondblclick={() => t.item.path && !t.item.isDir && api.reveal(t.item.path)}
              >
                {#if t.w > 70 && t.h > 34}
                  <span class="tname ellipsis">{t.item.name}</span>
                  <span class="tsize">{bytes(t.item.bytes)}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/key}
      {/if}
    </div>

    <aside class="side">
      {#if result}
        <div class="total card">
          <span class="faint">This folder</span>
          <b class="big">{bytes(result.bytes)}</b>
          <span class="muted">{plural(result.files, "file")} · measured in {(result.ms / 1000).toFixed(1)}s</span>
        </div>
      {/if}

      {#if info}
        <div class="info card" in:fly={{ y: 8, duration: 200, easing: cubicOut }}>
          <div class="info-head">
            <FileGlyph ext={info.ext} isDir={info.isDir} size={40} />
            <div class="ellipsis">
              <b class="ellipsis" title={info.name}>{info.name}</b>
              <span class="muted">{bytes(info.bytes)}{info.isDir ? ` · ${plural(info.files, "file")}` : ""}</span>
            </div>
          </div>
          {#if info.newest}<span class="faint small">Last changed {age(info.newest)} ago</span>{/if}
          {#if info.path && picked?.path === info.path}
            <div class="info-acts">
              <button class="btn sm" onclick={() => api.reveal(info.path)}><ExternalLink size={13} /> Show</button>
              {#if info.isDir}<button class="btn sm" onclick={() => go(info.path)}><FolderOpen size={13} /> Open</button>{/if}
              <button class="btn sm danger" onclick={() => recycle(info)}><Trash2 size={13} /> {confirm ? "Sure?" : "Recycle"}</button>
            </div>
          {:else if info.path}
            <span class="faint small">{info.isDir ? "Click to dive in." : "Click to select."}</span>
          {/if}
        </div>
      {/if}

      {#if result}
        <div class="biggest scroll">
          {#each result.children.slice(0, 40) as n (n.path)}
            {@const pct = result.bytes ? n.bytes / result.bytes : 0}
            <button class="brow" class:sel={picked?.path === n.path} onclick={() => (n.isDir ? go(n.path) : (picked = n))} onmouseenter={() => (hovered = n)} onmouseleave={() => (hovered = null)}>
              <span class="bname ellipsis">{n.name}</span>
              <span class="bsize tabular">{bytes(n.bytes)}</span>
              <span class="bbar"><i style="width:{pct * 100}%; background:{colorOf(n)}"></i></span>
            </button>
          {/each}
        </div>
      {/if}
    </aside>
  </div>
</div>

<style>
  .space {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 22px 32px 20px;
    gap: 14px;
  }
  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-right: 120px;
  }
  .roots {
    display: flex;
    gap: 6px;
  }
  .r.on {
    background: var(--accent-soft);
    color: var(--text);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .crumb {
    padding: 3px 7px;
    border-radius: 7px;
    color: var(--text-2);
  }
  .crumb:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .crumb.last {
    color: var(--text);
    font-weight: 600;
  }
  .up {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    margin-right: 4px;
  }
  .up:disabled {
    opacity: 0.35;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 16px;
  }
  .map {
    position: relative;
    min-height: 0;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .tiles {
    position: absolute;
    inset: 0;
    transition: opacity 0.2s;
  }
  .tiles.dim {
    opacity: 0.5;
  }
  .tile {
    position: absolute;
    padding: 2px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    text-align: left;
    animation: grow 0.5s var(--ease) both;
  }
  .tile::before {
    content: "";
    position: absolute;
    inset: 2px;
    border-radius: 9px;
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 34%, transparent), color-mix(in srgb, var(--c) 16%, transparent));
    border: 1px solid color-mix(in srgb, var(--c) 35%, transparent);
    transition:
      background 0.2s,
      border-color 0.2s,
      transform 0.2s var(--ease);
  }
  .tile:hover::before {
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 55%, transparent), color-mix(in srgb, var(--c) 28%, transparent));
    border-color: var(--c);
  }
  .tile.sel::before {
    border: 2px solid var(--text);
  }
  .tname,
  .tsize {
    position: relative;
    max-width: 100%;
    padding: 0 9px;
    font-size: 12px;
  }
  .tname {
    font-weight: 600;
  }
  .tsize {
    padding-bottom: 7px;
    color: var(--text-2);
    font-size: 11px;
  }
  @keyframes grow {
    from {
      opacity: 0;
      transform: scale(0.94);
    }
  }
  .loading {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }
  .total,
  .info {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .big {
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 600;
  }
  .info-head {
    display: flex;
    gap: 12px;
    align-items: center;
    min-width: 0;
  }
  .info-head > div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .info-acts {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .small {
    font-size: 12px;
    margin-top: 4px;
  }
  .biggest {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .brow {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 10px;
    padding: 7px 10px;
    border-radius: 10px;
    text-align: left;
  }
  .brow:hover,
  .brow.sel {
    background: var(--surface-2);
  }
  .bname {
    font-size: 13px;
  }
  .bsize {
    font-size: 12px;
    color: var(--text-2);
  }
  .bbar {
    grid-column: 1 / -1;
    height: 3px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bbar i {
    display: block;
    height: 100%;
    border-radius: 3px;
  }
</style>
