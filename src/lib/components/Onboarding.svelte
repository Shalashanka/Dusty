<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { ArrowRight, ArrowLeft, ShieldCheck, Moon, Hand, FolderPlus, X, Check, Download, Monitor, Code, Search, Turtle, Scale, Rocket } from "@lucide/svelte";
  import Mascot, { type Mood } from "./Mascot.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, ONBOARDING_VERSION } from "$lib/app.svelte";
  import { api, type Kind, type Mode, type PolicyMode, type Settings } from "$lib/api";

  let step = $state(0);
  let s = $state<Settings>(structuredClone($state.snapshot(app.settings!) as Settings));
  let style = $state<"careful" | "balanced" | "bold">("balanced");
  let hotkeyTried = $state(false);
  const STEPS = 5;

  const moods: Mood[] = ["wave", "curious", "idle", "happy", "sweeping"];

  const downloads = $derived(s.inboxDirs[0] ?? "");
  const downloadsMode = $derived(s.policies.find((p) => p.path === downloads)?.mode ?? "watch");
  const desktopMode = $derived(s.policies.find((p) => p.path === s.desktopDir)?.mode ?? "watch");

  function setPolicy(path: string, mode: PolicyMode) {
    if (!path) return;
    const others = s.policies.filter((p) => p.path !== path);
    s.policies = [...others, { path, mode, graceHours: 48, askEveryDays: 3 }];
  }

  async function addFolder(key: "devRoots" | "watchedDirs") {
    const p = await api.pickFolder();
    if (p && !s[key].includes(p)) s[key] = [...s[key], p];
  }

  function applyStyle() {
    const autoKinds: Kind[] =
      style === "bold" ? ["system_junk", "extracted_archive", "empty_folder", "dev_junk"] : style === "balanced" ? ["system_junk"] : [];
    const rules: Partial<Record<Kind, Mode>> = { ...s.rules };
    for (const k of Object.keys(rules) as Kind[]) if (rules[k] === "auto") rules[k] = "ask";
    for (const k of autoKinds) rules[k] = "auto";
    s.rules = rules;
    s.nudgeGapHours = style === "careful" ? 24 : style === "balanced" ? 6 : 3;
  }

  async function finish() {
    applyStyle();
    s.onboarded = true;
    s.seenVersion = ONBOARDING_VERSION;
    await app.saveSettings(s);
    await api.scanNow();
    await app.refresh();
  }

  function onKey(e: KeyboardEvent) {
    // Pressing the search hotkey here proves it works.
    const combo = (s.searchHotkey || "").toLowerCase();
    if (step === 3 && combo.includes("space") && e.code === "Space" && e.ctrlKey && e.altKey) hotkeyTried = true;
    if (e.key === "Enter" && step < STEPS - 1) step++;
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="overlay" out:fade={{ duration: 300 }}>
  <div class="dots">
    {#each Array(STEPS) as _, i}<span class:on={i === step} class:done={i < step}></span>{/each}
  </div>

  <div class="stage">
    <div class="m">
      {#key step}<div in:fly={{ y: 16, duration: 500, easing: cubicOut }}><Mascot mood={moods[step]} size={step === 0 ? 200 : 140} /></div>{/key}
    </div>

    {#key step}
      <div class="panel" in:fly={{ x: 30, duration: 420, easing: cubicOut }}>
        {#if step === 0}
          <h1>Hi, I'm Dusty!</h1>
          <p class="muted">
            I'm a small housekeeper who lives in your tray. While you work, I quietly look around, and when something starts gathering dust
            (an old installer, a forgotten zip, a project you abandoned) I'll give you a gentle nudge.
          </p>
          <ul class="promises">
            <li><ShieldCheck size={17} /> Nothing leaves without your OK, and almost everything can be undone.</li>
            <li><Moon size={17} /> I work at the lowest priority and never ping you while you're gaming or presenting.</li>
            <li><Hand size={17} /> Trust me more over time, and I'll handle the boring stuff on my own.</li>
          </ul>
        {:else if step === 1}
          <h1>How strict should I be?</h1>
          <p class="muted">Pick a rule for the two messiest places. You can change it any time.</p>
          <div class="choice">
            <div class="ch-head"><Download size={18} /><b>Downloads</b></div>
            <div class="opts">
              {#each [["watch", "Nudge me about old stuff", "I'll point out things nobody opened in a while."], ["zero", "Keep it empty", "Everything gets a proper home. I'll draw up a plan you approve."]] as [m, t, d] (m)}
                <button class="opt" class:on={downloadsMode === m} onclick={() => setPolicy(downloads, m as PolicyMode)}>
                  <span class="radio">{#if downloadsMode === m}<Check size={12} />{/if}</span>
                  <span><b>{t}</b><span class="faint">{d}</span></span>
                </button>
              {/each}
            </div>
          </div>
          <div class="choice">
            <div class="ch-head"><Monitor size={18} /><b>Desktop</b></div>
            <div class="opts">
              {#each [["watch", "Sweep it when it's crowded", "Loose files go into a dated folder, nothing deleted."], ["sorted", "Keep it sorted", "Files go into category folders on the desktop."], ["zero", "Keep it clean", "Only shortcuts stay. I'll plan where the rest goes."]] as [m, t, d] (m)}
                <button class="opt" class:on={desktopMode === m} onclick={() => setPolicy(s.desktopDir, m as PolicyMode)}>
                  <span class="radio">{#if desktopMode === m}<Check size={12} />{/if}</span>
                  <span><b>{t}</b><span class="faint">{d}</span></span>
                </button>
              {/each}
            </div>
          </div>
        {:else if step === 2}
          <h1>Anything else I should watch?</h1>
          <p class="muted">I already know your Downloads, Desktop and personal folders. Tell me where your code lives and any other folder that tends to pile up.</p>
          <div class="choice">
            <div class="ch-head"><Code size={18} /><b>Project folders</b><button class="btn sm" onclick={() => addFolder("devRoots")}><FolderPlus size={13} /> Add</button></div>
            {#each s.devRoots as p (p)}
              <div class="fitem"><span class="mono ellipsis">{p}</span><button class="x" onclick={() => (s.devRoots = s.devRoots.filter((x) => x !== p))}><X size={13} /></button></div>
            {:else}<span class="faint small">None yet.</span>{/each}
          </div>
          <div class="choice">
            <div class="ch-head"><FolderPlus size={18} /><b>Extra folders to watch</b><button class="btn sm" onclick={() => addFolder("watchedDirs")}><FolderPlus size={13} /> Add</button></div>
            {#each s.watchedDirs as p (p)}
              <div class="fitem"><span class="mono ellipsis">{p}</span><button class="x" onclick={() => (s.watchedDirs = s.watchedDirs.filter((x) => x !== p))}><X size={13} /></button></div>
            {:else}<span class="faint small">Optional.</span>{/each}
          </div>
        {:else if step === 3}
          <h1>Dusty at your fingertips</h1>
          <p class="muted">
            I index all your drives in the background, so you can search everything from anywhere. Just ask like you'd ask a friend:
            <i>“that pdf with invoice from about 5 months ago”</i>.
          </p>
          <div class="hotkey" class:ok={hotkeyTried}>
            <Search size={20} />
            <div>
              <b>Press <span class="kbd">{s.searchHotkey}</span> anywhere</b>
              <span class="faint">{hotkeyTried ? "That's the one! (It works everywhere once we're done.)" : "Try it now: you'll use it a lot."}</span>
            </div>
            {#if hotkeyTried}<Check size={20} class="okc" />{/if}
          </div>
          <div class="keys">
            <div><span class="kbd">{s.paletteHotkey}</span><span>Commands: clean, pause, open… without opening Dusty</span></div>
            <div><span class="kbd">{s.voiceHotkey}</span><span>Ask out loud (turn on voice in Settings; runs 100% on your PC)</span></div>
          </div>
          <div class="srow"><div><b>Index my drives</b><span class="faint">Takes a minute in the background, ~100 MB of memory.</span></div><Toggle bind:checked={s.indexEnabled} /></div>
        {:else}
          <h1>Last thing: how hands-on are you?</h1>
          <p class="muted">This decides how much I'm allowed to do by myself. You can fine-tune each rule later.</p>
          <div class="styles">
            {#each [
              { id: "careful", icon: Turtle, t: "Careful", d: "Ask me about everything. Ping me at most once a day." },
              { id: "balanced", icon: Scale, t: "Balanced", d: "Clear temp files and caches on my own; ask about the rest." },
              { id: "bold", icon: Rocket, t: "Bold", d: "Also clear build junk, extracted zips and empty folders by myself." },
            ] as o (o.id)}
              {@const I = o.icon}
              <button class="style" class:on={style === o.id} onclick={() => (style = o.id as typeof style)}>
                <I size={22} />
                <b>{o.t}</b>
                <span class="faint">{o.d}</span>
              </button>
            {/each}
          </div>
          <div class="srow"><div><b>Start with Windows</b><span class="faint">I stay hidden in the tray.</span></div><Toggle bind:checked={s.startWithWindows} /></div>
        {/if}
      </div>
    {/key}
  </div>

  <div class="nav">
    {#if step > 0}<button class="btn ghost lg" onclick={() => step--}><ArrowLeft size={18} /> Back</button>{:else}<span></span>{/if}
    {#if step < STEPS - 1}
      <button class="btn primary lg" onclick={() => step++}>{step === 0 ? "Nice to meet you" : "Next"} <ArrowRight size={18} /></button>
    {:else}
      <button class="btn primary lg" onclick={finish}>Start looking around <ArrowRight size={18} /></button>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 450;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 50px 40px 34px;
    background:
      radial-gradient(70% 60% at 30% 30%, rgba(139, 124, 255, 0.16), transparent 70%),
      radial-gradient(60% 50% at 80% 90%, rgba(79, 227, 193, 0.08), transparent 70%),
      var(--bg-solid);
  }
  .dots {
    display: flex;
    gap: 8px;
  }
  .dots span {
    width: 8px;
    height: 8px;
    border-radius: 99px;
    background: var(--surface-3);
    transition: all 0.4s var(--ease);
  }
  .dots span.on {
    width: 28px;
    background: var(--accent);
  }
  .dots span.done {
    background: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .stage {
    flex: 1;
    width: min(880px, 100%);
    display: grid;
    grid-template-columns: 220px 1fr;
    align-items: center;
    gap: 30px;
  }
  .m {
    display: flex;
    justify-content: center;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h1 {
    font-size: 30px;
  }
  p {
    margin: 0;
    font-size: 15px;
    line-height: 1.55;
  }
  .promises {
    list-style: none;
    padding: 0;
    margin: 8px 0 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .promises li {
    display: flex;
    gap: 10px;
    color: var(--text-2);
  }
  .promises :global(svg) {
    flex: none;
    color: var(--accent-2);
    margin-top: 2px;
  }
  .choice {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .ch-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ch-head .btn {
    margin-left: auto;
  }
  .opts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 8px;
  }
  .opt {
    display: flex;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--surface-2);
    border: 1px solid var(--border);
    text-align: left;
    font-size: 12.5px;
    transition: all 0.2s;
  }
  .opt > span:last-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .opt b {
    font-size: 13.5px;
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .radio {
    width: 18px;
    height: 18px;
    flex: none;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    display: grid;
    place-items: center;
    margin-top: 1px;
  }
  .opt.on .radio {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .fitem {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 10px;
    border-radius: 9px;
    background: var(--surface-2);
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
  .small {
    font-size: 12px;
  }
  .hotkey {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 18px;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    transition: all 0.3s;
  }
  .hotkey > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
  }
  .hotkey b {
    font-size: 15px;
  }
  .hotkey.ok {
    border-style: solid;
    border-color: var(--ok);
    background: var(--ok-soft);
  }
  .hotkey :global(.okc) {
    color: var(--ok);
  }
  .keys {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }
  .keys div {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .keys .kbd {
    min-width: 110px;
    justify-content: center;
  }
  .srow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 16px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .srow > div {
    display: flex;
    flex-direction: column;
    font-size: 12.5px;
  }
  .styles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .style {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    font-size: 12.5px;
    transition: all 0.2s;
  }
  .style b {
    font-size: 15px;
  }
  .style.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    transform: translateY(-2px);
  }
  .style :global(svg) {
    color: var(--accent);
  }
  .nav {
    width: min(880px, 100%);
    display: flex;
    justify-content: space-between;
  }
</style>
