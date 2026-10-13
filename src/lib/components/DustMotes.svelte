<script lang="ts">
  /** Slow, faint dust motes drifting in the light. Purely decorative. */
  const motes = Array.from({ length: 14 }, (_, i) => ({
    left: (i * 37) % 100,
    size: 2 + ((i * 7) % 4),
    dur: 18 + ((i * 13) % 14),
    delay: -((i * 5) % 20),
    drift: ((i % 3) - 1) * 30,
  }));
</script>

<div class="motes" aria-hidden="true">
  {#each motes as m, i (i)}
    <span style="left:{m.left}%;width:{m.size}px;height:{m.size}px;animation-duration:{m.dur}s;animation-delay:{m.delay}s;--drift:{m.drift}px"></span>
  {/each}
</div>

<style>
  .motes {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 0;
  }
  span {
    position: absolute;
    bottom: -10px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(215, 207, 255, 0.9), rgba(215, 207, 255, 0));
    opacity: 0;
    animation: float linear infinite;
  }
  @keyframes float {
    0% {
      transform: translate(0, 0);
      opacity: 0;
    }
    15% {
      opacity: 0.35;
    }
    85% {
      opacity: 0.25;
    }
    100% {
      transform: translate(var(--drift), -105vh);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .motes {
      display: none;
    }
  }
</style>
