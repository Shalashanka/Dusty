<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { FolderPlus, X, Bot, Bell, Clock, Power, FolderOpen, MapPin, Search } from "@lucide/svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type Kind, type Mode, type Settings } from "$lib/api";
  import { KINDS, KIND_ORDER } from "$lib/kinds";

  const s = $derived(app.settings!);
  let saveTimer: ReturnType<typeof setTimeout>;

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

  type ListKey = "inboxDirs" | "devRoots" | "libraryDirs";
  async function addFolder(key: ListKey) {
    const p = await api.pickFolder();
    if (p && !s[key].includes(p)) patch({ [key]: [...s[key], p] } as Partial<Settings>);
  }
  function removeFolder(key: ListKey, p: string) {
    patch({ [key]: s[key].filter((x) => x !== p) } as Partial<Settings>);
  }
  async function pickSingle(key: "archiveRoot" | "desktopDir") {
    const p = await api.pickFolder();
    if (p) patch({ [key]: p } as Partial<Settings>);
  }

  const FOLDERS: { key: ListKey; title: string; blurb: string }[] = [
    { key: "inboxDirs", title: "Inbox folders", blurb: "Where new stuff lands. Stale items, installers and archives are found here." },
    { key: "devRoots", title: "Project folders", blurb: "Where your code lives. Idle projects get their build leftovers cleared." },
    { key: "libraryDirs", title: "Personal libraries", blurb: "Searched for duplicates and big dormant files." },
  ];
  const hours = Array.from({ length: 24 }, (_, i) => i);
  const hh = (h: number) => `${String(h).padStart(2, "0")}:00`;
</script>

{#if s}
  <div class="rules scroll">
    <h1>Rules</h1>
    <p class="muted lead">Teach me what to look for, where, and how much I'm allowed to do on my own.</p>

    <h2><Search size={17} /> What I look for</h2>
    <div class="grid">
      {#each KIND_ORDER as k, i (k)}
        {@const meta = KINDS[k]}
        {@const Icon = meta.icon}
        {@const mode = s.rules[k] ?? "ask"}
        {@const th = meta.threshold}
        <div class="rule card" class:off={mode === "off"} style="--c:{meta.color}" in:fly={{ y: 12, duration: 400, delay: i * 30, easing: cubicOut }}>
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
              <input
                type="range"
                min={th.min}
                max={th.max}
                step={th.step}
                value={val}
                oninput={(e) => patch({ [th.key]: +(e.currentTarget as HTMLInputElement).value } as Partial<Settings>)}
              />
              <span class="val tabular">{val.toLocaleString()} {th.unit}</span>
            </label>
          {/if}
        </div>
      {/each}
    </div>

    <h2><MapPin size={17} /> Where I look</h2>
    <div class="folders">
      {#each FOLDERS as fo (fo.key)}
        <div class="card fcard">
          <div class="fhead">
            <div>
              <b>{fo.title}</b>
              <p class="faint">{fo.blurb}</p>
            </div>
            <button class="btn sm" onclick={() => addFolder(fo.key)}><FolderPlus size={14} /> Add</button>
          </div>
          <div class="flist">
            {#each s[fo.key] as p (p)}
              <div class="fitem" transition:fly={{ x: -10, duration: 200 }}>
                <FolderOpen size={14} class="faint" />
                <span class="ellipsis mono">{p}</span>
                <button class="x" title="Remove" onclick={() => removeFolder(fo.key, p)}><X size={13} /></button>
              </div>
            {:else}
              <span class="faint small">Nothing here yet.</span>
            {/each}
          </div>
        </div>
      {/each}
      <div class="card fcard">
        <div class="fhead">
          <div>
            <b>Archive folder</b>
            <p class="faint">Where “Archive” moves things, in monthly folders. Point it at a roomier drive.</p>
          </div>
          <button class="btn sm" onclick={() => pickSingle("archiveRoot")}>Change</button>
        </div>
        <div class="fitem"><FolderOpen size={14} class="faint" /><span class="ellipsis mono">{s.archiveRoot}</span></div>
      </div>
    </div>

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
      <div class="srow">
        <div><b>Notify me</b><span class="faint">A friendly ping when something new starts gathering dust.</span></div>
        <Toggle checked={s.notify} onchange={(v) => patch({ notify: v })} />
      </div>
      <label class="srow">
        <div><b>At most once every</b><span class="faint">I never nag. Promise.</span></div>
        <input type="range" min="1" max="72" value={s.nudgeGapHours} oninput={(e) => patch({ nudgeGapHours: +e.currentTarget.value })} />
        <span class="val tabular">{s.nudgeGapHours} h</span>
      </label>
      <div class="srow">
        <div><b>Quiet hours</b><span class="faint">No pings during these hours, or when you're gaming or presenting.</span></div>
        <div class="hours">
          <select value={s.quietStart} onchange={(e) => patch({ quietStart: +e.currentTarget.value })}>
            {#each hours as h}<option value={h}>{hh(h)}</option>{/each}
          </select>
          <span class="faint">to</span>
          <select value={s.quietEnd} onchange={(e) => patch({ quietEnd: +e.currentTarget.value })}>
            {#each hours as h}<option value={h}>{hh(h)}</option>{/each}
          </select>
        </div>
      </div>
      <label class="srow">
        <div><b>“Later” means</b><span class="faint">How long snoozed things stay out of sight.</span></div>
        <input type="range" min="1" max="90" value={s.snoozeDays} oninput={(e) => patch({ snoozeDays: +e.currentTarget.value })} />
        <span class="val tabular">{s.snoozeDays} days</span>
      </label>
    </div>

    <h2><Power size={17} /> General</h2>
    <div class="card settings">
      <div class="srow">
        <div><b>Start with Windows</b><span class="faint">I start quietly in the tray and stay out of your way.</span></div>
        <Toggle checked={s.startWithWindows} onchange={(v) => patch({ startWithWindows: v })} />
      </div>
    </div>
  </div>
{/if}

<style>
  .rules {
    height: 100%;
    padding: 28px 40px 80px;
  }
  h1 {
    font-size: 28px;
  }
  .lead {
    margin: 6px 0 8px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    margin: 30px 0 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }
  .rule {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.3s, border-color 0.3s;
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
    transition: transform 0.15s var(--spring);
  }
  input[type="range"]::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }
  .folders {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
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
    width: 22px;
    height: 22px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--text-3);
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
    grid-template-columns: minmax(260px, 1.2fr) 1fr auto;
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }
  .srow:last-child {
    border-bottom: 0;
  }
  .srow > div:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .srow > div:first-child .faint {
    font-size: 12.5px;
  }
  .srow :global(.toggle) {
    grid-column: 3;
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
</style>
