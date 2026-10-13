<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import {
    Search,
    CornerDownLeft,
    FolderOpen,
    Copy,
    Sparkles,
    Database,
    Mic,
    Command,
    RefreshCw,
    Pause,
    Play,
    Trash2,
    House,
    Layers,
    ChartPie,
    Gauge,
    History,
    SlidersHorizontal,
    Wand,
  } from "@lucide/svelte";
  import FileGlyph from "$lib/components/FileGlyph.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import VoiceOrb from "$lib/components/VoiceOrb.svelte";
  import { api, type SearchResult, type Settings, type VoiceStatus } from "$lib/api";
  import { age, bytes } from "$lib/format";

  type Mode = "search" | "commands" | "voice";
  let mode = $state<Mode>("search");
  let q = $state("");
  let res = $state<SearchResult | null>(null);
  let sel = $state(0);
  let input = $state<HTMLInputElement>();
  let panel = $state<HTMLDivElement>();
  let home = $state("");
  let settings = $state<Settings | null>(null);
  let voice = $state<VoiceStatus | null>(null);
  let vstate = $state<"listening" | "thinking" | "idle">("idle");
  let level = $state(0);
  let heard = $state("");
  let voiceError = $state("");
  let flash = $state("");
  let timer: ReturnType<typeof setTimeout>;
  let seq = 0;

  const EXAMPLES = [
    "invoice pdf from about 5 months ago",
    "big videos on d:",
    "screenshots last week",
    "foto vacanze dell'anno scorso",
    "budget folder in documents",
    "\"exact name\" bigger than 100 mb",
  ];
  const example = EXAMPLES[Math.floor(Math.random() * EXAMPLES.length)];

  onMount(async () => {
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      (await import("$lib/mock")).installMocks();
      const m = new URLSearchParams(location.search).get("mode");
      if (m) mode = m as Mode;
      if (mode === "search") q = "invoice pdf about 5 months ago";
      if (mode === "voice") vstate = "listening";
    }
    home = await api.homeDir().catch(() => "");
    settings = await api.getSettings().catch(() => null);
    voice = await api.voiceStatus().catch(() => null);
    focus();
    await listen<Mode>("search-open", async (e) => {
      mode = (e.payload as Mode) || "search";
      q = "";
      res = null;
      sel = 0;
      heard = "";
      voiceError = "";
      flash = "";
      settings = await api.getSettings().catch(() => settings);
      voice = await api.voiceStatus().catch(() => voice);
      if (mode === "voice") vstate = "listening";
      focus();
    });
    await listen<number>("voice-level", (e) => (level = e.payload));
    await listen<string>("voice-state", (e) => (vstate = e.payload as typeof vstate));
    await listen<string>("voice-error", (e) => {
      voiceError = e.payload;
      vstate = "idle";
    });
    await listen<string>("voice-text", async (e) => {
      heard = e.payload;
      mode = "search";
      q = e.payload;
      vstate = "idle";
    });
  });

  // Keep the window exactly as tall as the panel (the blur stays behind the panel only).
  $effect(() => {
    if (!panel || !("__TAURI_INTERNALS__" in window)) return;
    const ro = new ResizeObserver(() => {
      const h = Math.ceil(panel!.getBoundingClientRect().height);
      getCurrentWindow().setSize(new LogicalSize(760, Math.max(80, h))).catch(() => {});
    });
    ro.observe(panel);
    return () => ro.disconnect();
  });

  async function focus() {
    await tick();
    input?.focus();
    input?.select();
  }

  $effect(() => {
    const query = q;
    if (mode !== "search") return;
    clearTimeout(timer);
    if (query.startsWith(">")) {
      mode = "commands";
      q = query.slice(1);
      return;
    }
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
        if (heard && settings?.voiceReply) speak(r);
      }
    }, 60);
  });

  /** Short spoken answer, local voices only (nothing leaves the PC). */
  function speak(r: SearchResult) {
    if (!("speechSynthesis" in window)) return;
    const n = r.hits.length;
    const what = r.query.explain.slice(1).join(", ") || "files";
    const text = n === 0 ? "I couldn't find anything like that." : n === 1 ? `I found one match: ${r.hits[0].name}.` : `I found ${n >= 50 ? "lots of" : n} ${what}. The top one is ${r.hits[0].name}.`;
    const u = new SpeechSynthesisUtterance(text);
    const local = speechSynthesis.getVoices().filter((v) => v.localService);
    u.voice = local.find((v) => v.lang.startsWith("en")) ?? local[0] ?? null;
    u.rate = 1.05;
    speechSynthesis.cancel();
    speechSynthesis.speak(u);
  }

  // ---- commands ---------------------------------------------------------
  interface Cmd {
    id: string;
    label: string;
    hint?: string;
    icon: typeof Search;
    run: () => Promise<unknown> | unknown;
  }
  const commands = $derived.by((): Cmd[] => {
    const all: Cmd[] = [
      { id: "clean", label: "Clean the sure ones", hint: "things I'm 85%+ sure about", icon: Wand, run: () => cmd("clean_sure") },
      { id: "scan", label: "Look around now", icon: RefreshCw, run: () => cmd("scan") },
      { id: "voice", label: "Ask by voice", hint: settings?.voiceHotkey, icon: Mic, run: () => api.openBar("voice") },
      { id: "search", label: "Search files", hint: settings?.searchHotkey, icon: Search, run: () => ((mode = "search"), (q = ""), focus()) },
      { id: "pause", label: "Pause for 3 hours", icon: Pause, run: () => cmd("pause") },
      { id: "resume", label: "Wake up", icon: Play, run: () => cmd("resume") },
      { id: "bin", label: "Empty the Recycle Bin", hint: "permanent", icon: Trash2, run: () => cmd("empty_bin") },
      { id: "home", label: "Open Dusty", icon: House, run: () => cmd("view:home") },
      { id: "tidy", label: "Start tidying", icon: Layers, run: () => cmd("view:triage") },
      { id: "space", label: "Where did my space go?", icon: ChartPie, run: () => cmd("view:space") },
      { id: "speed", label: "Speed up my PC", icon: Gauge, run: () => cmd("view:speed") },
      { id: "history", label: "History & undo", icon: History, run: () => cmd("view:history") },
      { id: "settings", label: "Settings", icon: SlidersHorizontal, run: () => cmd("view:rules") },
    ];
    const n = q.trim().toLowerCase();
    return n ? all.filter((c) => (c.label + " " + (c.hint ?? "")).toLowerCase().includes(n)) : all;
  });
  async function cmd(c: string) {
    try {
      const msg = await api.barCommand(c);
      if (msg) {
        flash = msg;
        setTimeout(hide, 1100);
      }
    } catch (e) {
      flash = String(e);
    }
  }

  const hits = $derived(res?.hits ?? []);
  const current = $derived(hits[sel]);

  function hide() {
    if ("speechSynthesis" in window) speechSynthesis.cancel();
    api.voiceStop().catch(() => {});
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
      if (mode === "commands" && q) q = "";
      else hide();
      return;
    }
    const n = mode === "commands" ? commands.length : hits.length;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(n - 1, sel + 1);
      scrollSel();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(0, sel - 1);
      scrollSel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (mode === "commands") commands[sel]?.run();
      else if (e.ctrlKey || e.altKey) reveal();
      else open();
    } else if (e.key === "Backspace" && mode === "commands" && !q) {
      mode = "search";
    } else if (e.key.toLowerCase() === "c" && e.ctrlKey && current && !window.getSelection()?.toString()) {
      navigator.clipboard.writeText(current.path);
    }
  }
  function scrollSel() {
    tick().then(() => document.querySelector(".row.sel")?.scrollIntoView({ block: "nearest" }));
  }
  function startVoice() {
    mode = "voice";
    heard = "";
    voiceError = "";
    vstate = "listening";
    api.voiceListen().catch((e) => (voiceError = String(e)));
  }
  $effect(() => {
    mode;
    sel = 0;
  });
  const ext = (n: string) => (n.includes(".") ? n.split(".").pop()! : "");
  const short = (p: string) => (home && p.toLowerCase().startsWith(home.toLowerCase()) ? "~" + p.slice(home.length) : p);
  const dir = (p: string) => short(p.slice(0, p.lastIndexOf("\\")));
  const voiceReady = $derived(!!settings?.voiceEnabled && !!voice?.installed);
