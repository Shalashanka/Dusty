<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import Mascot, { type Mood } from "./Mascot.svelte";
  import ThoughtBubble from "./ThoughtBubble.svelte";
  import WalkSprite from "./WalkSprite.svelte";

  /**
   * Dusty has a life of their own on Home: usually here, sometimes off cleaning
   * somewhere (the spot is empty, with a note), and now and then sweeping past
   * far in the background. Click the empty spot to call Dusty back.
   *
   * The distant pass is positioned against the nearest positioned ancestor
   * (the hero), so it can cross the whole width.
   */
  let { mood, thoughts, size = 180 }: { mood: Mood; thoughts: string[]; size?: number } = $props();

  type Where = "here" | "leaving" | "away" | "back";
  let where = $state<Where>("here");
  let pass = $state<{ id: number; toRight: boolean } | null>(null);
  let greet = $state(false);
  const sizeStyle = $derived(`width:${size}px;height:${size}px`);
  const AWAY_NOTES = [
    "Out dusting the Downloads…",
    "Checking behind the sofa…",
    "Sweeping the hallway…",
    "Polishing the Recycle Bin…",
    "Fluffing the folders…",
  ];
  let note = $state(AWAY_NOTES[0]);

  const timers = new Set<ReturnType<typeof setTimeout>>();
  function later(fn: () => void, ms: number) {
    const t = setTimeout(() => {
      timers.delete(t);
      fn();
    }, ms);
    timers.add(t);
  }
  const rand = (a: number, b: number) => a + Math.random() * (b - a);

  function leave() {
    if (where !== "here") return;
    note = AWAY_NOTES[Math.floor(Math.random() * AWAY_NOTES.length)];
    where = "leaving";
    later(() => {
      where = "away";
      // Most of the time, Dusty is spotted sweeping past in the distance.
      if (Math.random() < 0.65) later(startPass, rand(3000, 9000));
      later(comeBack, rand(35000, 75000));
    }, 900);
  }

  function startPass() {
    if (where !== "away" || pass) return;
    const p = { id: Date.now(), toRight: Math.random() < 0.5 };
    pass = p;
    later(() => pass?.id === p.id && (pass = null), 26500);
  }

  function comeBack() {
    if (where !== "away") return;
    pass = null;
    where = "back";
    greet = true;
    later(() => (where = "here"), 700);
    later(() => (greet = false), 2200);
  }

  onMount(() => {
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    // Always present when you open the app; after that, now and then, off to work.
    const roll = () => {
      if (where === "here" && !document.hidden && Math.random() < 0.22) leave();
      later(roll, rand(40000, 70000));
    };
    later(roll, rand(45000, 80000));
    return () => timers.forEach(clearTimeout);
  });
</script>

<div class="slot" style={sizeStyle}>
  {#if where === "here" || where === "back" || where === "leaving"}
    <div class="dusty {where}">
      {#if where === "here"}<ThoughtBubble lines={thoughts} />{/if}
      <Mascot mood={greet ? "wave" : where === "leaving" ? "sweeping" : mood} {size} />
    </div>
  {:else}
    <button class="away" in:fade={{ duration: 500 }} onclick={comeBack} title="Call Dusty back">
      <span class="steps" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
      <span class="note">{note}</span>
      <span class="call">back soon · click to call</span>
    </button>
  {/if}
</div>

{#if pass}
  {#key pass.id}
    <div class="distance" class:rtl={!pass.toRight} aria-hidden="true">
      <WalkSprite height={52} flip={!pass.toRight} fps={7} />
    </div>
  {/key}
{/if}

<style>
  .slot {
    position: relative;
    display: grid;
    place-items: center;
  }
  .dusty {
    position: relative;
  }
  .dusty.leaving {
    animation: off 0.9s cubic-bezier(0.5, 0, 0.75, 0) both;
  }
  .dusty.back {
    animation: on 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  @keyframes off {
    to {
      opacity: 0;
      transform: translateX(-46px) scale(0.92);
      filter: blur(2px);
    }
  }
  @keyframes on {
    from {
      opacity: 0;
      transform: translateX(46px) scale(0.92);
      filter: blur(2px);
    }
  }
  .away {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: 50%;
    color: var(--text-2);
    transition: background 0.3s;
  }
  .away:hover {
    background: radial-gradient(circle, var(--surface-1), transparent 70%);
  }
  .note {
    font-size: 12.5px;
    font-weight: 600;
    text-align: center;
    max-width: 150px;
  }
  .call {
    font-size: 11px;
    color: var(--text-3);
  }
  /* Little dusty footprints leading off the spot. */
  .steps {
    position: relative;
    width: 70px;
    height: 22px;
    margin-bottom: 4px;
  }
  .steps i {
    position: absolute;
    width: 8px;
    height: 5px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 40%, transparent);
    animation: fadeStep 3s ease-in-out infinite;
  }
  .steps i:nth-child(1) {
    left: 56px;
    top: 14px;
  }
  .steps i:nth-child(2) {
    left: 40px;
    top: 6px;
    animation-delay: 0.3s;
  }
  .steps i:nth-child(3) {
    left: 24px;
    top: 14px;
    animation-delay: 0.6s;
  }
  .steps i:nth-child(4) {
    left: 8px;
    top: 6px;
    animation-delay: 0.9s;
  }
  @keyframes fadeStep {
    0%,
    100% {
      opacity: 0.2;
    }
    40% {
      opacity: 0.8;
    }
  }
  /* Far away: small, soft, a little hazy, behind everything. */
  .distance {
    position: absolute;
    left: 0;
    bottom: -14px;
    z-index: 0;
    pointer-events: none;
    opacity: 0.5;
    filter: blur(0.5px) saturate(0.85);
    animation: far 26s linear both;
  }
  .distance.rtl {
    animation-name: far-r;
  }
  @keyframes far {
    from {
      transform: translateX(-80px);
      opacity: 0;
    }
    6% {
      opacity: 0.5;
    }
    94% {
      opacity: 0.5;
    }
    to {
      transform: translateX(calc(100cqw + 20px));
      opacity: 0;
    }
  }
  @keyframes far-r {
    from {
      transform: translateX(calc(100cqw + 20px));
      opacity: 0;
    }
    6% {
      opacity: 0.5;
    }
    94% {
      opacity: 0.5;
    }
    to {
      transform: translateX(-80px);
      opacity: 0;
    }
  }
</style>
