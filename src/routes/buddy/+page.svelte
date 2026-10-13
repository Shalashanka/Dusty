<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Mascot, { type Mood } from "$lib/components/Mascot.svelte";
  import { api } from "$lib/api";

  /** A short stroll across the bottom of the screen, then gone. */
  let active = $state(false);
  let message = $state("");
  let mood = $state<Mood>("happy");
  let fromLeft = $state(true);
  let phase = $state<"walk" | "pause" | "leave">("walk");
  let run = $state(0);

  onMount(async () => {
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      (await import("$lib/mock")).installMocks();
      start("Just dusting a few corners…", "sweeping");
    }
    await listen<{ message: string; mood: Mood }>("buddy-stroll", (e) => start(e.payload.message, e.payload.mood));
  });

  function start(msg: string, m: Mood) {
    message = msg;
    mood = m;
    fromLeft = Math.random() < 0.5;
    phase = "walk";
    active = true;
    run++;
    const id = run;
    // walk in (5s) -> stop and say something (5s) -> walk out (5s) -> gone
    setTimeout(() => id === run && (phase = "pause"), 5000);
    setTimeout(() => id === run && (phase = "leave"), 10000);
    setTimeout(() => {
      if (id !== run) return;
      active = false;
      api.buddyDone().catch(() => {});
    }, 15200);
  }
</script>

{#if active}
  {#key run}
    <div class="lane" class:rtl={!fromLeft}>
      <div class="walker {phase}">
        {#if phase === "pause"}
          <div class="bubble">{message}</div>
        {/if}
        <div class="body" class:flip={!fromLeft}>
          <Mascot mood={phase === "pause" ? mood : "sweeping"} size={96} interactive={false} lively={false} />
        </div>
        <div class="puffs">
          {#each [0, 1, 2] as i}<span style="animation-delay:{i * 0.4}s"></span>{/each}
        </div>
      </div>
    </div>
  {/key}
{/if}

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
  }
  .lane {
    position: fixed;
    inset: 0;
    pointer-events: none;
  }
  .walker {
    position: absolute;
    bottom: 4px;
    left: 0;
    width: 120px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  /* In from one side, stop around a third of the way, then out the other side. */
  .walker.walk {
    animation: walkin 5s cubic-bezier(0.25, 0.1, 0.25, 1) both;
  }
  .walker.pause {
    left: 30vw;
  }
  .walker.leave {
    animation: walkout 5.2s cubic-bezier(0.55, 0, 0.75, 0.2) both;
  }
  .rtl .walker {
    left: auto;
    right: 0;
  }
  .rtl .walker.walk {
    animation-name: walkin-r;
  }
  .rtl .walker.pause {
    right: 30vw;
  }
  .rtl .walker.leave {
    animation-name: walkout-r;
  }
  @keyframes walkin {
    from {
      left: -140px;
    }
    to {
      left: 30vw;
    }
  }
  @keyframes walkout {
    from {
      left: 30vw;
    }
    to {
      left: calc(100vw + 20px);
    }
  }
  @keyframes walkin-r {
    from {
      right: -140px;
    }
    to {
      right: 30vw;
    }
  }
  @keyframes walkout-r {
    from {
      right: 30vw;
    }
    to {
      right: calc(100vw + 20px);
    }
  }
  .body {
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.35));
    animation: hop 0.5s ease-in-out infinite;
  }
  .pause .body {
    animation: none;
  }
  .body.flip {
    transform: scaleX(-1);
  }
  @keyframes hop {
    50% {
      translate: 0 -3px;
    }
  }
  .bubble {
    max-width: 260px;
    margin-bottom: 4px;
    padding: 8px 12px;
    border-radius: 14px 14px 14px 4px;
    background: color-mix(in srgb, #16141f 88%, transparent);
    color: #eeecf6;
    border: 1px solid rgba(255, 255, 255, 0.14);
    font: 600 12.5px var(--font);
    white-space: nowrap;
    box-shadow: 0 10px 30px -10px rgba(0, 0, 0, 0.6);
    animation: pop 0.35s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.9);
    }
  }
  .puffs {
    position: absolute;
    bottom: 6px;
    left: 30px;
  }
  .pause .puffs {
    display: none;
  }
  .puffs span {
    position: absolute;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: rgba(201, 194, 255, 0.55);
    animation: puff 1.2s ease-out infinite;
  }
  .rtl .puffs {
    left: auto;
    right: 30px;
  }
  @keyframes puff {
    from {
      opacity: 0.8;
      transform: translate(0, 0) scale(0.5);
    }
    to {
      opacity: 0;
      transform: translate(-16px, -14px) scale(1.6);
    }
  }
</style>
