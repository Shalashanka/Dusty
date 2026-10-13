<script lang="ts">
  import { ExternalLink, FolderOpen, Globe, Clock, Files, ShieldCheck, Ellipsis, Shield, Copy, Flame, AppWindow } from "@lucide/svelte";
  import Preview from "./Preview.svelte";
  import { api, type Action, type Finding } from "$lib/api";
  import { ACTIONS, KINDS, previewKind } from "$lib/kinds";
  import { age, bytes, host, shortPath, parent } from "$lib/format";
  import { app } from "$lib/app.svelte";
  import { menu, type MenuItem } from "$lib/menu.svelte";

  let {
    f,
    primary,
    secondary,
    dx = 0,
    onact,
    active = true,
  }: {
    f: Finding;
    primary: Action;
    secondary: Action | null;
    dx?: number;
    onact: (a: Action) => void;
    active?: boolean;
  } = $props();

  const meta = $derived(KINDS[f.kind]);
  const KindIcon = $derived(meta.icon);
  const extra = $derived(f.actions.filter((a) => a !== primary && a !== secondary));
  const leftStamp = $derived(Math.min(1, Math.max(0, -dx - 30) / 100));
  const rightStamp = $derived(Math.min(1, Math.max(0, dx - 30) / 100));
  const fileBased = $derived(!["new_startup", "resource_hog", "recycle_bin"].includes(f.kind));
  const canPreview = $derived(fileBased && !f.isDir && !!previewKind(f.ext));
  const compare = $derived((f.kind === "duplicate" || f.kind === "similar_photo") && f.related.length > 0);
  const relatedLabel = $derived(
    ({
      duplicate: "Original stays",
      similar_photo: "Better copy",
      dev_junk: "Will clear",
      stale_project: "Left out of the zip (rebuildable)",
      desktop_clutter: "Will move",
      extracted_archive: "Extracted to",
      backup: "Original",
      folder_plan: "In there",
      resource_hog: "Processes",
      new_startup: "Command",
    } as Record<string, string>)[f.kind] ?? "",
  );
  const related = $derived(f.related.slice(0, f.kind === "desktop_clutter" || f.kind === "folder_plan" ? 8 : 4));
  const name = (p: string) => p.slice(p.lastIndexOf("\\") + 1);

  function more(e: MouseEvent) {
    const items: MenuItem[] = [];
    if (fileBased) {
      const folder = f.kind === "folder_plan" || f.kind === "desktop_clutter" ? f.path : parent(f.path);
      items.push(
        { caption: "Don't touch this" },
        { label: "For a day", icon: Shield, run: () => app.protect(f.path, 1, `“${f.title}”`) },
        { label: "For a week", icon: Shield, run: () => app.protect(f.path, 7, `“${f.title}”`) },
        { label: "For a month", icon: Shield, run: () => app.protect(f.path, 30, `“${f.title}”`) },
        { label: "Never, it's important", icon: ShieldCheck, run: () => app.protect(f.path, 0, `“${f.title}”`) },
      );
      if (folder !== f.path) {
        items.push(
          { caption: `The whole folder “${name(folder)}”` },
          { label: "Leave it alone for a month", icon: Shield, run: () => app.protect(folder, 30, `“${name(folder)}”`) },
          { label: "It's important, never touch it", icon: ShieldCheck, run: () => app.protect(folder, 0, `“${name(folder)}”`) },
        );
      }
      items.push({ divider: true }, { label: "Copy path", icon: Copy, run: () => navigator.clipboard.writeText(f.path) });
      if (!f.actions.includes("delete") && f.actions.includes("recycle")) {
        items.push({ label: "Delete for good (skip the bin)", icon: Flame, danger: true, run: () => onact("delete") });
      }
    }
    menu.show(e, items);
  }
</script>

