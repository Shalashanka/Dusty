<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let { value, size = 64, stroke = 7, color = "var(--accent)" }: { value: number; size?: number; stroke?: number; color?: string } =
    $props();
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const t = Tween.of(() => Math.max(0, Math.min(1, value)), { duration: 1100, easing: cubicOut });
</script>

<svg width={size} height={size} viewBox="0 0 {size} {size}">
  <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--surface-3)" stroke-width={stroke} />
  <circle
    cx={size / 2}
    cy={size / 2}
    {r}
    fill="none"
    stroke={color}
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-dasharray={c}
    stroke-dashoffset={c * (1 - t.current)}
    transform="rotate(-90 {size / 2} {size / 2})"
  />
</svg>
