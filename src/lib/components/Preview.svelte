<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";
  import FileGlyph from "./FileGlyph.svelte";
  import { api } from "$lib/api";
  import { previewKind } from "$lib/kinds";

  let {
    path,
    ext = "",
    isDir = false,
    size = 64,
    rich = false,
    color = "",
  }: { path: string; ext?: string; isDir?: boolean; size?: number; rich?: boolean; color?: string } = $props();

  const kind = $derived(isDir ? null : previewKind(ext));
  let thumb = $state<string | null>(null);
  let text = $state<string | null>(null);
  let failed = $state(false);

  const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const src = $derived(inTauri && /^[a-z]:\\/i.test(path) ? convertFileSrc(path) : "");

  $effect(() => {
    thumb = null;
    text = null;
    failed = false;
    const p = path;
    if (!inTauri || !/^[a-z]:\\/i.test(p)) return;
    if (rich && kind === "text") {
      api.readText(p, 6000).then((t) => (text = t)).catch(() => (failed = true));
    }
    // Windows shell thumbnails cover everything else (PSD, video frames, folders...).
    if (!rich || !kind || kind === "pdf" || kind === "text") {
      api.thumbnail(p, rich ? 512 : 160).then((t) => (thumb = t));
    }
  });
</script>

{#if rich && kind === "image" && src && !failed}
  <img class="rich" {src} alt="" onerror={() => (failed = true)} in:fade />
{:else if rich && kind === "video" && src && !failed}
  <!-- svelte-ignore a11y_media_has_caption -->
  <video class="rich" {src} controls muted preload="metadata" onerror={() => (failed = true)} in:fade></video>
{:else if rich && kind === "audio" && src}
  <div class="audio">
    {#if thumb}<img src={thumb} alt="" class="cover" />{/if}
    <audio {src} controls preload="metadata"></audio>
  </div>
{:else if rich && kind === "text" && text !== null}
  <pre class="text" in:fade>{text}</pre>
{:else if rich && kind === "pdf" && src}
  <iframe class="rich pdf" {src} title="PDF preview"></iframe>
{:else if thumb}
  <img class="thumb" class:rich src={thumb} alt="" style={rich ? "" : `width:${size}px;height:${size}px`} in:fade={{ duration: 200 }} />
{:else}
  <FileGlyph {ext} {isDir} {size} {color} />
{/if}

<style>
  .thumb {
    border-radius: calc(var(--r) - 2px);
    object-fit: cover;
    background: var(--surface-2);
    flex: none;
  }
  .rich {
    width: 100%;
    max-height: 100%;
    border-radius: var(--r);
    object-fit: contain;
    background: rgba(0, 0, 0, 0.25);
    display: block;
  }
  .thumb.rich {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .pdf {
    height: 100%;
    border: 0;
    background: #fff;
  }
  .text {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 12px 14px;
    border-radius: var(--r);
    background: rgba(0, 0, 0, 0.25);
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-2);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .audio {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
    width: 100%;
  }
  .cover {
    width: 120px;
    height: 120px;
    border-radius: var(--r);
    object-fit: cover;
  }
  audio {
    width: 100%;
  }
</style>
