<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { bytesParts } from "$lib/format";

  let { value, kind = "number" }: { value: number; kind?: "number" | "bytes" } = $props();

  const t = Tween.of(() => value, { duration: 900, easing: cubicOut });
  const parts = $derived(kind === "bytes" ? bytesParts(t.current) : [Math.round(t.current).toLocaleString(), ""]);
</script>

<span class="tabular">{parts[0]}{#if parts[1]}<small>&nbsp;{parts[1]}</small>{/if}</span>

<style>
  small {
    font-size: 0.6em;
    font-weight: 500;
    opacity: 0.7;
    letter-spacing: 0;
  }
</style>
