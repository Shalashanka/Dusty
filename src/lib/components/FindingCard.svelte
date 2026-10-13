<script lang="ts">
  import { ExternalLink, Globe, Clock, Files, ShieldCheck } from "@lucide/svelte";
  import FileGlyph from "./FileGlyph.svelte";
  import { api, type Action, type Finding } from "$lib/api";
  import { ACTIONS, KINDS } from "$lib/kinds";
  import { age, bytes, host, shortPath } from "$lib/format";
  import { app } from "$lib/app.svelte";

  let {
    f,
    primary,
    secondary,
    dx = 0,
    onact,
  }: {
    f: Finding;
    primary: Action;
    secondary: Action | null;
    dx?: number;
    onact: (a: Action) => void;
  } = $props();

  const meta = $derived(KINDS[f.kind]);
  const KindIcon = $derived(meta.icon);
  const extra = $derived(f.actions.filter((a) => a !== primary && a !== secondary));
  const leftStamp = $derived(Math.min(1, Math.max(0, -dx - 30) / 100));
  const rightStamp = $derived(Math.min(1, Math.max(0, dx - 30) / 100));
  const relatedLabel = $derived(
    f.kind === "duplicate"
      ? "Original stays"
      : f.kind === "dev_junk"
        ? "Will clear"
        : f.kind === "desktop_clutter"
          ? "Will move"
          : f.kind === "extracted_archive"
            ? "Extracted to"
            : f.kind === "backup"
              ? "Original"
              : "",
  );
  const related = $derived(f.related.slice(0, f.kind === "desktop_clutter" ? 8 : 4));
  const name = (p: string) => p.slice(p.lastIndexOf("\\") + 1);
</script>

<article class="card-face" style="--c:{meta.color}">
  <div class="stamp left" style="opacity:{leftStamp}">{ACTIONS[primary].label}</div>
  {#if secondary}<div class="stamp right" style="opacity:{rightStamp}">{ACTIONS[secondary].label}</div>{/if}

  <header>
    <span class="kind"><KindIcon size={14} /> {meta.label}</span>
    <span class="sure" title="How sure I am this can go">
      <ShieldCheck size={13} />
      {Math.round(f.confidence * 100)}% sure
    </span>
  </header>

  <div class="ident">
    <FileGlyph ext={f.ext} isDir={f.isDir} size={64} color={f.kind === "system_junk" || f.kind === "dev_junk" ? meta.color : ""} />
    <div class="names">
      <h2 title={f.title}>{f.title}</h2>
      <button class="path mono" title="Show in Explorer" onclick={() => api.reveal(f.path)}>
        <span class="ellipsis">{shortPath(f.path, app.home)}</span>
        <ExternalLink size={12} />
      </button>
    </div>
  </div>

  <div class="facts">
    {#if f.bytes}<span class="chip strong">{bytes(f.bytes)}</span>{/if}
    {#if f.lastTouched && f.kind !== "system_junk"}{@const a = age(f.lastTouched)}<span class="chip"><Clock size={13} /> touched {a === "today" ? a : `${a} ago`}</span>{/if}
    {#if f.origin}<span class="chip" title={f.origin}><Globe size={13} /> from {host(f.origin)}</span>{/if}
    {#if f.isDir && f.items > 1}<span class="chip"><Files size={13} /> {f.items.toLocaleString()} files</span>{/if}
  </div>

  <p class="reason">{f.reason}</p>

  {#if related.length && relatedLabel}
    <div class="related">
      <span class="faint">{relatedLabel}</span>
      <div class="rel-list">
        {#each related as r (r)}
          <button class="chip rel" title={r} onclick={() => api.reveal(r)}>{name(r)}</button>
        {/each}
        {#if f.related.length > related.length}<span class="chip">+{f.related.length - related.length} more</span>{/if}
      </div>
    </div>
  {/if}

  <footer>
    <button class="act primary-act {ACTIONS[primary].tone}" onclick={() => onact(primary)}>
      {#if ACTIONS[primary]}{@const I = ACTIONS[primary].icon}<I size={17} />{/if}
      {ACTIONS[primary].label}
      <span class="kbd">←</span>
    </button>
    <div class="extra">
      {#each extra as a (a)}
        {@const I = ACTIONS[a].icon}
        <button class="act" onclick={() => onact(a)}>
          <I size={16} />
          {ACTIONS[a].label}
          <span class="kbd">{a === "snooze" ? "↓" : a === "archive" ? "A" : ACTIONS[a].keys[0]}</span>
        </button>
      {/each}
    </div>
    {#if secondary}
      {@const I = ACTIONS[secondary].icon}
      <button class="act {ACTIONS[secondary].tone}" onclick={() => onact(secondary)}>
        <I size={16} />
        {ACTIONS[secondary].label}
        <span class="kbd">→</span>
      </button>
    {/if}
  </footer>
</article>

<style>
  .card-face {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 24px 18px;
    border-radius: var(--r-xl);
    background:
      radial-gradient(120% 70% at 100% 0%, color-mix(in srgb, var(--c) 14%, transparent), transparent 60%),
      color-mix(in srgb, var(--bg-solid) 94%, white 6%);
    border: 1px solid var(--border-strong);
    box-shadow:
      0 30px 60px -30px rgba(0, 0, 0, 0.7),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .kind {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--c);
    font-weight: 600;
    font-size: 13px;
    padding: 5px 11px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .sure {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-2);
  }
  .ident {
    display: flex;
    gap: 16px;
    align-items: center;
    min-width: 0;
  }
  .names {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  h2 {
    font-size: 22px;
    line-height: 1.2;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    user-select: text;
  }
  .path {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
    max-width: 100%;
    text-align: left;
  }
  .path:hover {
    color: var(--accent);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip.strong {
    color: var(--text);
    font-weight: 600;
  }
  .reason {
    margin: 0;
    font-size: 15.5px;
    line-height: 1.5;
    color: var(--text);
    opacity: 0.92;
  }
  .related {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
  }
  .rel-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .rel {
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rel:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  footer {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .extra {
    flex: 1;
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 40px;
    padding: 0 12px 0 14px;
    border-radius: 999px;
    font-weight: 600;
    background: var(--surface-2);
    border: 1px solid var(--border);
    transition:
      transform 0.15s var(--ease),
      background 0.2s,
      color 0.2s;
  }
  .act:hover {
    background: var(--surface-3);
  }
  .act:active {
    transform: scale(0.95);
  }
  .act.danger {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: transparent;
  }
  .act.danger:hover {
    background: var(--danger);
    color: #fff;
  }
  .act.ok {
    background: var(--ok-soft);
    color: var(--ok);
    border-color: transparent;
  }
  .act.ok:hover {
    background: var(--ok);
    color: #0b2a22;
  }
  .act.accent {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: transparent;
  }
  .act.accent:hover {
    background: var(--accent);
    color: #fff;
  }
  .act :global(.kbd) {
    background: rgba(0, 0, 0, 0.15);
    border-color: transparent;
    color: inherit;
    opacity: 0.8;
  }
  .stamp {
    position: absolute;
    top: 26px;
    padding: 6px 14px;
    border: 3px solid;
    border-radius: 12px;
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    pointer-events: none;
    z-index: 2;
    background: color-mix(in srgb, var(--bg-solid) 70%, transparent);
  }
  .stamp.left {
    right: 24px;
    color: var(--danger);
    transform: rotate(10deg);
  }
  .stamp.right {
    left: 24px;
    color: var(--ok);
    transform: rotate(-10deg);
  }
</style>
