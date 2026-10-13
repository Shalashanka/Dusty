<script lang="ts">
  import { Folder } from "@lucide/svelte";
  import { extStyle } from "$lib/kinds";

  let { ext = "", isDir = false, size = 48, color = "" }: { ext?: string; isDir?: boolean; size?: number; color?: string } =
    $props();
  const st = $derived(extStyle(ext, isDir));
  const c = $derived(color || st.color);
</script>

<div class="glyph" style="--c:{c}; --s:{size}px">
  {#if isDir}
    <Folder size={size * 0.5} strokeWidth={1.8} />
  {:else}
    <span>{st.label}</span>
  {/if}
</div>

<style>
  .glyph {
    width: var(--s);
    height: var(--s);
    flex: none;
    border-radius: calc(var(--s) * 0.28);
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--c) 30%, transparent);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }
  span {
    font-weight: 700;
    font-size: calc(var(--s) * 0.24);
    letter-spacing: 0.02em;
  }
</style>
