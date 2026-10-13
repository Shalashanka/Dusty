<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { cubicOut, backOut } from "svelte/easing";
  import { Check, ArrowRight, ShieldCheck, Moon, Hand } from "@lucide/svelte";
  import Mascot from "./Mascot.svelte";
  import Toggle from "./Toggle.svelte";
  import { app } from "$lib/app.svelte";
  import { api } from "$lib/api";

  let step = $state(0);
  const s = $derived(app.settings!);
  const places = $derived([
    ...s.inboxDirs.map((p) => ({ p, what: "new arrivals" })),
    ...(s.desktopDir ? [{ p: s.desktopDir, what: "desktop piles" }] : []),
    ...s.devRoots.map((p) => ({ p, what: "idle projects" })),
    { p: "Temp folders & caches", what: "junk" },
  ]);

  async function finish() {
    await app.saveSettings({ ...s, onboarded: true });
    await api.scanNow();
    await app.refresh();
  }
</script>

<div class="overlay" out:fade={{ duration: 300 }}>
  {#key step}
    <div class="panel" in:fly={{ y: 24, duration: 500, easing: cubicOut }}>
      {#if step === 0}
        <div class="m" in:fly={{ y: 30, duration: 700, easing: backOut }}><Mascot mood="happy" size={190} /></div>
        <h1>Hi, I'm Dusty.</h1>
        <p class="muted">
          I live in your tray and quietly look around when you step away. When something starts gathering dust, like an old installer or a
          forgotten zip, I'll give you a little nudge.
        </p>
        <ul class="promises">
          <li><ShieldCheck size={17} /> Nothing leaves without your OK. Everything goes to the Recycle Bin, with undo.</li>
          <li><Moon size={17} /> I work at the lowest priority and never ping you while you're gaming or presenting.</li>
          <li><Hand size={17} /> Trust me more later and I'll handle boring stuff on autopilot.</li>
        </ul>
        <button class="btn primary lg" onclick={() => (step = 1)}>Nice to meet you <ArrowRight size={18} /></button>
      {:else}
        <div class="m"><Mascot mood="curious" size={130} /></div>
        <h1>Here's where I'll look</h1>
        <p class="muted">You can change any of this later in Rules.</p>
        <div class="places">
          {#each places as pl, i (pl.p)}
            <div class="place" in:fly={{ x: -16, duration: 400, delay: 120 + i * 80, easing: cubicOut }}>
              <span class="tick"><Check size={14} /></span>
              <span class="ellipsis mono">{pl.p}</span>
              <span class="faint">{pl.what}</span>
            </div>
          {/each}
        </div>
        <div class="startup">
          <div>
            <b>Start with Windows</b>
            <span class="faint">So I can keep an eye out. I stay hidden in the tray.</span>
          </div>
          <Toggle checked={s.startWithWindows} onchange={(v) => app.saveSettings({ ...s, startWithWindows: v })} />
        </div>
        <button class="btn primary lg" onclick={finish}>Start looking around <ArrowRight size={18} /></button>
      {/if}
    </div>
  {/key}
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: grid;
    place-items: center;
    background: radial-gradient(80% 80% at 50% 30%, rgba(139, 124, 255, 0.18), transparent 70%), var(--bg-solid);
  }
  .panel {
    width: min(560px, 90vw);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
  }
  .m {
    margin-bottom: 4px;
  }
  h1 {
    font-size: 32px;
  }
  p {
    margin: 0 0 8px;
    font-size: 15px;
  }
  .promises {
    list-style: none;
    padding: 0;
    margin: 4px 0 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    text-align: left;
  }
  .promises li {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    color: var(--text-2);
  }
  .promises :global(svg) {
    flex: none;
    color: var(--accent-2);
    margin-top: 1px;
  }
  .places {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 4px 0 10px;
  }
  .place {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    align-items: center;
    padding: 10px 14px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    font-size: 13px;
  }
  .tick {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--ok-soft);
    color: var(--ok);
  }
  .startup {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    margin-bottom: 12px;
  }
  .startup div {
    display: flex;
    flex-direction: column;
    font-size: 13px;
  }
</style>
