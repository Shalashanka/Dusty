<script lang="ts">
  import { onMount } from "svelte";
  import { scale, fade } from "svelte/transition";
  import { backOut } from "svelte/easing";

  /** Dusty muses now and then: shown for a few seconds, then quiet for a while. */
  let { lines }: { lines: string[] } = $props();
  let current = $state<string | null>(null);
  let i = 0;

  onMount(() => {
    let t: ReturnType<typeof setTimeout>;
    const next = () => {
      if (lines.length) {
        current = lines[i % lines.length];
        i++;
        t = setTimeout(() => {
          current = null;
          t = setTimeout(next, 9000 + Math.random() * 8000);
        }, 6500);
      } else {
        t = setTimeout(next, 4000);
      }
    };
    t = setTimeout(next, 2500);
    return () => clearTimeout(t);
  });
</script>

{#if current}
  {#key current}
    <div class="bubble" in:scale={{ start: 0.6, duration: 380, easing: backOut }} out:fade={{ duration: 250 }}>
      {current}
      <i class="dot a"></i><i class="dot b"></i>
    </div>
  {/key}
{/if}

<style>
  .bubble {
    position: absolute;
    left: 4%;
    bottom: 90%;
    z-index: 3;
    width: max-content;
    max-width: 250px;
    padding: 9px 13px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-solid) 88%, white 12%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 12px 30px -12px rgba(0, 0, 0, 0.6);
    font-size: 12.5px;
    font-weight: 500;
    line-height: 1.35;
    transform-origin: bottom left;
    pointer-events: none;
  }
  .dot {
    position: absolute;
    border-radius: 50%;
    background: inherit;
    border: inherit;
  }
  .dot.a {
    width: 10px;
    height: 10px;
    left: 4px;
    bottom: -12px;
  }
  .dot.b {
    width: 6px;
    height: 6px;
    left: -4px;
    bottom: -20px;
  }
</style>
