<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Rocket, Activity, HardDrive, Wrench, Package, ShieldAlert, CircleStop, Zap, Moon, Recycle, Layers, History as HistoryIcon, BatteryCharging, LoaderCircle } from "@lucide/svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import Ring from "$lib/components/Ring.svelte";
  import Counter from "$lib/components/Counter.svelte";
  import { app } from "$lib/app.svelte";
  import { api, type AppxApp, type OptimizeInfo, type PowerPlan, type ProcSnapshot, type StartupItem, type SystemFiles } from "$lib/api";
  import { ago, bytes } from "$lib/format";

  let startup = $state<StartupItem[]>([]);
  let procs = $state<ProcSnapshot | null>(null);
  let sys = $state<SystemFiles | null>(null);
  let plans = $state<PowerPlan[]>([]);
  let opt = $state<OptimizeInfo | null>(null);
  let appx = $state<AppxApp[] | null>(null);
  let busy = $state<string | null>(null);
  let frag = $state<Record<string, number>>({});
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    api.setLiveProcs(true);
    unlisten = await listen("procs", async () => (procs = await api.processes()));
    [startup, procs] = await Promise.all([api.startupList(), api.processes()]);
    [sys, plans, opt] = await Promise.all([api.systemFiles(), api.powerPlans(), api.optimizeInfo()]);
  });
  onDestroy(() => {
    api.setLiveProcs(false);
    unlisten?.();
  });

  const memPct = $derived(procs ? procs.usedMemory / procs.totalMemory : 0);
  const enabledStartup = $derived(startup.filter((s) => s.enabled).length);
  const visible = $derived(
    (procs?.groups ?? []).filter((g) => !g.flags.includes("system") && g.name.toLowerCase() !== "dusty.exe").slice(0, 16),
  );
  const flagged = $derived(visible.filter((g) => g.flags.some((f) => f !== "system")).length);
  const runningMem = (s: StartupItem) => {
    const exe = s.exe?.split("\\").pop()?.toLowerCase();
    const g = procs?.groups.find((g) => g.name.toLowerCase() === exe);
    return g ? g.memory : 0;
  };

  async function run<T>(key: string, fn: () => Promise<T>, ok?: string): Promise<T | undefined> {
    busy = key;
    try {
      const r = await fn();
      if (ok) app.toast(ok, { tone: "ok" });
      return r;
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    } finally {
      busy = null;
    }
  }

  async function toggleStartup(s: StartupItem, on: boolean) {
    const r = await run(s.id, () => api.startupSet(s.id, on).then(() => true), on ? `${s.name} will start with Windows` : `${s.name} won't start with Windows anymore`);
    if (!r) s.enabled = !on;
    startup = await api.startupList();
  }

  async function end(pids: number[], name: string) {
    if (!confirm(`End ${pids.length > 1 ? `${pids.length} copies of ` : ""}${name}? Unsaved work in it would be lost.`)) return;
    const n = await run("kill", () => api.killProcesses(pids));
    if (n !== undefined) app.toast(n ? `Ended ${n} process${n > 1 ? "es" : ""}` : "Couldn't end it", { tone: n ? "ok" : "error" });
    procs = await api.processes();
  }

  async function optimize(letter: string) {
    await run(`opt-${letter}`, () => api.optimizeDrive(letter), `${letter} optimized`);
  }
  async function analyze(letter: string) {
    const r = await run(`frag-${letter}`, () => api.analyzeDrive(letter));
    if (r !== undefined) frag = { ...frag, [letter]: r };
  }
  async function tune(action: string, ok: string, arg?: string) {
    await run(action, () => api.tune(action, arg), ok);
    sys = await api.systemFiles();
  }
  async function setPlan(guid: string) {
    await run("plan", () => api.setPowerPlan(guid), "Power plan switched");
    plans = await api.powerPlans();
  }
  async function loadApps() {
    appx = (await run("appx", () => api.preinstalledApps())) ?? [];
  }
  async function removeApp(a: AppxApp) {
    await run(a.fullName, () => api.tune("remove_app", a.fullName), `${a.friendly} removed`);
    appx = appx?.filter((x) => x.fullName !== a.fullName) ?? null;
  }
