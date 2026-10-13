<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import {
    ChevronRight,
    House,
    HardDrive,
    ExternalLink,
    Trash2,
    FolderOpen,
    ArrowUp,
    ZoomIn,
    Copy,
    Flame,
    Shield,
    Zap,
    Info,
    RefreshCw,
  } from "@lucide/svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type KbHit, type TreeNode } from "$lib/api";
  import { squarify } from "$lib/treemap";
  import { age, bytes, plural, shortPath } from "$lib/format";
  import { extStyle } from "$lib/kinds";
  import { menu } from "$lib/menu.svelte";

  let path = $state("");
  let tree = $state<TreeNode | null>(null);
  let loading = $state(false);
  let error = $state("");
  let hovered = $state<TreeNode | null>(null);
  let picked = $state<TreeNode | null>(null);
  let w = $state(0);
  let h = $state(0);
  let kb = $state<Map<string, KbHit>>(new Map());
  let ms = $state(0);
  const cache = new Map<string, TreeNode>();

  onMount(() => {
    const start =
      app.drive !== "all" ? `${app.drive}\\` : app.home || "C:\\";
    go(start);
  });

  async function go(p: string, fresh = false) {
    path = p;
    picked = null;
    error = "";
    if (!fresh && cache.has(p)) {
      tree = cache.get(p)!;
      return;
    }
    loading = true;
    const t = performance.now();
    try {
      const r = await api.spaceTree(p, 3);
      ms = Math.round(performance.now() - t);
      cache.set(p, r);
      if (path === p) tree = r;
      lookupKb(r);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function lookupKb(r: TreeNode) {
    const paths: string[] = [];
    const walk = (n: TreeNode, d: number) => {
      if (n.path && n.isDir) paths.push(n.path);
      if (d < 2) n.children.forEach((c) => walk(c, d + 1));
    };
    walk(r, 0);
    const hits = await api.kbLookup(paths);
    const m = new Map(kb);
    paths.forEach((p, i) => {
      const hit = hits[i];
      // Only label the folder the entry is actually about, not everything inside it.
      if (hit && hit.path.toLowerCase() === p.toLowerCase()) m.set(p.toLowerCase(), hit);
    });
    kb = m;
  }

  const crumbs = $derived.by(() => {
    const parts = path.replace(/\\$/, "").split("\\");
    return parts.map((name, i) => ({ name: i === 0 ? name + "\\" : name, path: parts.slice(0, i + 1).join("\\") + (i === 0 ? "\\" : "") }));
  });
  const parentPath = $derived(crumbs.length > 1 ? crumbs[crumbs.length - 2].path : null);

  interface Tile {
    n: TreeNode;
    x: number;
    y: number;
    w: number;
    h: number;
    depth: number;
    header: boolean;
  }
  const HEADER = 20;

  const MAX_TILES = 900;
  function layout(node: TreeNode, box: { x: number; y: number; w: number; h: number }, depth: number, out: Tile[]) {
    const items = node.children.filter((c) => c.bytes > 0);
    for (const r of squarify(items, (c) => c.bytes, box)) {
      // Boxes too small to see or click only cost rendering time.
      if (out.length >= MAX_TILES || (depth > 0 && r.w * r.h < 220)) continue;
      const nested = r.item.isDir && r.item.children.length > 0 && r.w > 90 && r.h > 64 && depth < 3;
      out.push({ n: r.item, x: r.x, y: r.y, w: r.w, h: r.h, depth, header: nested });
      if (nested) {
        layout(r.item, { x: r.x + 3, y: r.y + HEADER, w: r.w - 6, h: r.h - HEADER - 3 }, depth + 1, out);
      }
    }
    return out;
  }
  const tiles = $derived(tree && w && h ? layout(tree, { x: 0, y: 0, w, h }, 0, []) : []);

  function colorOf(n: TreeNode): string {
    if (!n.path) return "#6f6b85";
    const k = kb.get(n.path.toLowerCase());
    if (k) return k.safety === "safe" ? "#9ece6a" : k.safety === "caution" ? "#e0af68" : "#f7768e";
    if (n.isDir) {
      let hsh = 0;
      for (const ch of n.name) hsh = (hsh * 31 + ch.charCodeAt(0)) | 0;
      const hues = [250, 265, 280, 220, 205, 190, 300, 235];
      return `hsl(${hues[Math.abs(hsh) % hues.length]} 65% 66%)`;
    }
    return extStyle(n.name.split(".").pop() ?? "", false).color;
  }
  const extOf = (n: TreeNode) => (n.isDir ? "" : (n.name.includes(".") ? n.name.split(".").pop()! : ""));

  function click(n: TreeNode) {
    if (!n.path) return;
    if (n.isDir) go(n.path);
    else picked = n;
  }

  async function recycle(n: TreeNode) {
    try {
      const id = await api.recyclePath(n.path, n.bytes);
      cache.clear();
      app.refreshDisks();
      app.toast(`Recycled “${n.name}” · ${bytes(n.bytes)} (space returns once the bin is emptied)`, {
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

  async function deleteForGood(n: TreeNode) {
    try {
      await api.deletePath(n.path, n.bytes);
      cache.clear();
      app.refreshDisks();
      app.toast(`Deleted “${n.name}” for good · ${bytes(n.bytes)} freed`, { tone: "ok" });
      picked = null;
      go(path, true);
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    }
  }

  function contextMenu(e: MouseEvent, n: TreeNode) {
    if (!n.path) return;
    const k = kb.get(n.path.toLowerCase());
    let armed = false;
    menu.show(e, [
      ...(k ? [{ caption: `${k.app}: ${k.safety === "safe" ? "usually safe to clear" : k.safety === "caution" ? "be careful" : "leave alone"}` }] : []),
      ...(n.isDir ? [{ label: "Zoom in", icon: ZoomIn, run: () => go(n.path) }] : [{ label: "Open", icon: ExternalLink, run: () => api.openFile(n.path) }]),
      { label: n.isDir ? "Open in Explorer" : "Show in Explorer", icon: FolderOpen, run: () => (n.isDir ? api.openPath(n.path) : api.reveal(n.path)) },
      { label: "Copy path", icon: Copy, run: () => navigator.clipboard.writeText(n.path) },
      { divider: true },
      { label: "Recycle", icon: Trash2, danger: true, disabled: k?.safety === "never", run: () => recycle(n) },
      {
        label: "Delete for good…",
        icon: Flame,
        danger: true,
        disabled: k?.safety === "never",
        hint: "Permanent, frees space immediately",
        run: () => {
          if (!armed && !confirm(`Delete “${n.name}” (${bytes(n.bytes)}) permanently? This can't be undone.`)) return;
          armed = true;
          deleteForGood(n);
        },
      },
      { divider: true },
      { label: "Don't flag this for a month", icon: Shield, run: () => app.protect(n.path, 30, `“${n.name}”`) },
      { label: "It's important, never flag it", icon: Shield, run: () => app.protect(n.path, 0, `“${n.name}”`) },
    ]);
  }

  const info = $derived(hovered ?? picked);
  const infoKb = $derived(info?.path ? kb.get(info.path.toLowerCase()) : undefined);
</script>

<div class="space">
  <div class="top">
    <div class="roots">
      <button class="chip r" class:on={path === app.home} onclick={() => go(app.home)}><House size={13} /> Home</button>
      {#each app.drives as d (d.mount)}
        <button class="chip r" class:on={path === d.mount} onclick={() => go(d.mount)}><HardDrive size={13} /> {d.mount.replace(/\\$/, "")} <i>{bytes(d.total - d.free)}</i></button>
      {/each}
      <span class="sp"></span>
      <span class="badge" class:live={!app.index?.ready}>
        {#if app.index?.ready}<Zap size={12} /> Instant (indexed) · {ms} ms{:else}Measuring live{/if}
      </span>
      <button class="chip r" title="Re-measure" onclick={() => { cache.clear(); go(path, true); }}><RefreshCw size={13} /></button>
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
      {#if loading && !tree}
        <div class="loading" in:fade><Mascot mood="sweeping" size={130} /><p class="muted">Measuring every folder…</p></div>
      {:else if error}
        <div class="loading"><p class="muted">{error}</p></div>
      {:else}
        {#key tree?.path}
          <div class="tiles" in:fade={{ duration: 220 }} class:dim={loading}>
            {#each tiles as t, i (t.n.path || `rest-${t.depth}-${i}`)}
              {@const c = colorOf(t.n)}
              {@const k = t.n.path ? kb.get(t.n.path.toLowerCase()) : undefined}
              <button
                class="tile d{t.depth}"
                class:dir={t.header}
                class:sel={picked?.path === t.n.path}
                style="left:{t.x}px;top:{t.y}px;width:{t.w}px;height:{t.h}px;--c:{c};z-index:{t.depth + 1};animation-delay:{Math.min(i * 6, 300)}ms"
                onmouseenter={() => (hovered = t.n)}
                onmouseleave={() => (hovered = null)}
                onclick={(e) => { e.stopPropagation(); click(t.n); }}
                oncontextmenu={(e) => { e.stopPropagation(); contextMenu(e, t.n); }}
              >
                {#if t.header}
                  <span class="th ellipsis">{t.n.name} <i>{bytes(t.n.bytes)}</i>{#if k}<em class={k.safety}>{k.app}</em>{/if}</span>
                {:else if t.w > 64 && t.h > 32}
                  <span class="tname ellipsis">{t.n.name}</span>
                  <span class="tsize">{bytes(t.n.bytes)}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/key}
      {/if}
    </div>

    <aside class="side">
      <div class="total card">
        <span class="faint ellipsis">{shortPath(path, app.home)}</span>
        <b class="big">{tree ? bytes(tree.bytes) : "…"}</b>
        <span class="muted small">Right-click any box for options. Click a folder to zoom in.</span>
      </div>

      <!-- Fixed-height slot: hovering never shifts the layout. -->
      <div class="info card">
        {#if info}
          <div class="info-head">
            <Preview path={info.path} ext={extOf(info)} isDir={info.isDir} size={44} />
            <div class="names">
              <b class="ellipsis" title={info.name}>{info.name}</b>
              <span class="muted">{bytes(info.bytes)}{info.newest ? ` · changed ${age(info.newest)} ago` : ""}</span>
            </div>
          </div>
          {#if infoKb}
            <p class="kb {infoKb.safety}"><Info size={13} /> <span><b>{infoKb.app}</b>: {infoKb.note}</span></p>
          {:else if info.path}
            <p class="faint small">{info.isDir ? "Click to zoom in, right-click for options." : "Right-click for options."}</p>
          {/if}
          {#if picked && info.path === picked.path}
            <div class="info-acts">
              <button class="btn sm" onclick={() => api.openFile(info.path)}><ExternalLink size={13} /> Open</button>
              <button class="btn sm" onclick={() => api.reveal(info.path)}><FolderOpen size={13} /> Show</button>
              <button class="btn sm danger" onclick={() => recycle(info)}><Trash2 size={13} /> Recycle</button>
            </div>
          {/if}
        {:else}
          <p class="faint small placeholder">Hover a box to see what it is. Green boxes are caches Dusty knows are safe to clear.</p>
        {/if}
      </div>

      {#if tree}
        <div class="biggest scroll">
          {#each tree.children.slice(0, 60) as n, i (n.path || `rest-${i}`)}
            {@const pct = tree.bytes ? n.bytes / tree.bytes : 0}
            <button
              class="brow"
              class:sel={picked?.path === n.path}
              onclick={() => click(n)}
              oncontextmenu={(e) => contextMenu(e, n)}
              onmouseenter={() => (hovered = n)}
              onmouseleave={() => (hovered = null)}
            >
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
    padding: 42px 32px 20px;
    gap: 12px;
  }
  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .roots {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .roots i {
    font-style: normal;
    color: var(--text-3);
    font-size: 11.5px;
  }
  .sp {
    flex: 1;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--ok);
  }
  .badge.live {
    color: var(--text-3);
  }
  .r {
    cursor: pointer;
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
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    text-align: left;
    border-radius: 7px;
    overflow: hidden;
    animation: grow 0.35s var(--ease) both;
    contain: layout paint;
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 32%, transparent), color-mix(in srgb, var(--c) 14%, transparent));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 32%, transparent);
    transition:
      background 0.15s,
      box-shadow 0.15s;
  }
  .tile.dir {
    justify-content: flex-start;
    background: color-mix(in srgb, var(--c) 10%, transparent);
  }
  .tile:not(.dir):hover {
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 55%, transparent), color-mix(in srgb, var(--c) 26%, transparent));
    box-shadow: inset 0 0 0 1.5px var(--c);
  }
  .tile.dir:hover {
    box-shadow: inset 0 0 0 1.5px var(--c);
  }
  .tile.sel {
    box-shadow: inset 0 0 0 2px var(--text);
  }
  .th {
    width: 100%;
    height: 20px;
    padding: 2px 7px 0;
    font-size: 11.5px;
    font-weight: 600;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .th i {
    font-style: normal;
    font-weight: 400;
    color: var(--text-2);
  }
  .th em {
    font-style: normal;
    font-size: 10px;
    padding: 0 6px;
    border-radius: 99px;
    background: rgba(0, 0, 0, 0.25);
  }
  .th em.safe {
    color: #9ece6a;
  }
  .th em.caution {
    color: #e0af68;
  }
  .th em.never {
    color: #f7768e;
  }
  .tname,
  .tsize {
    position: relative;
    max-width: 100%;
    padding: 0 8px;
    font-size: 11.5px;
  }
  .tname {
    font-weight: 600;
  }
  .tsize {
    padding-bottom: 6px;
    color: var(--text-2);
    font-size: 11px;
  }
  @keyframes grow {
    from {
      opacity: 0;
      transform: scale(0.96);
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
  .total {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .big {
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 600;
  }
  .info {
    height: 168px;
    flex: none;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .placeholder {
    margin: auto 0;
  }
  .info-head {
    display: flex;
    gap: 12px;
    align-items: center;
    min-width: 0;
  }
  .names {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .kb {
    margin: 2px 0 0;
    display: flex;
    gap: 6px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-2);
  }
  .kb :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .kb.safe b {
    color: #9ece6a;
  }
  .kb.caution b {
    color: #e0af68;
  }
  .kb.never b {
    color: #f7768e;
  }
  .info-acts {
    display: flex;
    gap: 6px;
    margin-top: auto;
    flex-wrap: wrap;
  }
  .small {
    font-size: 12px;
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
    flex: none;
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
