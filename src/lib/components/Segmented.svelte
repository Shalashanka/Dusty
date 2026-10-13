<script lang="ts" generics="T extends string">
  let {
    value,
    options,
    onchange,
  }: { value: T; options: { value: T; label: string; title?: string }[]; onchange: (v: T) => void } = $props();
  const idx = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
</script>

<div class="seg" style="--n:{options.length}; --i:{idx}">
  <span class="thumb {value}"></span>
  {#each options as o (o.value)}
    <button class:on={o.value === value} title={o.title} onclick={() => onchange(o.value)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    padding: 3px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    min-width: 230px;
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--n));
    border-radius: 999px;
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    transform: translateX(calc(var(--i) * 100%));
    transition: transform 0.4s var(--spring), background 0.3s;
  }
  .thumb.auto {
    background: var(--grad);
    border-color: transparent;
  }
  .thumb.off {
    background: transparent;
  }
  button {
    position: relative;
    height: 26px;
    padding: 0 12px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
    transition: color 0.2s;
  }
  button.on {
    color: var(--text);
  }
  .thumb.auto ~ button.on {
    color: #fff;
  }
</style>
