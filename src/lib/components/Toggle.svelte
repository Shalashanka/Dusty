<script lang="ts">
  let { checked = $bindable(false), onchange }: { checked?: boolean; onchange?: (v: boolean) => void } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label="Toggle"
  onclick={() => {
    checked = !checked;
    onchange?.(checked);
  }}><span></span></button
>

<style>
  .toggle {
    width: 42px;
    height: 24px;
    flex: none;
    border-radius: 99px;
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    position: relative;
    transition: background 0.25s;
  }
  .toggle span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-2);
    transition: transform 0.35s var(--spring), background 0.25s, width 0.2s;
  }
  .toggle:active span {
    width: 20px;
  }
  .toggle.on {
    background: var(--accent);
    border-color: transparent;
  }
  .toggle.on span {
    transform: translateX(18px);
    background: #fff;
  }
  .toggle.on:active span {
    transform: translateX(14px);
  }
</style>
