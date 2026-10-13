<script lang="ts">
  import { scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { menu } from "$lib/menu.svelte";

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    if (!menu.open) return;
    // Keep the menu inside the window.
    const w = el?.offsetWidth ?? 240;
    const h = el?.offsetHeight ?? 300;
    pos = {
      x: Math.min(menu.x, window.innerWidth - w - 8),
      y: Math.min(menu.y, window.innerHeight - h - 8),
    };
  });
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && menu.close()}
  onblur={() => menu.close()}
  onresize={() => menu.close()}
/>

{#if menu.open}
  <div class="veil" role="presentation" onclick={() => menu.close()} oncontextmenu={(e) => { e.preventDefault(); menu.close(); }}></div>
  <div
    bind:this={el}
    class="menu"
    style="left:{pos.x}px; top:{pos.y}px"
    role="menu"
    transition:scale={{ start: 0.95, duration: 140, easing: cubicOut }}
  >
    {#each menu.items as it, i (i)}
      {#if it.divider}
        <div class="div"></div>
      {:else if it.caption}
        <div class="cap">{it.caption}</div>
      {:else}
        <button
          class="item"
          class:danger={it.danger}
          disabled={it.disabled}
          title={it.hint}
          role="menuitem"
          onclick={() => {
            menu.close();
            it.run?.();
          }}
        >
          {#if it.icon}{@const I = it.icon}<I size={15} />{:else}<span class="sp"></span>{/if}
          <span>{it.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 500;
  }
  .menu {
    position: fixed;
    z-index: 501;
    min-width: 220px;
    max-width: 320px;
    padding: 5px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-solid) 94%, white 6%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 20px 50px -10px rgba(0, 0, 0, 0.6);
    transform-origin: top left;
    backdrop-filter: blur(20px);
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    text-align: left;
    font-size: 13px;
    color: var(--text);
  }
  .item:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger:hover {
    background: var(--danger-soft);
  }
  .item:disabled {
    opacity: 0.4;
  }
  .sp {
    width: 15px;
  }
  .div {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
  .cap {
    padding: 6px 10px 3px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
</style>
