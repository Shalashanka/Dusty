<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { X, Copy, FolderOpen, Check, ArrowLeftRight } from "@lucide/svelte";
  import { app } from "$lib/app.svelte";
  import { api } from "$lib/api";

  interface Option {
    a: string;
    b: string;
    depth: number;
  }

  const split = (p: string) => p.replace(/\\+$/, "").split("\\");
  const join = (parts: string[]) => (parts.length === 1 ? parts[0] + "\\" : parts.join("\\"));

  /** Folder pairs from the files' parents upwards, while the stripped tails match. */
  function options(pa: string, pb: string): Option[] {
    const A = split(pa);
    const B = split(pb);
    let k = 0;
    while (k < Math.min(A.length, B.length) - 1 && A[A.length - 1 - k].toLowerCase() === B[B.length - 1 - k].toLowerCase()) k++;
    const out: Option[] = [];
    // i = number of trailing components stripped (1 = the files' own folders).
    for (let i = 1; i <= Math.max(1, k); i++) {
      if (A.length - i < 1 || B.length - i < 1) break;
      out.push({ a: join(A.slice(0, A.length - i)), b: join(B.slice(0, B.length - i)), depth: i });
    }
    return out;
  }

  const opts = $derived(app.mirrorPair ? options(app.mirrorPair.a, app.mirrorPair.b) : []);
  let chosen = $state(0);
  let manualA = $state("");
  let manualB = $state("");

  $effect(() => {
    if (!app.mirrorPair) return;
    // Suggest the highest level where both folders still share a name (…\Photos ↔ …\Photos).
    const o = opts;
    const named = o.filter((x) => {
      const na = split(x.a).at(-1)!.toLowerCase();
      const nb = split(x.b).at(-1)!.toLowerCase();
      return na === nb && !na.endsWith(":");
    });
    chosen = named.length ? o.indexOf(named[named.length - 1]) : 0;
    manualA = "";
    manualB = "";
  });

  const pick = $derived(manualA && manualB ? { a: manualA, b: manualB } : opts[chosen]);

  async function save() {
    if (!pick) return;
    try {
      await api.addMirror(pick.a, pick.b);
      if (app.settings) app.settings.mirrors = [...(app.settings.mirrors ?? []), { a: pick.a, b: pick.b }];
      app.mirrorPair = null;
      await app.refresh();
      app.toast(`Got it: ${pick.a} and ${pick.b} are mirrors. I won't compare them anymore.`, { tone: "ok" });
    } catch (e) {
      app.toast(String(e), { tone: "error" });
    }
  }
  async function choose(side: "a" | "b") {
    const p = await api.pickFolder();
    if (!p) return;
    if (side === "a") manualA = p;
    else manualB = p;
    if (!manualA) manualA = opts[chosen]?.a ?? "";
    if (!manualB) manualB = opts[chosen]?.b ?? "";
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && app.mirrorPair && (app.mirrorPair = null)} />

{#if app.mirrorPair}
  <div class="veil" transition:fade={{ duration: 160 }} role="presentation" onclick={() => (app.mirrorPair = null)}></div>
  <div class="modal" transition:scale={{ start: 0.96, duration: 220, easing: cubicOut }} role="dialog" aria-label="Mirror folders">
    <header>
      <span class="ic"><Copy size={18} /></span>
      <div>
        <h2>These are backups of each other</h2>
        <span class="faint">Pick how far up the two folders mirror each other. I'll skip both when hunting duplicates and similar photos.</span>
      </div>
      <button class="close" onclick={() => (app.mirrorPair = null)} aria-label="Close"><X size={18} /></button>
    </header>

    <div class="body">
      {#each opts as o, i (o.depth)}
        <button class="opt" class:on={!manualA && chosen === i} onclick={() => { chosen = i; manualA = ""; manualB = ""; }}>
          <span class="radio">{#if !manualA && chosen === i}<Check size={12} />{/if}</span>
          <span class="mono ellipsis" title={o.a}>{o.a}</span>
          <ArrowLeftRight size={14} class="faint" />
          <span class="mono ellipsis" title={o.b}>{o.b}</span>
        </button>
      {/each}

      <div class="manual">
        <span class="faint">Or choose the folders yourself:</span>
        <button class="btn sm" onclick={() => choose("a")}><FolderOpen size={13} /> {manualA ? manualA : "First folder…"}</button>
        <button class="btn sm" onclick={() => choose("b")}><FolderOpen size={13} /> {manualB ? manualB : "Second folder…"}</button>
      </div>
    </div>

    <footer>
      {#if pick}<span class="faint ellipsis">Mirrors: <b>{pick.a}</b> ↔ <b>{pick.b}</b></span>{/if}
      <span class="sp"></span>
      <button class="btn ghost" onclick={() => (app.mirrorPair = null)}>Cancel</button>
      <button class="btn primary" disabled={!pick} onclick={save}>Don't compare these</button>
    </footer>
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(3px);
    z-index: 300;
  }
  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: min(760px, 90vw);
    z-index: 301;
    display: flex;
    flex-direction: column;
    border-radius: var(--r-xl);
    background: color-mix(in srgb, var(--bg-solid) 96%, white 4%);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 100px -20px rgba(0, 0, 0, 0.8);
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 18px 22px;
    border-bottom: 1px solid var(--border);
  }
  header > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }
  h2 {
    font-size: 18px;
  }
  .ic {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: var(--accent);
    background: var(--accent-soft);
  }
  .close {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .body {
    padding: 14px 22px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .opt {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .radio {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    display: grid;
    place-items: center;
  }
  .opt.on .radio {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .manual {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 12.5px;
    flex-wrap: wrap;
  }
  .manual .btn {
    max-width: 260px;
    overflow: hidden;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 22px;
    border-top: 1px solid var(--border);
    font-size: 12.5px;
    min-width: 0;
  }
  .sp {
    flex: 1;
  }
</style>
