<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Search, CornerDownLeft, FolderOpen, Copy, Sparkles, Database } from "@lucide/svelte";
  import FileGlyph from "$lib/components/FileGlyph.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import { api, type SearchResult } from "$lib/api";
  import { age, bytes } from "$lib/format";

  let q = $state("");
  let res = $state<SearchResult | null>(null);
  let sel = $state(0);
  let input = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout>;
  let seq = 0;
  let home = $state("");

  const EXAMPLES = [
    "invoice pdf from about 5 months ago",
    "big videos on d:",
    "screenshots last week",
    "setup exe in downloads",
    "budget folder in documents",
    "\"exact name\" bigger than 100 mb",
  ];
  const example = EXAMPLES[Math.floor(Math.random() * EXAMPLES.length)];

  onMount(async () => {
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      (await import("$lib/mock")).installMocks();
      q = "invoice pdf about 5 months ago";
    }
    home = await api.homeDir().catch(() => "");
    focus();
    await listen("search-open", () => {
      q = "";
      res = null;
      sel = 0;
      focus();
    });
  });

  async function focus() {
    await tick();
    input?.focus();
    input?.select();
  }

  $effect(() => {
    const query = q;
    clearTimeout(timer);
    if (!query.trim()) {
      res = null;
      return;
    }
    timer = setTimeout(async () => {
      const my = ++seq;
      const r = await api.search(query, 50);
      if (my === seq) {
        res = r;
        sel = 0;
      }
    }, 70);
  });

  const hits = $derived(res?.hits ?? []);
  const current = $derived(hits[sel]);

  function hide() {
    api.hideSearch();
  }
  async function open(i = sel) {
    const h = hits[i];
    if (!h) return;
    await api.openFile(h.path);
    hide();
  }
  async function reveal(i = sel) {
    const h = hits[i];
    if (!h) return;
    await api.reveal(h.path);
    hide();
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      hide();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(hits.length - 1, sel + 1);
      scrollSel();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(0, sel - 1);
      scrollSel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (e.ctrlKey || e.altKey) reveal();
      else open();
    } else if (e.key.toLowerCase() === "c" && e.ctrlKey && current && !window.getSelection()?.toString()) {
      navigator.clipboard.writeText(current.path);
    }
  }
  function scrollSel() {
    tick().then(() => document.querySelector(".hit.sel")?.scrollIntoView({ block: "nearest" }));
  }
  const ext = (n: string) => (n.includes(".") ? n.split(".").pop()! : "");
  const short = (p: string) => (home && p.toLowerCase().startsWith(home.toLowerCase()) ? "~" + p.slice(home.length) : p);
  const dir = (p: string) => short(p.slice(0, p.lastIndexOf("\\")));
</script>

<svelte:window onkeydown={key} />

<div class="wrap" data-tauri-drag-region>
  <div class="panel" in:fly={{ y: -10, duration: 220, easing: cubicOut }}>
    <div class="bar">
      <Search size={20} class="faint" />
      <input bind:this={input} bind:value={q} placeholder={`Search everything… try “${example}”`} spellcheck="false" autocomplete="off" />
      {#if res}<span class="ms faint">{res.hits.length} · {res.ms} ms</span>{/if}
    </div>

    {#if res && res.query.explain.length}
      <div class="chips">
        <Sparkles size={13} class="accent" />
        {#each res.query.explain as c}<span class="chip">{c}</span>{/each}
      </div>
    {/if}

    {#if res && !res.indexed}
      <p class="empty faint"><Database size={14} /> Dusty is still indexing your drives. Results will appear in a moment.</p>
    {:else if res && !hits.length}
      <p class="empty faint">Nothing matches. Try fewer words, or say roughly when (“last month”) and what (“pdf”, “video”).</p>
    {:else if hits.length}
      <div class="results">
        <div class="list">
          {#each hits as h, i (h.path)}
            <button class="hit" class:sel={i === sel} onmouseenter={() => (sel = i)} onclick={() => open(i)} ondblclick={() => open(i)}>
              <FileGlyph ext={ext(h.name)} isDir={h.isDir} size={30} />
              <div class="hn">
                <b class="ellipsis">{h.name}</b>
                <span class="faint ellipsis">{dir(h.path)}</span>
              </div>
              <span class="hm faint">{h.isDir ? "" : bytes(h.size)}</span>
            </button>
          {/each}
        </div>
        {#if current}
          <aside class="pv">
            <div class="pvbox"><Preview path={current.path} ext={ext(current.name)} isDir={current.isDir} rich /></div>
            <b class="ellipsis">{current.name}</b>
            <span class="faint small">{current.isDir ? "Folder" : bytes(current.size)} · changed {age(current.mtime)} ago</span>
            <span class="faint small mono path">{short(current.path)}</span>
          </aside>
        {/if}
      </div>
    {:else}
      <p class="empty faint">Type a name, or ask the way you'd ask a friend: “that zip from github last spring”.</p>
    {/if}

    <footer class="faint">
      <span><span class="kbd"><CornerDownLeft size={11} /></span> open</span>
      <span><span class="kbd">Ctrl</span>+<span class="kbd"><CornerDownLeft size={11} /></span> <FolderOpen size={12} /> show in folder</span>
      <span><span class="kbd">Ctrl</span>+<span class="kbd">C</span> <Copy size={12} /> copy path</span>
      <span class="sp"></span>
      <span><span class="kbd">Esc</span> close</span>
    </footer>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  .wrap {
    position: fixed;
    inset: 0;
    padding: 10px;
    display: flex;
    align-items: flex-start;
  }
  .panel {
    width: 100%;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-solid) 97%, white 3%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 20px 50px -12px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 18px;
  }
  input {
    flex: 1;
    background: none;
    border: 0;
    outline: none;
    font-size: 19px;
    font-family: var(--font-display);
  }
  input::placeholder {
    color: var(--text-3);
  }
  .ms {
    font-size: 12px;
  }
  .chips {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 18px 10px;
    flex-wrap: wrap;
  }
  .chips :global(.accent) {
    color: var(--accent);
  }
  .chip {
    height: 22px;
    font-size: 12px;
  }
  .empty {
    margin: 0;
    padding: 18px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .results {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 230px;
    border-top: 1px solid var(--border);
    min-height: 0;
    height: 340px;
  }
  .list {
    overflow-y: auto;
    padding: 6px;
  }
  .hit {
    width: 100%;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 7px 10px;
    border-radius: 10px;
    text-align: left;
  }
  .hit.sel {
    background: var(--accent-soft);
  }
  .hn {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-size: 12px;
  }
  .hn b {
    font-size: 13.5px;
    font-weight: 600;
  }
  .hm {
    font-size: 12px;
  }
  .pv {
    border-left: 1px solid var(--border);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .pvbox {
    height: 170px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 8px;
  }
  .small {
    font-size: 11.5px;
  }
  .path {
    word-break: break-all;
    font-size: 10.5px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 9px 18px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
  }
  footer span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .sp {
    flex: 1;
  }
</style>
