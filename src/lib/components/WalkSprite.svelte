<script lang="ts">
  import walk from "$lib/assets/mascot/walk.webp";

  /** Dusty's painted walk-and-sweep cycle (8 frames, facing right). */
  let {
    height = 120,
    flip = false,
    playing = true,
    fps = 9,
  }: { height?: number; flip?: boolean; playing?: boolean; fps?: number } = $props();

  const FRAMES = 8;
  const CELL_W = 324;
  const CELL_H = 324;
  const width = $derived((height * CELL_W) / CELL_H);
</script>

<div
  class="sprite"
  class:flip
  class:paused={!playing}
  style="--w:{width}px; --h:{height}px; --dur:{FRAMES / fps}s; background-image:url({walk})"
  aria-hidden="true"
></div>

<style>
  .sprite {
    width: var(--w);
    height: var(--h);
    background-repeat: no-repeat;
    background-size: calc(var(--w) * 8) var(--h);
    animation: cycle var(--dur) steps(8) infinite;
    filter: drop-shadow(0 8px 10px rgba(0, 0, 0, 0.35));
  }
  .flip {
    transform: scaleX(-1);
  }
  .paused {
    animation-play-state: paused;
  }
  @keyframes cycle {
    from {
      background-position: 0 0;
    }
    to {
      background-position: calc(var(--w) * -8) 0;
    }
  }
</style>