</script>

<svelte:window onkeydown={key} />

<div class="panel" bind:this={panel} in:fly={{ y: -8, duration: 200, easing: cubicOut }}>
  {#if mode === "voice"}
    <div class="voice">
      {#if voiceReady}
        <VoiceOrb {level} state={vstate} height={120} />
        <p class="vstate" in:fade>
          {#if voiceError}{voiceError}
          {:else if vstate === "thinking"}Got it, one sec…
          {:else if vstate === "listening"}I'm listening. Ask for any file, like “the invoice PDF from March”.
          {:else}Press <span class="kbd">{settings?.voiceHotkey}</span> to ask again{/if}
        </p>
        {#if voiceError}<button class="btn sm" onclick={startVoice}><Mic size={13} /> Try again</button>{/if}
      {:else}
        <div class="vsetup">
          <Mic size={22} />
          <div>
            <b>Voice search is off</b>
            <span class="faint">It runs entirely on your PC with a small speech model (download once). Nothing you say leaves this computer.</span>
          </div>
          <button class="btn sm primary" onclick={() => cmd("view:rules")}>Set it up</button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="bar" data-tauri-drag-region>
      {#if mode === "commands"}<Command size={19} class="accent" />{:else}<Search size={20} class="faint" />{/if}
      <input
        bind:this={input}
        bind:value={q}
        placeholder={mode === "commands" ? "What should Dusty do?" : `Search everything… try “${example}”`}
        spellcheck="false"
        autocomplete="off"
      />
      {#if res && mode === "search"}<span class="ms faint">{res.hits.length} · {res.ms} ms</span>{/if}
      {#if mode === "search"}
        <button class="mic" title="Ask by voice ({settings?.voiceHotkey})" onclick={startVoice}><Mic size={17} /></button>
      {/if}
    </div>

    {#if flash}
      <p class="flash" in:fly={{ y: 4, duration: 200 }}><Sparkles size={14} /> {flash}</p>
    {:else if mode === "commands"}
      <div class="cmds">
        {#each commands as c, i (c.id)}
          {@const I = c.icon}
          <button class="row cmd" class:sel={i === sel} onmouseenter={() => (sel = i)} onclick={() => c.run()}>
            <span class="ci"><I size={16} /></span>
            <span class="ellipsis">{c.label}</span>
            {#if c.hint}<span class="hint faint">{c.hint}</span>{/if}
          </button>
        {/each}
      </div>
    {:else}
      {#if heard}
        <div class="chips"><Mic size={13} class="accent" /><span class="heard">“{heard}”</span></div>
      {/if}
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
              <button class="row" class:sel={i === sel} onmouseenter={() => (sel = i)} onclick={() => open(i)}>
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
      {/if}
    {/if}

    <footer class="faint">
      {#if mode === "commands"}
        <span><span class="kbd"><CornerDownLeft size={11} /></span> run</span>
        <span><span class="kbd">⌫</span> back to search</span>
      {:else}
        <span><span class="kbd"><CornerDownLeft size={11} /></span> open</span>
        <span><span class="kbd">Ctrl</span>+<span class="kbd"><CornerDownLeft size={11} /></span> <FolderOpen size={12} /> show</span>
        <span><span class="kbd">Ctrl</span>+<span class="kbd">C</span> <Copy size={12} /> copy path</span>
        <span><span class="kbd">&gt;</span> commands</span>
      {/if}
      <span class="sp"></span>
      <span class="private">100% on this PC</span>
      <span><span class="kbd">Esc</span></span>
    </footer>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
  }
  /* Acrylic shows through: the panel is only lightly tinted. */
  .panel {
    position: fixed;
    inset: 0 0 auto 0;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--bg-solid) 62%, transparent);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 15px 14px 15px 18px;
  }
  .bar :global(.accent),
  .chips :global(.accent) {
    color: var(--accent);
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
  .mic {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--text-2);
    transition: all 0.2s;
  }
  .mic:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .chips {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 18px 10px;
    flex-wrap: wrap;
  }
  .heard {
    font-size: 13px;
    color: var(--text-2);
    font-style: italic;
  }
  .chip {
    height: 22px;
    font-size: 12px;
  }
  .empty,
  .flash {
    margin: 0;
    padding: 16px 18px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .flash {
    color: var(--accent);
    font-weight: 600;
  }
  .results {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 230px;
    border-top: 1px solid var(--border);
    height: 340px;
  }
  .list,
  .cmds {
    overflow-y: auto;
    padding: 6px;
  }
  .cmds {
    max-height: 380px;
    border-top: 1px solid var(--border);
  }
  .row {
    width: 100%;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 7px 10px;
    border-radius: 10px;
    text-align: left;
    transition: background 0.12s;
  }
  .row.sel {
    background: var(--accent-soft);
  }
  .cmd {
    padding: 9px 10px;
  }
  .ci {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--accent);
    background: var(--accent-soft);
  }
  .hint {
    font-size: 12px;
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
  .private {
    color: var(--ok);
    opacity: 0.8;
  }
  .voice {
    padding: 18px 18px 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .vstate {
    margin: 0;
    font-size: 14px;
    color: var(--text-2);
    text-align: center;
  }
  .vsetup {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 4px;
    color: var(--accent);
  }
  .vsetup > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
    color: var(--text);
  }
</style>
