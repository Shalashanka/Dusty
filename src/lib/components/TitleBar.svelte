<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, X } from "@lucide/svelte";

  const win = () => getCurrentWindow();
</script>

<header class="bar" data-tauri-drag-region>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="controls">
    <button aria-label="Minimize" onclick={() => win().minimize()}><Minus size={15} strokeWidth={1.6} /></button>
    <button aria-label="Maximize" onclick={() => win().toggleMaximize()}><Square size={12} strokeWidth={1.6} /></button>
    <button aria-label="Hide to tray" class="close" onclick={() => win().hide()}><X size={16} strokeWidth={1.6} /></button>
  </div>
</header>

<style>
  /* Content scrolls underneath; the gradient keeps the controls readable. */
  .bar {
    position: absolute;
    inset: 0 0 auto 84px;
    height: 46px;
    display: flex;
    z-index: 400;
    background: linear-gradient(
      to bottom,
      var(--bg-solid) 0%,
      color-mix(in srgb, var(--bg-solid) 85%, transparent) 45%,
      transparent 100%
    );
  }
  .spacer {
    flex: 1;
  }
  .controls {
    display: flex;
    align-self: flex-start;
  }
  .controls button {
    width: 46px;
    height: 34px;
    display: grid;
    place-items: center;
    color: var(--text-2);
    transition:
      background 0.15s,
      color 0.15s;
  }
  .controls button:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .controls .close:hover {
    background: #e81123;
    color: #fff;
  }
</style>
