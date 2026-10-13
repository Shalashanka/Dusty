<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { House, Layers, ChartPie, Gauge, History, SlidersHorizontal, Pause, Play, Search } from "@lucide/svelte";
  import { app, type View } from "$lib/app.svelte";
  import { api } from "$lib/api";
  import { clock } from "$lib/format";

  const items: { id: View; label: string; icon: typeof House; key: string }[] = [
    { id: "home", label: "Home", icon: House, key: "1" },
    { id: "triage", label: "Tidy", icon: Layers, key: "2" },
    { id: "space", label: "Space", icon: ChartPie, key: "3" },
    { id: "speed", label: "Speed", icon: Gauge, key: "4" },
    { id: "history", label: "History", icon: History, key: "5" },
    { id: "rules", label: "Settings", icon: SlidersHorizontal, key: "6" },
  ];
  const index = $derived(items.findIndex((i) => i.id === app.view));

  async function togglePause() {
    await api.pause(app.paused ? 0 : 180);
    await app.refresh();
    app.toast(app.paused ? `Napping until ${clock(app.snap!.pausedUntil)}. Wake me anytime.` : "I'm awake and watching again.");
  }
</script>

<nav class="rail" data-tauri-drag-region>
  <div class="logo" data-tauri-drag-region>
    <svg viewBox="0 0 40 40" width="34" height="34" aria-hidden="true">
      <defs>
        <radialGradient id="lg" cx="38%" cy="30%" r="75%">
          <stop offset="0%" stop-color="#fff" /><stop offset="45%" stop-color="#d7cfff" /><stop offset="100%" stop-color="#7b6cf0" />
        </radialGradient>
      </defs>
      <circle cx="20" cy="22" r="14" fill="url(#lg)" />
      <path d="M7 19 Q8 7 20 7 Q32 7 33 19 Q27 15 20 15 Q13 15 7 19Z" fill="#ff7a90" />
      <ellipse cx="15.5" cy="21.5" rx="2" ry="2.7" fill="#1d1a2e" />
      <ellipse cx="24.5" cy="21.5" rx="2" ry="2.7" fill="#1d1a2e" />
      <path d="M17.5 27 q2.5 2.3 5 0" stroke="#1d1a2e" stroke-width="1.4" fill="none" stroke-linecap="round" />
    </svg>
  </div>

  <div class="items">
    <div class="pill" style="transform: translateY({index * 62}px)"></div>
    {#each items as it (it.id)}
      {@const Icon = it.icon}
      <button class="item" class:active={app.view === it.id} onclick={() => app.go(it.id)} title="{it.label} ({it.key})">
        <span class="icon">
          <Icon size={20} strokeWidth={app.view === it.id ? 2.2 : 1.8} />
          {#if it.id === "triage" && app.allFindings.length}
            <span class="badge">{app.allFindings.length > 99 ? "99+" : app.allFindings.length}</span>
          {/if}
        </span>
        <span class="label">{it.label}</span>
      </button>
    {/each}
  </div>

  <div class="bottom">
    {#if app.scanning || app.index?.building}
      <div class="scanning" title={app.index?.building ? "Indexing your drives" : app.phase}><span></span><span></span><span></span></div>
    {/if}
    <button class="item small" onclick={() => invoke("toggle_search")} title="Search everything ({app.settings?.searchHotkey})">
      <span class="icon"><Search size={18} /></span>
      <span class="label">Search</span>
    </button>
    <button class="item small" onclick={togglePause} title={app.paused ? "Wake up" : "Pause for 3 hours"}>
      <span class="icon">
        {#if app.paused}<Play size={18} />{:else}<Pause size={18} />{/if}
      </span>
      <span class="label">{app.paused ? "Wake" : "Pause"}</span>
    </button>
  </div>
</nav>

<style>
  .rail {
    width: 84px;
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 14px 0 14px;
    gap: 16px;
    border-right: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
    position: relative;
    z-index: 401;
  }
  .logo {
    height: 44px;
    display: grid;
    place-items: center;
    filter: drop-shadow(0 4px 12px rgba(139, 124, 255, 0.5));
  }
  .items {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .pill {
    position: absolute;
    top: 0;
    left: 6px;
    width: 56px;
    height: 58px;
    border-radius: 16px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    transition: transform 0.4s var(--ease);
  }
  .item {
    position: relative;
    width: 68px;
    height: 58px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--text-2);
    border-radius: 16px;
    transition: color 0.2s;
  }
  .item:hover {
    color: var(--text);
  }
  .item.active {
    color: var(--accent);
  }
  .item:active .icon {
    transform: scale(0.9);
  }
  .icon {
    position: relative;
    display: grid;
    transition: transform 0.2s var(--ease);
  }
  .label {
    font-size: 11px;
    font-weight: 600;
  }
  .badge {
    position: absolute;
    top: -7px;
    left: 13px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--danger);
    color: #fff;
    font-size: 10.5px;
    font-weight: 700;
    display: grid;
    place-items: center;
    box-shadow: 0 0 0 2px var(--bg-solid);
  }
  .bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .item.small {
    height: 50px;
  }
  .scanning {
    display: flex;
    gap: 4px;
    margin-bottom: 6px;
  }
  .scanning span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: dot 1.2s ease-in-out infinite;
  }
  .scanning span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .scanning span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes dot {
    0%,
    100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    50% {
      transform: translateY(-4px);
      opacity: 1;
    }
  }
</style>
