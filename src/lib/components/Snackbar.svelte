<script lang="ts">
  import { fly } from "svelte/transition";
  import { backOut } from "svelte/easing";
  import { Undo2, Check, CircleAlert } from "@lucide/svelte";
  import { app } from "$lib/app.svelte";
</script>

{#if app.snack}
  {#key app.snack.id}
    <div class="snack {app.snack.tone ?? ''}" in:fly={{ y: 30, duration: 380, easing: backOut }} out:fly={{ y: 20, duration: 200 }}>
      {#if app.snack.tone === "error"}<CircleAlert size={16} />{:else}<Check size={16} />{/if}
      <span class="ellipsis">{app.snack.text}</span>
      {#if app.snack.undo}
        <button
          class="undo"
          onclick={() => {
            const u = app.snack?.undo;
            app.snack = null;
            u?.();
          }}><Undo2 size={14} /> Undo <span class="kbd">Z</span></button
        >
      {/if}
    </div>
  {/key}
{/if}

<style>
  .snack {
    position: fixed;
    left: 50%;
    bottom: 24px;
    translate: -50% 0;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(640px, 80vw);
    padding: 10px 10px 10px 16px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-solid) 92%, white 8%);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    backdrop-filter: blur(20px);
  }
  .snack.ok :global(svg:first-child) {
    color: var(--ok);
  }
  .snack.error :global(svg:first-child) {
    color: var(--danger);
  }
  .undo {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 6px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    font-weight: 600;
    color: var(--accent);
  }
  .undo:hover {
    background: var(--surface-3);
  }
</style>