</script>

<div class="speed scroll">
  <h1>Speed</h1>
  <p class="muted lead">Less baggage, faster PC. Everything here is reversible unless it says otherwise.</p>

  <section class="summary">
    <div class="card sum" in:fly={{ y: 12, duration: 400, easing: cubicOut }}>
      <div class="ring"><Ring value={memPct} size={64} stroke={7} color={memPct > 0.85 ? "var(--danger)" : memPct > 0.7 ? "var(--warn)" : "var(--accent)"} /><span>{Math.round(memPct * 100)}%</span></div>
      <div><b>Memory in use</b><span class="faint">{bytes(procs?.usedMemory ?? 0)} of {bytes(procs?.totalMemory ?? 0)}</span></div>
    </div>
    <div class="card sum" in:fly={{ y: 12, duration: 400, delay: 50, easing: cubicOut }}>
      <span class="big"><Counter value={enabledStartup} /></span>
      <div><b>Startup apps</b><span class="faint">start with Windows</span></div>
    </div>
    <div class="card sum" in:fly={{ y: 12, duration: 400, delay: 100, easing: cubicOut }}>
      <span class="big" class:warn={flagged > 0}><Counter value={flagged} /></span>
      <div><b>Suspicious processes</b><span class="faint">leftovers or busy while idle</span></div>
    </div>
  </section>

  <h2><Rocket size={17} /> Startup apps</h2>
  <p class="faint hint">Each one slows down boot a little. Turning one off just stops it from launching by itself; the app stays installed.</p>
  <div class="card table">
    {#each startup as s (s.id)}
      <div class="trow">
        <div class="tname">
          <b class="ellipsis">{s.name}</b>
          <span class="faint mono ellipsis" title={s.command}>{s.command}</span>
        </div>
        <span class="tmeta faint">{runningMem(s) ? `${bytes(runningMem(s))} now` : ""}</span>
        {#if s.machine}<span class="chip tiny" title="Changing this asks for administrator permission"><ShieldAlert size={12} /> all users</span>{:else}<span></span>{/if}
        <Toggle checked={s.enabled} onchange={(v) => toggleStartup(s, v)} />
      </div>
    {:else}
      <p class="faint pad">Nothing starts with Windows. Impressive.</p>
    {/each}
  </div>

  <h2><Activity size={17} /> What's running</h2>
  <p class="faint hint">Live, grouped by app. I flag dev servers left running after their terminal closed, and apps that keep working while you're away.</p>
  <div class="card table">
    <div class="trow head faint"><span>App</span><span>Memory</span><span>CPU</span><span>Disk</span><span></span></div>
    {#each visible as g (g.name)}
      <div class="trow procs">
        <div class="tname">
          <b class="ellipsis">{g.name}{g.count > 1 ? ` ×${g.count}` : ""}</b>
          <span class="flags">
            {#if g.flags.includes("leftover")}<span class="flag bad">{g.leftoverPids.length} leftover{g.leftoverPids.length > 1 ? "s" : ""}</span>{/if}
            {#if g.flags.includes("busy_idle")}<span class="flag warn">busy while you're away ({g.idleCpu.toFixed(0)}%)</span>{/if}
            {#if g.flags.includes("memory_hog")}<span class="flag warn">hungry, no window</span>{/if}
            {#if !g.hasWindow && !g.flags.length}<span class="faint small">background</span>{/if}
          </span>
        </div>
        <span class="tabular">{bytes(g.memory)}</span>
        <span class="tabular">{g.cpu.toFixed(1)}%</span>
        <span class="tabular faint">{g.diskRate > 1024 ? `${bytes(g.diskRate)}/s` : ""}</span>
        <span class="tact">
          {#if g.leftoverPids.length}
            <button class="btn sm danger" onclick={() => end(g.leftoverPids, g.name)}><CircleStop size={13} /> End leftovers</button>
          {:else}
            <button class="btn sm ghost" onclick={() => end(g.pids, g.name)}>End</button>
          {/if}
        </span>
      </div>
    {/each}
  </div>

  <h2><HardDrive size={17} /> Drives</h2>
  <p class="faint hint">
    Optimizing defragments hard disks and re-trims SSDs, whichever fits. Windows does it weekly too:
    {#if opt}<b>{opt.scheduled ? "scheduled" : "turned off"}</b>{#if opt.lastRun} · last run {ago(Date.parse(opt.lastRun) / 1000)}{/if}{:else}…{/if}.
  </p>
  <div class="drives">
    {#each app.drives as d (d.mount)}
      {@const letter = d.mount.replace(/\\$/, "")}
      <div class="card drive">
        <b>{letter}{d.name ? ` · ${d.name}` : ""}</b>
        <span class="faint">{d.kind === "ssd" ? "SSD: benefits from TRIM, never needs defrag" : d.kind === "hdd" ? "Hard disk: defragmenting helps" : "Unknown drive type"}</span>
        {#if frag[letter] !== undefined}<span class="frag">Fragmentation: <b>{frag[letter]}%</b> {frag[letter] < 5 ? "· nothing to worry about" : "· worth optimizing"}</span>{/if}
        <div class="row">
          <button class="btn sm" disabled={busy === `opt-${letter}`} onclick={() => optimize(letter)}>
            {#if busy === `opt-${letter}`}<LoaderCircle size={13} class="spin" />{:else}<Zap size={13} />{/if}
            {d.kind === "ssd" ? "Re-trim" : "Optimize"}
          </button>
          {#if d.kind !== "ssd"}
            <button class="btn sm ghost" disabled={busy === `frag-${letter}`} onclick={() => analyze(letter)}>Check fragmentation</button>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <h2><Wrench size={17} /> Tune-ups</h2>
  <p class="faint hint">The deep cleaning a fresh install gives you, without reinstalling. Ones marked with a shield ask for administrator permission.</p>
  <div class="tunes">
    <div class="card tune">
      <span class="ic"><Moon size={18} /></span>
      <div><b>Hibernation file</b><span class="faint">{sys?.hiberfil ? `${bytes(sys.hiberfil)} reserved on ${sys.systemDrive}. Turning hibernation off frees it (and disables Fast Startup).` : "Hibernation is off. Nothing reserved."}</span></div>
      {#if sys?.hiberfil}
        <button class="btn sm" disabled={busy === "hibernation_off"} onclick={() => tune("hibernation_off", "Hibernation turned off")}><ShieldAlert size={13} /> Turn off</button>
      {:else}
        <button class="btn sm ghost" disabled={busy === "hibernation_on"} onclick={() => tune("hibernation_on", "Hibernation turned on")}>Turn on</button>
      {/if}
    </div>
    <div class="card tune">
      <span class="ic"><Recycle size={18} /></span>
      <div><b>Delivery Optimization cache</b><span class="faint">Update pieces Windows shares with other PCs. Safe to clear.</span></div>
      <button class="btn sm" disabled={busy === "delivery_optimization"} onclick={() => tune("delivery_optimization", "Delivery Optimization cache cleared")}><ShieldAlert size={13} /> Clear</button>
    </div>
    <div class="card tune">
      <span class="ic"><Layers size={18} /></span>
      <div><b>Windows component cleanup</b><span class="faint">Removes superseded update files from WinSxS. Takes a few minutes; can't be undone (old updates can't be uninstalled after).</span></div>
      <button class="btn sm" disabled={busy === "component_cleanup"} onclick={() => tune("component_cleanup", "Component store cleaned")}>
        {#if busy === "component_cleanup"}<LoaderCircle size={13} class="spin" />{:else}<ShieldAlert size={13} />{/if} Clean up
      </button>
    </div>
    {#if sys?.windowsOld}
      <div class="card tune">
        <span class="ic"><HistoryIcon size={18} /></span>
        <div><b>Previous Windows install</b><span class="faint">Windows.old is still around ({bytes(sys.windowsOld)}). Remove it from Storage Sense once you're happy with the upgrade.</span></div>
        <button class="btn sm" onclick={() => tune("storage_sense", "Opened Storage Sense")}>Open Storage Sense</button>
      </div>
    {/if}
    <div class="card tune">
      <span class="ic"><HistoryIcon size={18} /></span>
      <div><b>Restore points</b><span class="faint">They can quietly use many gigabytes. Adjust their space in System Protection.</span></div>
      <button class="btn sm ghost" onclick={() => tune("system_protection", "Opened System Protection")}>Open</button>
    </div>
    <div class="card tune">
      <span class="ic"><BatteryCharging size={18} /></span>
      <div><b>Power plan</b><span class="faint">“High performance” makes everything snappier at the cost of power.</span></div>
      <select value={plans.find((p) => p.active)?.guid ?? ""} onchange={(e) => setPlan(e.currentTarget.value)}>
        {#each plans as p (p.guid)}<option value={p.guid}>{p.name}</option>{/each}
      </select>
    </div>
  </div>

  <h2><Package size={17} /> Preinstalled apps</h2>
  <p class="faint hint">Apps that came with Windows and most people never open. Removing them only affects your account.</p>
  {#if appx === null}
    <button class="btn" disabled={busy === "appx"} onclick={loadApps}>
      {#if busy === "appx"}<LoaderCircle size={14} class="spin" /> Looking…{:else}Find preinstalled apps{/if}
    </button>
  {:else if appx.length === 0}
    <p class="faint">None of the usual suspects are installed. Clean!</p>
  {:else}
    <div class="card table">
      {#each appx as a (a.fullName)}
        <div class="trow apps">
          <div class="tname"><b>{a.friendly}</b><span class="faint">{a.note}</span></div>
          <button class="btn sm" disabled={busy === a.fullName} onclick={() => removeApp(a)}>Remove</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .speed {
    height: 100%;
    padding: 42px 40px 80px;
  }
  h1 {
    font-size: 28px;
  }
  .lead {
    margin: 6px 0 20px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    margin: 30px 0 4px;
  }
  .hint {
    margin: 0 0 12px;
    font-size: 12.5px;
    max-width: 760px;
  }
  .summary {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }
  .sum {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
  }
  .sum > div:last-child {
    display: flex;
    flex-direction: column;
    font-size: 12.5px;
  }
  .sum b {
    font-size: 14px;
  }
  .ring {
    position: relative;
    display: grid;
    place-items: center;
  }
  .ring span {
    position: absolute;
    font-size: 12.5px;
    font-weight: 700;
  }
  .big {
    font-family: var(--font-display);
    font-size: 34px;
    font-weight: 600;
    min-width: 56px;
    text-align: center;
  }
  .big.warn {
    color: var(--warn);
  }
  .table {
    padding: 4px 0;
  }
  .trow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 100px 100px auto;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }
  .trow:last-child {
    border-bottom: 0;
  }
  .trow.procs,
  .trow.head {
    grid-template-columns: minmax(0, 1fr) 90px 70px 90px 150px;
  }
  .trow.head {
    font-size: 12px;
    padding-top: 6px;
    padding-bottom: 6px;
  }
  .trow.apps {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .tname {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 2px;
  }
  .tname .mono {
    font-size: 11px;
  }
  .tmeta {
    font-size: 12px;
    text-align: right;
  }
  .tact {
    display: flex;
    justify-content: flex-end;
  }
  .chip.tiny {
    height: 22px;
    font-size: 11px;
  }
  .flags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .flag {
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 99px;
  }
  .flag.bad {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .flag.warn {
    background: color-mix(in srgb, var(--warn) 15%, transparent);
    color: var(--warn);
  }
  .small {
    font-size: 11.5px;
  }
  .pad {
    padding: 12px 16px;
    margin: 0;
  }
  .drives {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }
  .drive {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12.5px;
  }
  .drive b {
    font-size: 14px;
  }
  .frag {
    color: var(--text-2);
  }
  .row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .tunes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(440px, 1fr));
    gap: 12px;
  }
  .tune {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
  }
  .tune > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
    min-width: 0;
  }
  .tune b {
    font-size: 14px;
  }
  .ic {
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: var(--accent);
    background: var(--accent-soft);
  }
  select {
    height: 32px;
    padding: 0 10px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    max-width: 180px;
  }
  select option {
    background: var(--bg-solid);
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
