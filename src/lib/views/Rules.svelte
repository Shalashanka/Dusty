<script lang="ts">
  import { onMount } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import {
    FolderPlus,
    X,
    Bot,
    Bell,
    Clock,
    Power,
    FolderOpen,
    Search,
    ShieldCheck,
    Coffee,
    Heart,
    Code,
    Database,
    Brain,
    HardDrive,
    Eye,
    ListChecks,
    Info,
    Bug,
    ExternalLink,
    Copy,
    Mic,
    Footprints,
    RefreshCw,
    Play,
  } from "@lucide/svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import Mascot from "$lib/components/Mascot.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type AppInfo, type Kind, type Mode, type PolicyMode, type Protected, type Settings, type TtsModel, type UpdateInfo, type VoiceStatus } from "$lib/api";
  import { listen } from "@tauri-apps/api/event";
  import { KINDS, KIND_ORDER } from "$lib/kinds";
  import { ago, shortPath } from "$lib/format";
  import { CONFIG } from "$lib/config";

  type Tab = "rules" | "folders" | "behavior" | "protected" | "about" | "dev";
  let tab = $state<Tab>("rules");
  let info = $state<AppInfo | null>(null);
  let protectedList = $state<Protected[]>([]);
  let taps = $state(0);
  let devOut = $state("");
  type HotkeyField = "searchHotkey" | "paletteHotkey" | "voiceHotkey";
  let recording = $state<HotkeyField | null>(null);
  let voice = $state<VoiceStatus | null>(null);
  let voiceProgress = $state<number | null>(null);
  let ttsModels = $state<TtsModel[]>([]);
  let ttsProgress = $state<{ id: string; p: number } | null>(null);
  let update = $state<UpdateInfo | null>(null);
  let updateError = $state("");
  let checking = $state(false);

  const s = $derived(app.settings!);
  let saveTimer: ReturnType<typeof setTimeout>;

  onMount(async () => {
    info = await api.appInfo();
    protectedList = await api.protectedList();
    voice = await api.voiceStatus().catch(() => null);
    await listen<number>("voice-download", (e) => (voiceProgress = e.payload));
    ttsModels = await api.ttsModels().catch(() => []);
    await listen<number>("tts-download", (e) => ttsProgress && (ttsProgress = { ...ttsProgress, p: e.payload }));
  });

  async function downloadTts(id: string) {
    ttsProgress = { id, p: 0 };
    try {
      await api.ttsDownload(id);
      patch({ ttsModel: id, voiceReply: true });
      tryVoice(id);
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    } finally {
      ttsProgress = null;
      ttsModels = await api.ttsModels().catch(() => ttsModels);
    }
  }
  function tryVoice(id = s.ttsModel) {
    if (id === "system") {
      const u = new SpeechSynthesisUtterance("Hi! I'm Dusty. I found three invoices from March.");
      u.voice = speechSynthesis.getVoices().find((v) => v.localService && v.lang.startsWith("en")) ?? null;
      speechSynthesis.speak(u);
    } else {
      setTimeout(() => api.say("Hi! I'm Dusty. I found three invoices from March. The top one is in Downloads.", "en"), 150);
    }
  }

  async function downloadVoice(model: string) {
    voiceProgress = 0;
    try {
      await api.voiceDownload(model);
      patch({ voiceModel: model, voiceEnabled: true });
      app.toast("Voice is ready. Press " + s.voiceHotkey + " anywhere and ask.", { tone: "ok" });
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    } finally {
      voiceProgress = null;
      voice = await api.voiceStatus().catch(() => voice);
    }
  }
  async function checkUpdate() {
    checking = true;
    updateError = "";
    try {
      update = await api.checkUpdate();
    } catch (e) {
      updateError = String(e);
    } finally {
      checking = false;
    }
  }
  async function pickUpdateFolder() {
    const p = await api.pickFolder();
    if (p) patch({ updateSource: p });
  }

  function patch(p: Partial<Settings>) {
    const next = { ...s, ...p } as Settings;
    app.settings = next;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      try {
        await app.saveSettings(next);
      } catch (e) {
        app.toast(String(e), { tone: "error" });
      }
    }, 350);
  }

  function setMode(k: Kind, m: Mode) {
    patch({ rules: { ...s.rules, [k]: m } });
    if (m === "auto") app.toast(`${KINDS[k].label} on autopilot. I'll handle them and tell you what I did.`);
  }

  const MODES: { value: Mode; label: string; title: string }[] = [
    { value: "off", label: "Off", title: "Don't look for these" },
    { value: "ask", label: "Ask me", title: "Find them and ask before doing anything" },
    { value: "auto", label: "Autopilot", title: "Handle them on my own and report back" },
  ];
  const POLICY_MODES: { value: PolicyMode; label: string; title: string }[] = [
    { value: "watch", label: "Watch", title: "Flag stale things one by one" },
    { value: "zero", label: "Keep empty", title: "Everything should leave: I propose where each item goes" },
    { value: "sorted", label: "Keep sorted", title: "File loose items into category sub-folders" },
  ];

  type ListKey = "inboxDirs" | "devRoots" | "libraryDirs" | "watchedDirs";
  async function addFolder(key: ListKey) {
    const p = await api.pickFolder();
    if (p && !s[key].includes(p)) patch({ [key]: [...s[key], p] } as Partial<Settings>);
  }
  function removeFolder(key: ListKey, p: string) {
    patch({ [key]: s[key].filter((x) => x !== p) } as Partial<Settings>);
  }
  async function pickSingle(key: "archiveRoot" | "coldStorage") {
    const p = await api.pickFolder();
    if (p) patch({ [key]: p } as Partial<Settings>);
  }
  async function addPolicy() {
    const p = await api.pickFolder();
    if (p && !s.policies.some((x) => x.path.toLowerCase() === p.toLowerCase())) {
      patch({ policies: [...s.policies, { path: p, mode: "zero", graceHours: 48, askEveryDays: 3 }] });
    }
  }
  function setPolicy(i: number, p: Partial<Settings["policies"][number]>) {
    patch({ policies: s.policies.map((x, j) => (j === i ? { ...x, ...p } : x)) });
  }
  async function pickDest(cat: string) {
    const p = await api.pickFolder();
    if (p) patch({ sortDest: { ...s.sortDest, [cat]: p } });
  }
  async function addMirror() {
    const a = await api.pickFolder();
    if (!a) return;
    app.toast("Now pick its mirror (the backup copy).");
    const b = await api.pickFolder();
    if (!b) return;
    try {
      await api.addMirror(a, b);
      patch({ mirrors: [...(s.mirrors ?? []), { a, b }] });
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    }
  }
  function toggleDrive(mount: string, on: boolean) {
    const all = app.drives.map((d) => d.mount);
    const current = s.drives.length ? s.drives : all;
    const next = on ? [...new Set([...current, mount])] : current.filter((d) => d !== mount);
    patch({ drives: next.length === all.length ? [] : next });
  }

  function recordHotkey(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    if (!mods.length) return;
    const key = e.code === "Space" ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.code.replace(/^Key|^Digit/, "");
    const field = recording;
    recording = null;
    patch({ [field]: [...mods, key].join("+") } as Partial<Settings>);
  }

  function tapVersion() {
    taps++;
    if (taps >= 7 && !s.devMode) {
      patch({ devMode: true });
      app.toast("Developer mode unlocked. Hi, maker! 🛠", { tone: "ok" });
      tab = "dev";
    } else if (taps >= 4 && taps < 7 && !s.devMode) {
      app.toast(`${7 - taps} more…`);
    }
  }

  async function dev(action: string) {
    try {
      devOut = await api.devAction(action);
    } catch (e) {
      devOut = String(e);
    }
  }

  async function unprotect(p: Protected) {
    await api.unprotect(p.path);
    protectedList = await api.protectedList();
    app.toast("I'll look at it again next time.", { tone: "ok" });
  }

  const hours = Array.from({ length: 24 }, (_, i) => i);
  const hh = (h: number) => `${String(h).padStart(2, "0")}:00`;
  const CATS: [string, string][] = [
    ["installer", "Installers"],
    ["archive", "Archives"],
    ["document", "Documents"],
    ["spreadsheet", "Spreadsheets"],
    ["presentation", "Presentations"],
    ["ebook", "E-books"],
    ["image", "Images"],
    ["screenshot", "Screenshots"],
    ["video", "Videos"],
    ["audio", "Audio"],
    ["script", "Scripts (.bat, .ps1…)"],
    ["code", "Code & projects"],
    ["font", "Fonts"],
    ["disk_image", "Disk images"],
    ["design", "Design files"],
    ["data", "Data files"],
    ["folder", "Other folders"],
    ["other", "Everything else"],
  ];
  const TABS = $derived([
    ["rules", "What I look for"],
    ["folders", "Folders & policies"],
    ["behavior", "Behavior"],
    ["protected", "Protected"],
    ["about", "About"],
    ...(s?.devMode ? [["dev", "Developer"]] : []),
  ] as [Tab, string][]);