<article class="card-face" class:question={meta.question} style="--c:{meta.color}">
  <div class="stamp left" style="opacity:{leftStamp}">{ACTIONS[primary].label}</div>
  {#if secondary}<div class="stamp right" style="opacity:{rightStamp}">{ACTIONS[secondary].label}</div>{/if}

  <header>
    <span class="kind"><KindIcon size={14} /> {meta.label}</span>
    {#if f.app}<span class="chip app"><AppWindow size={12} /> {f.app}</span>{/if}
    <span class="sp"></span>
    {#if !meta.question}
      <span class="sure" title="How sure I am this can go">
        <ShieldCheck size={13} />
        {Math.round(f.confidence * 100)}% sure
      </span>
    {/if}
    {#if fileBased && active}
      {#if !f.isDir || f.kind === "unsure"}
        <button class="tool" title="Open" onclick={() => api.openFile(f.path)}><ExternalLink size={15} /></button>
      {/if}
      <button class="tool" title="Show in folder (O)" onclick={() => api.reveal(f.path)}><FolderOpen size={15} /></button>
      <button class="tool" title="More" onclick={more}><Ellipsis size={16} /></button>
    {/if}
  </header>

  <div class="ident">
    {#if fileBased && active}
      <Preview path={f.path} ext={f.ext} isDir={f.isDir} size={64} color={f.kind === "system_junk" || f.kind === "dev_junk" ? meta.color : ""} />
    {:else}
      <div class="glyph" style="--c:{meta.color}"><KindIcon size={28} /></div>
    {/if}
    <div class="names">
      <h2 title={f.title}>{meta.question ? `What's “${f.title}”?` : f.title}</h2>
      {#if fileBased}
        <button class="path mono" title="Show in Explorer" onclick={() => api.reveal(f.path)}>
          <span class="ellipsis">{shortPath(f.path, app.home)}</span>
        </button>
      {/if}
    </div>
  </div>

  <div class="facts">
    {#if f.bytes}<span class="chip strong">{bytes(f.bytes)}{f.kind === "resource_hog" ? " RAM" : ""}</span>{/if}
    {#if f.lastTouched && f.kind !== "system_junk"}
      {@const a = age(f.lastTouched)}
      <span class="chip"><Clock size={13} /> touched {a === "today" ? a : `${a} ago`}</span>
    {/if}
    {#if f.origin}<span class="chip" title={f.origin}><Globe size={13} /> from {host(f.origin)}</span>{/if}
    {#if f.isDir && f.items > 1}<span class="chip"><Files size={13} /> {f.items.toLocaleString()} {f.kind === "folder_plan" || f.kind === "desktop_clutter" ? "items" : "files"}</span>{/if}
  </div>

  <p class="reason">{f.reason}</p>

  {#if active && compare}
    <div class="compare">
      <figure>
        <Preview path={f.path} ext={f.ext} rich />
        <figcaption>This one · {bytes(f.bytes)}</figcaption>
      </figure>
      <figure>
        <Preview path={f.related[0]} ext={f.ext} rich />
        <figcaption>{f.kind === "duplicate" ? "Original" : "Better copy"} · {name(f.related[0])}</figcaption>
      </figure>
    </div>
  {:else if active && canPreview}
    <div class="preview"><Preview path={f.path} ext={f.ext} rich /></div>
  {:else if related.length && relatedLabel}
    <div class="related">
      <span class="faint">{relatedLabel}</span>
      <div class="rel-list">
        {#each related as r (r)}
          <button class="chip rel" title={r} onclick={() => fileBased && api.reveal(r)}>{fileBased ? name(r) : r}</button>
        {/each}
        {#if f.related.length > related.length}<span class="chip">+{f.related.length - related.length} more</span>{/if}
      </div>
    </div>
  {/if}

  <footer>
    <button class="act primary-act {ACTIONS[primary].tone}" title={ACTIONS[primary].hint} onclick={() => onact(primary)}>
      {#if ACTIONS[primary]}{@const I = ACTIONS[primary].icon}<I size={17} />{/if}
      {ACTIONS[primary].label}
      <span class="kbd">←</span>
    </button>
    <div class="extra">
      {#each extra as a (a)}
        {@const I = ACTIONS[a].icon}
        <button class="act" title={ACTIONS[a].hint} onclick={() => onact(a)}>
          <I size={16} />
          {ACTIONS[a].label}
          {#if a === "snooze" || a === "archive"}<span class="kbd">{a === "snooze" ? "↓" : "A"}</span>{/if}
        </button>
      {/each}
    </div>
    {#if secondary}
      {@const I = ACTIONS[secondary].icon}
      <button class="act {ACTIONS[secondary].tone}" title={ACTIONS[secondary].hint} onclick={() => onact(secondary)}>
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
    gap: 14px;
    padding: 20px 22px 16px;
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
  .card-face.question {
    background:
      radial-gradient(120% 70% at 0% 0%, color-mix(in srgb, var(--c) 18%, transparent), transparent 60%),
      color-mix(in srgb, var(--bg-solid) 94%, white 6%);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sp {
    flex: 1;
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
  .app {
    height: 24px;
    font-size: 12px;
  }
  .sure {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-2);
    margin-right: 6px;
  }
  .tool {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: var(--text-2);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .tool:hover {
    color: var(--text);
    background: var(--surface-3);
  }
  .ident {
    display: flex;
    gap: 14px;
    align-items: center;
    min-width: 0;
  }
  .glyph {
    width: 64px;
    height: 64px;
    flex: none;
    border-radius: 18px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
  }
  .names {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  h2 {
    font-size: 21px;
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
    font-size: 15px;
    line-height: 1.5;
    color: var(--text);
    opacity: 0.92;
  }
  .preview {
    flex: 1;
    min-height: 0;
    display: flex;
    justify-content: center;
    align-items: stretch;
  }
  .compare {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .compare figure {
    margin: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .compare figure :global(> :first-child) {
    flex: 1;
    min-height: 0;
  }
  figcaption {
    font-size: 11.5px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    max-width: 280px;
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
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .extra {
    flex: 1;
    display: flex;
    justify-content: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 38px;
    padding: 0 12px 0 14px;
    border-radius: 999px;
    font-weight: 600;
    white-space: nowrap;
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
    top: 70px;
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
