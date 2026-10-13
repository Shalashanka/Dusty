<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Snackbar from "$lib/components/Snackbar.svelte";
  import Palette from "$lib/components/Palette.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import Home from "$lib/views/Home.svelte";
  import Triage from "$lib/views/Triage.svelte";
  import Space from "$lib/views/Space.svelte";
  import History from "$lib/views/History.svelte";
  import Rules from "$lib/views/Rules.svelte";
  import { app, type View } from "$lib/app.svelte";
  import { api } from "$lib/api";

  let loaded = $state(false);

  onMount(async () => {
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      (await import("$lib/mock")).installMocks();
      const v = new URLSearchParams(location.search).get("view");
      if (v) app.view = v as View;
    }
    await app.init();
    loaded = true;
    // Let the first frame paint before the window appears (no white flash).
    requestAnimationFrame(() => requestAnimationFrame(() => api.ready()));
  });

  const VIEWS: View[] = ["home", "triage", "space", "history", "rules"];

  function onkey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key.toLowerCase() === "k") {
      e.preventDefault();
      app.paletteOpen = !app.paletteOpen;
      return;
    }
    if (app.paletteOpen || (e.target as HTMLElement)?.closest("input, select, textarea")) return;
    if (e.key === "z" && app.view !== "triage" && app.snack?.undo) {
      const u = app.snack.undo;
      app.snack = null;
      u();
      return;
    }
    const n = Number(e.key);
    if (!e.ctrlKey && !e.altKey && n >= 1 && n <= VIEWS.length) app.go(VIEWS[n - 1]);
  }
</script>

<svelte:window onkeydown={onkey} oncontextmenu={(e) => e.preventDefault()} />

<div class="shell">
  <TitleBar />
  {#if loaded}
    <Sidebar />
    <main>
      {#key app.view}
        <div class="view" in:fly={{ y: 12, duration: 380, easing: cubicOut }} out:fade={{ duration: 90 }}>
          {#if app.view === "home"}<Home />
          {:else if app.view === "triage"}<Triage />
          {:else if app.view === "space"}<Space />
          {:else if app.view === "history"}<History />
          {:else}<Rules />{/if}
        </div>
      {/key}
    </main>
    <Snackbar />
    <Palette />
    {#if app.settings && !app.settings.onboarded}<Onboarding />{/if}
  {/if}
</div>

<style>
  .shell {
    position: fixed;
    inset: 0;
    display: flex;
    background:
      radial-gradient(900px 500px at 85% -10%, rgba(139, 124, 255, 0.12), transparent 60%),
      radial-gradient(700px 400px at 0% 110%, rgba(79, 227, 193, 0.07), transparent 60%),
      var(--bg);
    overflow: hidden;
  }
  main {
    flex: 1;
    position: relative;
    min-width: 0;
  }
  .view {
    position: absolute;
    inset: 0;
    padding-top: 12px;
  }
</style>
