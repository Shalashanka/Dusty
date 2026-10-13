<script lang="ts">
  import { HardDrive, Layers } from "@lucide/svelte";
  import { app, driveOf } from "$lib/app.svelte";

  const letters = $derived(app.drives.map((d) => d.mount.replace(/\\$/, "").toUpperCase()));
  const count = (d: string) => app.allFindings.filter((f) => driveOf(f.path) === d).length;
</script>

{#if letters.length > 1}
  <div class="drives" role="radiogroup" aria-label="Drive">
    <button class:on={app.drive === "all"} onclick={() => (app.drive = "all")} title="All drives"><Layers size={13} /> All</button>
    {#each letters as d (d)}
      <button class:on={app.drive === d} onclick={() => (app.drive = d)} title="Only {d}">
        <HardDrive size={13} />
        {d}
        {#if count(d)}<i>{count(d)}</i>{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .drives {
    display: flex;
    padding: 3px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    flex: none;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
    transition:
      background 0.2s,
      color 0.2s;
  }
  button.on {
    background: var(--surface-3);
    color: var(--text);
  }
  i {
    font-style: normal;
    font-size: 11px;
    color: var(--text-3);
  }
</style>
