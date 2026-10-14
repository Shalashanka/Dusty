<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import WalkSprite from "$lib/components/WalkSprite.svelte";
  import { api } from "$lib/api";

  /**
   * One calm pass across the bottom of the screen: Dusty walks in from one edge,
   * sweeping, says one thing on the way, and walks out the other edge. The window
   * is shown only once Dusty is parked off-screen, so nothing ever "pops" in.
   */
  const CROSS_MS = 22000;
  let active = $state(false);
  let message = $state("");
  let toRight = $state(true);
  let talking = $state(false);
  let run = $state(0);
  const isTauri = "__TAURI_INTERNALS__" in window;

  onMount(async () => {
    if (import.meta.env.DEV && !isTauri) {
      (await import("$lib/mock")).installMocks();
      start("Just dusting a few corners…");
    }
    await listen<{ message: string }>("buddy-stroll", (e) => start(e.payload.message));
  });

  async function start(msg: string) {
    message = msg;
    toRight = Math.random() < 0.5;
    talking = false;
    active = true;
    const id = ++run;
    await tick();
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    if (isTauri) await getCurrentWebviewWindow().show().catch(() => {});
    // Say something while passing the middle of the screen.
    setTimeout(() => id === run && (talking = true), CROSS_MS * 0.35);
    setTimeout(() => id === run && (talking = false), CROSS_MS * 0.62);
    setTimeout(() => {
      if (id !== run) return;
      active = false;
      api.buddyDone().catch(() => {});
    }, CROSS_MS + 300);
  }
</script>

{#if active}
  {#key run}
    <div class="walker" class:rtl={!toRight} style="--dur:{CROSS_MS}ms">
      {#if talking}
        <div class="bubble">{message}</div>
      {/if}
      <WalkSprite height={104} flip={!toRight} fps={8} />
      <div class="puffs" class:rtl={!toRight}>
        {#each [0, 1, 2] as i}<span style="animation-delay:{i * 0.45}s"></span>{/each}
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
  .walker {
    position: fixed;
    bottom: 2px;
    left: 0;
    width: 104px;
    display: flex;
    flex-direction: column;
    align-items: center;
    pointer-events: none;
    animation: cross var(--dur) linear both;
  }
  .walker.rtl {
    animation-name: cross-r;
  }
  @keyframes cross {
    from {
      transform: translateX(-140px);
    }
    to {
      transform: translateX(calc(100vw + 40px));
    }
  }
  @keyframes cross-r {
    from {
      transform: translateX(calc(100vw + 40px));
    }
    to {
      transform: translateX(-140px);
    }
  }
  .bubble {
    position: absolute;
    bottom: 100%;
    max-width: 280px;
    margin-bottom: 2px;
    padding: 8px 12px;
    border-radius: 14px;
    background: color-mix(in srgb, #16141f 90%, transparent);
    color: #eeecf6;
    border: 1px solid rgba(255, 255, 255, 0.14);
    font: 600 12.5px var(--font);
    white-space: nowrap;
    box-shadow: 0 10px 30px -10px rgba(0, 0, 0, 0.6);
    animation: pop 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.9);
    }
  }
  .puffs {
    position: absolute;
    bottom: 8px;
    right: 6px;
  }
  .puffs.rtl {
    right: auto;
    left: 6px;
  }
  .puffs span {
    position: absolute;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: rgba(240, 220, 170, 0.45);
    animation: puff 1.3s ease-out infinite;
  }
  @keyframes puff {
    from {
      opacity: 0.8;
      transform: translate(0, 0) scale(0.5);
    }
    to {
      opacity: 0;
      transform: translate(-18px, -12px) scale(1.7);
    }
  }
  .rtl .puffs span {
    animation-name: puff-r;
  }
  @keyframes puff-r {
    to {
      opacity: 0;
      transform: translate(18px, -12px) scale(1.7);
    }
  }
</style>