</script>

<svelte:window onkeydown={recordHotkey} />

{#if s}
  <div class="rules scroll">
    <h1>Settings</h1>
    <nav class="tabs">
      {#each TABS as [id, label] (id)}
        <button class:on={tab === id} onclick={() => (tab = id)}>{label}</button>
      {/each}
    </nav>

    {#key tab}
      <div in:fade={{ duration: 180 }}>
        {#if tab === "rules"}
          <p class="muted lead">Teach me what to look for, and how much I'm allowed to do on my own.</p>
          <div class="grid">
            {#each KIND_ORDER as k, i (k)}
              {@const meta = KINDS[k]}
              {@const Icon = meta.icon}
              {@const mode = s.rules[k] ?? "ask"}
              {@const th = meta.threshold}
              <div class="rule card" class:off={mode === "off"} style="--c:{meta.color}" in:fly={{ y: 10, duration: 350, delay: i * 20, easing: cubicOut }}>
                <div class="rule-head">
                  <span class="ri"><Icon size={17} /></span>
                  <div class="rt">
                    <b>{meta.label}</b>
                    <span class="faint">{meta.blurb}</span>
                  </div>
                </div>
                <div class="rule-ctl">
                  <Segmented value={mode} options={MODES} onchange={(m) => setMode(k, m)} />
                  {#if mode === "auto"}<span class="autonote"><Bot size={13} /> acts on its own</span>{/if}
                </div>
                {#if th && mode !== "off"}
                  {@const val = s[th.key as keyof Settings] as number}
                  <label class="thresh">
                    <span class="muted">{th.label}</span>
                    <input type="range" min={th.min} max={th.max} step={th.step} value={val} oninput={(e) => patch({ [th.key]: +(e.currentTarget as HTMLInputElement).value } as Partial<Settings>)} />
                    <span class="val tabular">{val.toLocaleString()} {th.unit}</span>
                  </label>
                {/if}
              </div>
            {/each}
          </div>
        {:else if tab === "folders"}
          <p class="muted lead">Where I look, which folders follow special rules, and where things go when we tidy.</p>

          <h2><ListChecks size={17} /> Folder policies</h2>
          <p class="faint hint">“Keep empty” means everything in the folder should find a better home: I'll propose a plan you can edit before anything moves. “Keep sorted” files loose items into category sub-folders.</p>
          <div class="card settings">
            {#each s.policies as p, i (p.path)}
              <div class="policy">
                <div class="pname"><FolderOpen size={15} class="faint" /><b class="ellipsis">{shortPath(p.path, app.home)}</b></div>
                <Segmented value={p.mode} options={POLICY_MODES} onchange={(m) => setPolicy(i, { mode: m })} />
                {#if p.mode !== "watch"}
                  <label class="mini">grace
                    <select value={p.graceHours} onchange={(e) => setPolicy(i, { graceHours: +e.currentTarget.value })}>
                      {#each [0, 6, 24, 48, 72, 168] as g}<option value={g}>{g === 0 ? "none" : g < 24 ? `${g} h` : `${g / 24} d`}</option>{/each}
                    </select>
                  </label>
                  <label class="mini">ask every
                    <select value={p.askEveryDays} onchange={(e) => setPolicy(i, { askEveryDays: +e.currentTarget.value })}>
                      {#each [1, 2, 3, 7, 14, 30] as d}<option value={d}>{d} d</option>{/each}
                    </select>
                  </label>
                {/if}
                <button class="x" title="Remove policy" onclick={() => patch({ policies: s.policies.filter((_, j) => j !== i) })}><X size={13} /></button>
              </div>
            {/each}
            <button class="btn sm add" onclick={addPolicy}><FolderPlus size={14} /> Add a folder policy</button>
          </div>

          <h2><Eye size={17} /> Where I look</h2>
          <div class="folders">
            {#each [
              { key: "inboxDirs", title: "Inbox folders", blurb: "Where new stuff lands. Stale items, installers and archives are found here." },
              { key: "watchedDirs", title: "Extra folders to watch", blurb: "Any folder you want me to keep an eye on for old, forgotten things." },
              { key: "devRoots", title: "Project folders", blurb: "Where your code lives. Idle projects get build leftovers cleared, sleepy ones can hibernate." },
              { key: "libraryDirs", title: "Personal libraries", blurb: "Searched for duplicates, similar photos and big dormant files." },
            ] as fo (fo.key)}
              {@const key = fo.key as ListKey}
              <div class="card fcard">
                <div class="fhead">
                  <div><b>{fo.title}</b><p class="faint">{fo.blurb}</p></div>
                  <button class="btn sm" onclick={() => addFolder(key)}><FolderPlus size={14} /> Add</button>
                </div>
                <div class="flist">
                  {#each s[key] as p (p)}
                    <div class="fitem" transition:fly={{ x: -10, duration: 200 }}>
                      <FolderOpen size={14} class="faint" />
                      <span class="ellipsis mono">{p}</span>
                      <button class="x" title="Remove" onclick={() => removeFolder(key, p)}><X size={13} /></button>
                    </div>
                  {:else}
                    <span class="faint small">Nothing here yet.</span>
                  {/each}
                </div>
              </div>
            {/each}
          </div>

          <h2><Copy size={17} /> Mirrored folders (backups)</h2>
          <p class="faint hint">Folders that are copies of each other, like a photo archive on two drives. I never compare them for duplicates or similar photos. Tip: use “These folders are backups of each other…” on a duplicate card.</p>
          <div class="card settings">
            {#each s.mirrors ?? [] as m, i (m.a + m.b)}
              <div class="srow">
                <div><b class="mono ellipsis">{m.a}</b><span class="faint mono ellipsis">↔ {m.b}</span></div>
                <span></span>
                <button class="btn sm ghost" onclick={() => patch({ mirrors: s.mirrors.filter((_, j) => j !== i) })}>Remove</button>
              </div>
            {/each}
            <button class="btn sm add" onclick={addMirror}><FolderPlus size={14} /> Add a mirrored pair</button>
          </div>

          <h2><HardDrive size={17} /> Drives I look after</h2>
          <p class="faint hint">These get indexed for search and Space, and checked for duplicates, mystery folders and big dormant files.</p>
          <div class="card settings">
            {#each app.drives as d (d.mount)}
              <div class="srow">
                <div><b>{d.mount.replace(/\\$/, "")}{d.name ? ` · ${d.name}` : ""}</b><span class="faint">{d.kind.toUpperCase()} · {d.fs}</span></div>
                <span></span>
                <Toggle checked={!s.drives.length || s.drives.includes(d.mount)} onchange={(v) => toggleDrive(d.mount, v)} />
              </div>
            {/each}
          </div>

          <h2><FolderOpen size={17} /> Where things go</h2>
          <p class="faint hint">Used by organizing plans. When you change a destination inside a plan, I remember it for that file type.</p>
          <div class="card settings dests">
            {#each CATS as [cat, label] (cat)}
              {@const d = s.sortDest[cat] ?? ""}
              <div class="dest">
                <span>{label}</span>
                <select value={d === "recycle" || d === "leave" ? d : "folder"} onchange={(e) => {
                  const v = e.currentTarget.value;
                  if (v === "folder") pickDest(cat);
                  else patch({ sortDest: { ...s.sortDest, [cat]: v } });
                }}>
                  <option value="folder">Move to folder…</option>
                  <option value="recycle">Recycle</option>
                  <option value="leave">Leave it</option>
                </select>
                <button class="mono ellipsis destpath" onclick={() => pickDest(cat)} title="Change">{d === "recycle" ? "→ Recycle Bin" : d === "leave" ? "stays put" : shortPath(d, app.home)}</button>
              </div>
            {/each}
          </div>

          <div class="folders two">
            <div class="card fcard">
              <div class="fhead"><div><b>Archive folder</b><p class="faint">Where “Archive” moves things, in monthly folders.</p></div><button class="btn sm" onclick={() => pickSingle("archiveRoot")}>Change</button></div>
              <div class="fitem"><FolderOpen size={14} class="faint" /><span class="ellipsis mono">{s.archiveRoot}</span></div>
            </div>
            <div class="card fcard">
              <div class="fhead"><div><b>Cold storage</b><p class="faint">Where hibernated projects are zipped. Another drive is ideal.</p></div><button class="btn sm" onclick={() => pickSingle("coldStorage")}>Change</button></div>
              <div class="fitem"><FolderOpen size={14} class="faint" /><span class="ellipsis mono">{s.coldStorage}</span></div>
            </div>
          </div>
        {:else if tab === "behavior"}
          <p class="muted lead">When I work, how I talk to you, and how I learn.</p>

          <h2><Clock size={17} /> When I check</h2>
          <div class="card settings">
            <label class="srow">
              <div><b>Wait until you're away</b><span class="faint">I only look around after this much inactivity.</span></div>
              <input type="range" min="1" max="30" value={s.idleMin} oninput={(e) => patch({ idleMin: +e.currentTarget.value })} />
              <span class="val tabular">{s.idleMin} min</span>
            </label>
            <label class="srow">
              <div><b>Check at most every</b><span class="faint">When things are busy, I look this often.</span></div>
              <input type="range" min="10" max="240" step="5" value={s.scanFloorMin} oninput={(e) => patch({ scanFloorMin: +e.currentTarget.value })} />
              <span class="val tabular">{s.scanFloorMin} min</span>
            </label>
            <label class="srow">
              <div><b>Check at least every</b><span class="faint">When nothing changes, I slowly stretch out to this.</span></div>
              <input type="range" min="60" max="2880" step="60" value={s.scanCapMin} oninput={(e) => patch({ scanCapMin: +e.currentTarget.value })} />
              <span class="val tabular">{Math.round(s.scanCapMin / 60)} h</span>
            </label>
          </div>

          <h2><Bell size={17} /> Nudges</h2>
          <div class="card settings">
            <div class="srow"><div><b>Notify me</b><span class="faint">A friendly ping when something new starts gathering dust.</span></div><span></span><Toggle checked={s.notify} onchange={(v) => patch({ notify: v })} /></div>
            <div class="srow"><div><b>Quick actions in notifications</b><span class="faint">“Clean the sure ones” right from the toast, for things I'm 85%+ sure about.</span></div><span></span><Toggle checked={s.quickActions} onchange={(v) => patch({ quickActions: v })} /></div>
            <div class="srow"><div><b>Weekly digest</b><span class="faint">Once a week: what we freed together and what's waiting.</span></div><span></span><Toggle checked={s.weeklyDigest} onchange={(v) => patch({ weeklyDigest: v })} /></div>
            <label class="srow">
              <div><b>At most once every</b><span class="faint">I never nag. Promise.</span></div>
              <input type="range" min="1" max="72" value={s.nudgeGapHours} oninput={(e) => patch({ nudgeGapHours: +e.currentTarget.value })} />
              <span class="val tabular">{s.nudgeGapHours} h</span>
            </label>
            <div class="srow">
              <div><b>Quiet hours</b><span class="faint">No pings during these hours, or when you're gaming or presenting.</span></div>
              <div class="hours">
                <select value={s.quietStart} onchange={(e) => patch({ quietStart: +e.currentTarget.value })}>{#each hours as h}<option value={h}>{hh(h)}</option>{/each}</select>
                <span class="faint">to</span>
                <select value={s.quietEnd} onchange={(e) => patch({ quietEnd: +e.currentTarget.value })}>{#each hours as h}<option value={h}>{hh(h)}</option>{/each}</select>
              </div>
            </div>
            <label class="srow">
              <div><b>“Later” means</b><span class="faint">How long snoozed things stay out of sight.</span></div>
              <input type="range" min="1" max="90" value={s.snoozeDays} oninput={(e) => patch({ snoozeDays: +e.currentTarget.value })} />
              <span class="val tabular">{s.snoozeDays} days</span>
            </label>
          </div>

          <h2><Search size={17} /> Search & index</h2>
          <div class="card settings">
            {#each [
              { field: "searchHotkey", title: "Search shortcut", blurb: "Opens the search bar from anywhere in Windows." },
              { field: "paletteHotkey", title: "Command shortcut", blurb: "Dusty's command palette, from anywhere: clean, pause, open views…" },
              { field: "voiceHotkey", title: "Voice shortcut", blurb: "Ask for a file out loud (when voice is on)." },
            ] as h (h.field)}
              {@const f = h.field as HotkeyField}
              <div class="srow">
                <div><b>{h.title}</b><span class="faint">{h.blurb}</span></div>
                <span></span>
                <button class="btn sm hotkey" class:rec={recording === f} onclick={() => (recording = recording === f ? null : f)}>{recording === f ? "Press keys…" : s[f] || "None"}</button>
              </div>
            {/each}
            <div class="srow"><div><b>Index my drives</b><span class="faint">{app.index?.ready ? `${app.index.entries.toLocaleString()} files, ~${app.index.memoryMb} MB of memory, built in ${(app.index.buildMs / 1000).toFixed(0)} s, ${ago(app.index.builtAt)}.` : "Makes search and Space instant."}</span></div><span></span><Toggle checked={s.indexEnabled} onchange={(v) => patch({ indexEnabled: v })} /></div>
            <div class="srow"><div><b>Rebuild the index now</b><span class="faint">It refreshes itself every 12 hours at idle and follows changes live in between.</span></div><span></span><button class="btn sm" disabled={app.index?.building} onclick={() => api.indexRebuild()}><Database size={13} /> {app.index?.building ? `Indexing… ${app.index.progress.toLocaleString()}` : "Rebuild"}</button></div>
          </div>

          <h2><Mic size={17} /> Voice</h2>
          <p class="faint hint">Ask for files out loud. Speech is turned into text <b>on this PC</b> by a small open model; no audio or words ever leave your computer. Dusty only searches, it never takes actions by voice.</p>
          <div class="card settings">
            <div class="srow">
              <div><b>Voice search</b><span class="faint">{voice?.hasMic === false ? "No microphone found." : `Press ${s.voiceHotkey} anywhere, or the mic in the search bar.`}</span></div>
              <span></span>
              <Toggle checked={s.voiceEnabled} onchange={(v) => (v && !voice?.models.find((m) => m.id === s.voiceModel)?.installed ? downloadVoice(s.voiceModel) : patch({ voiceEnabled: v }))} />
            </div>
            {#each voice?.models ?? [] as m (m.id)}
              <div class="srow">
                <div><b>{m.name}</b><span class="faint">{m.languages} · {m.sizeMb} MB{m.installed ? " · downloaded" : ""}</span></div>
                <span>
                  {#if voiceProgress !== null && s.voiceModel === m.id}
                    <span class="bar"><i style="width:{Math.round(voiceProgress * 100)}%"></i></span>
                  {/if}
                </span>
                {#if s.voiceModel === m.id && m.installed}
                  <span class="chip on">In use</span>
                {:else if m.installed}
                  <button class="btn sm" onclick={() => patch({ voiceModel: m.id })}>Use</button>
                {:else}
                  <button class="btn sm" disabled={voiceProgress !== null} onclick={() => { patch({ voiceModel: m.id }); downloadVoice(m.id); }}>Download</button>
                {/if}
              </div>
            {/each}
            <div class="srow"><div><b>Answer out loud</b><span class="faint">A short spoken summary of what I found. Off by default.</span></div><span></span><Toggle checked={s.voiceReply} onchange={(v) => patch({ voiceReply: v })} /></div>
          </div>

          <h3 class="sub">Dusty's voice</h3>
          <p class="faint hint">Small neural voices that run on this PC, downloaded once. Pick one and press play.</p>
          <div class="card settings">
            {#each ttsModels as m (m.id)}
              <div class="srow">
                <div><b>{m.name}</b><span class="faint">{m.blurb} {m.languages} · {m.sizeMb} MB</span></div>
                <span>
                  {#if ttsProgress?.id === m.id}
                    <span class="bar"><i style="width:{Math.round(ttsProgress.p * 100)}%"></i></span>
                  {/if}
                </span>
                {#if m.installed}
                  <span class="vbtns">
                    <button class="btn sm ghost" title="Hear it" onclick={() => { patch({ ttsModel: m.id }); tryVoice(m.id); }}><Play size={13} /></button>
                    {#if s.ttsModel === m.id}<span class="chip on">In use</span>{:else}<button class="btn sm" onclick={() => patch({ ttsModel: m.id })}>Use</button>{/if}
                  </span>
                {:else}
                  <button class="btn sm" disabled={ttsProgress !== null} onclick={() => downloadTts(m.id)}>Download</button>
                {/if}
              </div>
            {/each}
            <div class="srow">
              <div><b>Windows voices</b><span class="faint">Built in, nothing to download. More robotic.</span></div>
              <span></span>
              <span class="vbtns">
                <button class="btn sm ghost" title="Hear it" onclick={() => tryVoice("system")}><Play size={13} /></button>
                {#if s.ttsModel === "system"}<span class="chip on">In use</span>{:else}<button class="btn sm" onclick={() => patch({ ttsModel: "system" })}>Use</button>{/if}
              </span>
            </div>
          </div>

          <h2><Footprints size={17} /> Dusty's presence</h2>
          <div class="card settings">
            <div class="srow"><div><b>Let Dusty wander</b><span class="faint">Every few hours, a tiny Dusty strolls along the bottom of your screen with a broom and a thought. Click-through, never in games or presentations.</span></div><span></span><Toggle checked={s.wander} onchange={(v) => patch({ wander: v })} /></div>
            <div class="srow"><div><b>Say hi now</b><span class="faint">See what it looks like.</span></div><span></span><button class="btn sm" onclick={() => api.buddyTest()}>Wave 👋</button></div>
          </div>

          <h2><Brain size={17} /> Smarts</h2>
          <div class="card settings">
            <div class="srow"><div><b>Learn from my decisions</b><span class="faint">If you always keep certain files, I stop asking; if you always remove them, I get bolder.</span></div><span></span><Toggle checked={s.learn} onchange={(v) => patch({ learn: v })} /></div>
            <div class="srow"><div><b>Watch startup apps</b><span class="faint">Tell me when an app adds itself to Windows startup.</span></div><span></span><Toggle checked={s.watchStartup} onchange={(v) => patch({ watchStartup: v })} /></div>
            <div class="srow"><div><b>Watch background hogs</b><span class="faint">Flag leftover dev processes and apps busy while you're away.</span></div><span></span><Toggle checked={s.watchHogs} onchange={(v) => patch({ watchHogs: v })} /></div>
          </div>

          <h2><Power size={17} /> General</h2>
          <div class="card settings">
            <div class="srow"><div><b>Start with Windows</b><span class="faint">I start quietly in the tray and stay out of your way.</span></div><span></span><Toggle checked={s.startWithWindows} onchange={(v) => patch({ startWithWindows: v })} /></div>
          </div>
        {:else if tab === "protected"}
          <p class="muted lead">Things you told me not to touch. I skip them and everything inside.</p>
          <div class="card settings">
            {#each protectedList as p (p.path)}
              <div class="srow">
                <div><b class="mono ellipsis">{p.path}</b><span class="faint">{p.until === 0 ? "Forever" : `Until ${new Date(p.until * 1000).toLocaleDateString()}`} · added {ago(p.at)}</span></div>
                <span></span>
                <button class="btn sm ghost" onclick={() => unprotect(p)}>Remove</button>
              </div>
            {:else}
              <p class="faint pad"><ShieldCheck size={14} /> Nothing protected yet. Use “Don't touch this” on any card or right-click in Space.</p>
            {/each}
          </div>
        {:else if tab === "about"}
          <div class="about">
            <Mascot mood="wave" size={150} />
            <h2 class="title">Dusty</h2>
            <button class="version faint" onclick={tapVersion} title="Version">Version {info?.version ?? "…"}{info?.debug ? " (dev build)" : ""}</button>
            <p class="muted">A tiny housekeeper for your PC. Made with <Heart size={13} class="heart" /> by <b>{CONFIG.author}</b>.</p>
            <div class="about-acts">
              <button class="btn primary lg coffee" disabled={!CONFIG.coffeeUrl} title={CONFIG.coffeeUrl ? "Thank you!" : "Coming soon"} onclick={() => CONFIG.coffeeUrl && api.openFile(CONFIG.coffeeUrl)}>
                <Coffee size={18} /> Buy me a coffee
              </button>
              <button class="btn lg" onclick={() => api.openFile(CONFIG.repoUrl)}><Code size={16} /> Source code <ExternalLink size={13} /></button>
            </div>
            <div class="card updates">
              <div class="uh">
                <b><RefreshCw size={15} /> Updates</b>
                {#if update?.available}
                  <span class="chip on">Dusty {update.available} is ready</span>
                {:else if update}
                  <span class="faint">You're up to date</span>
                {/if}
              </div>
              {#if update?.notes && update.available}<p class="muted small">{update.notes}</p>{/if}
              {#if updateError}<p class="err small">{updateError}</p>{/if}
              <div class="row">
                <button class="btn sm" disabled={checking} onclick={checkUpdate}>{checking ? "Checking…" : "Check now"}</button>
                {#if update?.available}<button class="btn sm primary" onclick={() => api.installUpdate(false)}>Update & restart</button>{/if}
              </div>
              <div class="srow tight"><div><b>Install updates by myself</b><span class="faint">While you're away; Dusty restarts in a few seconds. Updates are signature-checked first.</span></div><span></span><Toggle checked={s.autoInstall} onchange={(v) => patch({ autoInstall: v })} /></div>
              <div class="srow tight">
                <div><b>Update source</b><span class="faint mono ellipsis">{s.updateSource}</span></div>
                <span></span>
                <button class="btn sm ghost" onclick={pickUpdateFolder}>Folder…</button>
              </div>
              <input class="src" placeholder="…or a URL to latest.json" value={s.updateSource.startsWith("http") ? s.updateSource : ""} onchange={(e) => e.currentTarget.value && patch({ updateSource: e.currentTarget.value })} />
            </div>
            <p class="faint small credits">Private by design: everything runs on this PC, nothing is uploaded, no telemetry. Open source. Built with Rust, Tauri and Svelte.</p>
          </div>
        {:else if tab === "dev"}
          <p class="muted lead"><Bug size={15} /> Developer tools. Poke things without waiting for Dusty's schedule.</p>
          <div class="devgrid">
            {#each [
              ["toast", "Test notification"],
              ["nudge", "Nudge about current findings"],
              ["autopilot_toast", "Autopilot summary toast"],
              ["digest", "Weekly digest now"],
              ["scan_bg", "Background scan (low priority)"],
              ["rebuild_index", "Rebuild index"],
              ["reset_onboarding", "Show onboarding again"],
              ["reset_seen", "Forget history & decisions"],
              ["open_data", "Open data folder"],
            ] as [a, label] (a)}
              <button class="btn" onclick={() => dev(a)}>{label}</button>
            {/each}
            <button class="btn ghost" onclick={() => { patch({ devMode: false }); tab = "about"; taps = 0; }}>Turn developer mode off</button>
          </div>
          {#if devOut}<pre class="devout">{devOut}</pre>{/if}
          <p class="faint small"><Info size={12} /> Data lives in <span class="mono">{info?.dataDir}</span></p>
        {/if}
      </div>
    {/key}
  </div>
{/if}

<style>
  .rules {
    height: 100%;
    padding: 42px 40px 80px;
  }
  h1 {
    font-size: 28px;
  }
  .tabs {
    display: flex;
    gap: 4px;
    margin: 14px 0 6px;
    padding: 4px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    width: fit-content;
    position: sticky;
    top: 0;
    z-index: 3;
    backdrop-filter: blur(20px);
  }
  .tabs button {
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    font-weight: 600;
    font-size: 13px;
    color: var(--text-2);
    transition: all 0.2s;
  }
  .tabs button.on {
    background: var(--surface-3);
    color: var(--text);
  }
  .lead {
    margin: 12px 0 8px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    margin: 28px 0 8px;
  }
  .hint {
    margin: -2px 0 12px;
    font-size: 12.5px;
    max-width: 760px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
    margin-top: 14px;
  }
  .rule {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition:
      opacity 0.3s,
      border-color 0.3s;
  }
  .rule.off {
    opacity: 0.6;
  }
  .rule-head {
    display: flex;
    gap: 12px;
  }
  .ri {
    width: 36px;
    height: 36px;
    flex: none;
    border-radius: 11px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
  }
  .rt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
  }
  .rt b {
    font-size: 14px;
  }
  .rule-ctl {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .autonote {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--accent);
  }
  .thresh,
  .srow {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 14px;
    font-size: 13px;
  }
  .val {
    min-width: 76px;
    text-align: right;
    font-weight: 600;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    height: 4px;
    border-radius: 4px;
    background: var(--surface-3);
    outline: none;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
    transition: transform 0.15s var(--ease);
  }
  input[type="range"]::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }
  .folders {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }
  .folders.two {
    margin-top: 12px;
  }
  .fcard {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fhead {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .fhead p {
    margin: 2px 0 0;
    font-size: 12.5px;
  }
  .flist {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fitem {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 10px;
    border-radius: 9px;
    background: var(--surface-2);
    min-width: 0;
  }
  .fitem .ellipsis {
    flex: 1;
  }
  .x {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--text-3);
    flex: none;
  }
  .x:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .small {
    font-size: 12px;
  }
  .settings {
    padding: 6px 18px;
    display: flex;
    flex-direction: column;
  }
  .srow {
    grid-template-columns: minmax(260px, 1.3fr) 1fr auto;
    padding: 13px 0;
    border-bottom: 1px solid var(--border);
  }
  .srow:last-child {
    border-bottom: 0;
  }
  .srow > div:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .srow > div:first-child .faint {
    font-size: 12.5px;
  }
  .hours {
    grid-column: 2 / 4;
    justify-self: end;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  select {
    height: 32px;
    padding: 0 10px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  select option {
    background: var(--bg-solid);
  }
  .policy {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) auto auto auto auto;
    gap: 12px;
    align-items: center;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .pname {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .mini {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
  }
  .mini select {
    height: 28px;
    font-size: 12px;
  }
  .add {
    margin: 12px 0;
    align-self: flex-start;
  }
  .dests {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(420px, 1fr));
    gap: 0 24px;
  }
  .dest {
    display: grid;
    grid-template-columns: 150px 140px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .dest select {
    height: 28px;
    font-size: 12px;
  }
  .destpath {
    text-align: left;
    color: var(--text-2);
  }
  .destpath:hover {
    color: var(--accent);
  }
  .hotkey {
    min-width: 150px;
    justify-content: center;
    font-family: var(--mono);
  }
  .hotkey.rec {
    border-color: var(--accent);
    color: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 4px var(--accent-soft);
    }
  }
  .pad {
    padding: 14px 0;
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
    padding: 30px 0;
  }
  .title {
    font-size: 32px;
    margin: 8px 0 0;
  }
  .version {
    font-size: 12.5px;
    padding: 2px 8px;
    border-radius: 6px;
  }
  .version:active {
    transform: scale(0.96);
  }
  .about :global(.heart) {
    color: var(--danger);
    vertical-align: -2px;
  }
  .about-acts {
    display: flex;
    gap: 10px;
    margin: 14px 0;
  }
  .coffee {
    background: linear-gradient(135deg, #ffcf6e, #ff8f5a);
    color: #2a1606;
    box-shadow: 0 8px 24px -8px rgba(255, 143, 90, 0.7);
  }
  .updates {
    width: min(560px, 100%);
    margin-top: 18px;
    padding: 14px 18px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .updates .uh {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .updates .uh b {
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }
  .updates .row {
    display: flex;
    gap: 8px;
  }
  .srow.tight {
    padding: 8px 0;
  }
  .src {
    height: 32px;
    padding: 0 10px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 12px;
    outline: none;
  }
  .err {
    color: var(--danger);
    margin: 0;
  }
  .chip.on {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: var(--accent-soft);
  }
  .sub {
    margin: 18px 0 4px;
    font-size: 14px;
  }
  .vbtns {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .bar {
    display: block;
    height: 4px;
    border-radius: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--grad);
    transition: width 0.2s;
  }
  .credits {
    margin-top: 10px;
  }
  .devgrid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 8px;
    margin: 14px 0;
  }
  .devout {
    padding: 12px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--border);
    font-family: var(--mono);
    font-size: 12px;
    user-select: text;
  }
</style>
