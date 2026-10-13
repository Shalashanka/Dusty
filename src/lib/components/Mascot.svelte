<script module lang="ts">
  export type Mood = "idle" | "sleeping" | "sweeping" | "happy" | "curious";
</script>

<script lang="ts">
  import { onMount } from "svelte";

  let { mood = "idle", size = 160 }: { mood?: Mood; size?: number } = $props();

  let el: SVGSVGElement;
  let look = $state({ x: 0, y: 0 });
  let blink = $state(false);
  const uid = Math.random().toString(36).slice(2, 8);

  onMount(() => {
    const onMove = (e: PointerEvent) => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      const dx = e.clientX - (r.left + r.width / 2);
      const dy = e.clientY - (r.top + r.height * 0.5);
      const d = Math.hypot(dx, dy) || 1;
      const k = Math.min(1, d / 400);
      look = { x: (dx / d) * 5 * k, y: (dy / d) * 4 * k };
    };
    window.addEventListener("pointermove", onMove);
    let t: ReturnType<typeof setTimeout>;
    const loop = () => {
      blink = true;
      setTimeout(() => (blink = false), 140);
      t = setTimeout(loop, 2500 + Math.random() * 3500);
    };
    t = setTimeout(loop, 1800);
    return () => {
      window.removeEventListener("pointermove", onMove);
      clearTimeout(t);
    };
  });

  const eyesClosed = $derived(mood === "sleeping");
  const eyesHappy = $derived(mood === "happy");
  const lookX = $derived(mood === "curious" ? look.x * 1.3 : mood === "sweeping" ? 0 : look.x);
  const lookY = $derived(mood === "sweeping" ? 2 : look.y);
</script>

<svg
  bind:this={el}
  class="mascot {mood}"
  width={size}
  height={size}
  viewBox="0 0 200 200"
  role="img"
  aria-label="Dustling, feeling {mood}"
>
  <defs>
    <radialGradient id="body-{uid}" cx="38%" cy="30%" r="75%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="35%" stop-color="#e4defe" />
      <stop offset="75%" stop-color="#a99bff" />
      <stop offset="100%" stop-color="#7b6cf0" />
    </radialGradient>
    <radialGradient id="glow-{uid}" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#8b7cff" stop-opacity="0.55" />
      <stop offset="100%" stop-color="#8b7cff" stop-opacity="0" />
    </radialGradient>
    <filter id="fluff-{uid}" x="-20%" y="-20%" width="140%" height="140%">
      <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed="7" result="n">
        <animate attributeName="seed" values="7;8;9;8;7" dur="1.6s" repeatCount="indefinite" calcMode="discrete" />
      </feTurbulence>
      <feDisplacementMap in="SourceGraphic" in2="n" scale="11" xChannelSelector="R" yChannelSelector="G" />
    </filter>
  </defs>

  <circle class="aura" cx="100" cy="105" r="95" fill="url(#glow-{uid})" />
  <ellipse class="shadow" cx="100" cy="182" rx="44" ry="7" fill="#000" opacity="0.28" />

  <g class="bob">
    <g class="wobble">
      <!-- fuzzy outline + solid core -->
      <circle cx="100" cy="108" r="60" fill="url(#body-{uid})" filter="url(#fluff-{uid})" />
      <circle cx="100" cy="108" r="54" fill="url(#body-{uid})" />
      <ellipse cx="80" cy="82" rx="16" ry="10" fill="#fff" opacity="0.55" transform="rotate(-25 80 82)" />

      <!-- feet -->
      <ellipse class="foot l" cx="82" cy="164" rx="11" ry="6" fill="#8d7ef5" />
      <ellipse class="foot r" cx="118" cy="164" rx="11" ry="6" fill="#8d7ef5" />

      <!-- cheeks -->
      <ellipse cx="68" cy="122" rx="9" ry="5.5" fill="#ff8fb8" opacity={mood === "happy" ? 0.75 : 0.45} />
      <ellipse cx="132" cy="122" rx="9" ry="5.5" fill="#ff8fb8" opacity={mood === "happy" ? 0.75 : 0.45} />

      <!-- eyes -->
      <g transform="translate({lookX} {lookY})" class="eyes">
        {#if eyesClosed}
          <path d="M72 108 q9 7 18 0" stroke="#1d1a2e" stroke-width="4" fill="none" stroke-linecap="round" />
          <path d="M110 108 q9 7 18 0" stroke="#1d1a2e" stroke-width="4" fill="none" stroke-linecap="round" />
        {:else if eyesHappy}
          <path d="M71 110 q10 -12 20 0" stroke="#1d1a2e" stroke-width="4.5" fill="none" stroke-linecap="round" />
          <path d="M109 110 q10 -12 20 0" stroke="#1d1a2e" stroke-width="4.5" fill="none" stroke-linecap="round" />
        {:else}
          <g class="lids" class:blink style="transform-origin: 100px 104px">
            <ellipse cx="81" cy="104" rx={mood === "curious" ? 10.5 : 9} ry={mood === "curious" ? 14 : 12} fill="#1d1a2e" />
            <ellipse cx="119" cy="104" rx={mood === "curious" ? 10.5 : 9} ry={mood === "curious" ? 14 : 12} fill="#1d1a2e" />
            <circle cx="84.5" cy="98" r="3.6" fill="#fff" />
            <circle cx="122.5" cy="98" r="3.6" fill="#fff" />
            <circle cx="78" cy="110" r="1.6" fill="#fff" opacity="0.7" />
            <circle cx="116" cy="110" r="1.6" fill="#fff" opacity="0.7" />
          </g>
        {/if}
      </g>

      <!-- mouth -->
      {#if mood === "happy"}
        <path d="M90 128 q10 12 20 0 z" fill="#1d1a2e" />
        <path d="M95 133 q5 3 10 0" fill="#ff8fb8" />
      {:else if mood === "curious"}
        <ellipse cx="100" cy="131" rx="4.5" ry="5.5" fill="#1d1a2e" />
      {:else if mood === "sleeping"}
        <path d="M94 130 q6 3 12 0" stroke="#1d1a2e" stroke-width="3" fill="none" stroke-linecap="round" />
      {:else}
        <path d="M92 127 q8 8 16 0" stroke="#1d1a2e" stroke-width="3.5" fill="none" stroke-linecap="round" />
      {/if}
    </g>
  </g>

  {#if mood === "sleeping"}
    <g class="zzz" fill="var(--text-2)" font-family="Segoe UI Variable Display, Segoe UI" font-weight="700">
      <text x="150" y="64" font-size="18">z</text>
      <text x="162" y="46" font-size="14">z</text>
      <text x="172" y="30" font-size="10">z</text>
    </g>
  {/if}

  {#if mood === "sweeping"}
    <g class="dust">
      {#each [0, 1, 2, 3, 4, 5] as i}
        <circle cx={60 + i * 16} cy="176" r={3 + (i % 3)} fill="#c9c2ff" style="animation-delay: {i * 0.18}s" />
      {/each}
    </g>
  {/if}

  {#if mood === "happy"}
    <g class="sparkles" fill="#ffd36e">
      <path d="M38 52 l3 8 8 3 -8 3 -3 8 -3 -8 -8 -3 8 -3z" />
      <path d="M160 40 l2.4 6 6 2.4 -6 2.4 -2.4 6 -2.4 -6 -6 -2.4 6 -2.4z" style="animation-delay: .4s" />
      <path d="M170 120 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" style="animation-delay: .8s" />
    </g>
  {/if}

  {#if mood === "curious"}
    <g class="bang">
      <circle cx="160" cy="50" r="16" fill="var(--warn)" />
      <rect x="157.5" y="38" width="5" height="15" rx="2.5" fill="#1d1a2e" />
      <circle cx="160" cy="59" r="2.8" fill="#1d1a2e" />
    </g>
  {/if}
</svg>

<style>
  .mascot {
    overflow: visible;
    display: block;
  }
  .eyes {
    transition: transform 0.25s var(--ease);
  }
  .lids {
    transition: transform 0.08s;
  }
  .lids.blink {
    transform: scaleY(0.1);
  }
  .bob {
    animation: bob 3.2s ease-in-out infinite;
    transform-origin: 100px 160px;
  }
  .shadow {
    animation: shadow 3.2s ease-in-out infinite;
    transform-origin: 100px 182px;
  }
  .aura {
    animation: aura 4s ease-in-out infinite;
    transform-origin: center;
  }
  @keyframes bob {
    0%, 100% { transform: translateY(0) scale(1, 1); }
    50% { transform: translateY(-8px) scale(0.985, 1.015); }
  }
  @keyframes shadow {
    0%, 100% { transform: scale(1); opacity: 0.28; }
    50% { transform: scale(0.85); opacity: 0.18; }
  }
  @keyframes aura {
    0%, 100% { opacity: 0.5; transform: scale(0.95); }
    50% { opacity: 0.85; transform: scale(1.03); }
  }

  .sleeping .bob { animation-duration: 5s; }
  .sleeping .aura { opacity: 0.25; animation: none; }
  .zzz text { animation: zzz 3s ease-in-out infinite; opacity: 0; }
  .zzz text:nth-child(2) { animation-delay: 0.6s; }
  .zzz text:nth-child(3) { animation-delay: 1.2s; }
  @keyframes zzz {
    0% { opacity: 0; transform: translate(0, 6px); }
    30% { opacity: 1; }
    100% { opacity: 0; transform: translate(8px, -10px); }
  }

  .sweeping .wobble {
    animation: wobble 0.7s ease-in-out infinite;
    transform-origin: 100px 165px;
  }
  @keyframes wobble {
    0%, 100% { transform: rotate(-7deg) translateX(-4px); }
    50% { transform: rotate(7deg) translateX(4px); }
  }
  .dust circle {
    animation: puff 1.1s ease-out infinite;
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes puff {
    0% { opacity: 0.9; transform: translate(0, 0) scale(0.6); }
    100% { opacity: 0; transform: translate(var(--dx, 6px), -26px) scale(1.8); }
  }
  .dust circle:nth-child(odd) { --dx: -10px; }

  .happy .bob { animation: hop 0.9s var(--spring) infinite; }
  @keyframes hop {
    0%, 100% { transform: translateY(0) scale(1.03, 0.97); }
    45% { transform: translateY(-16px) scale(0.97, 1.04); }
  }
  .sparkles path {
    animation: twinkle 1.4s ease-in-out infinite;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes twinkle {
    0%, 100% { opacity: 0; transform: scale(0.4) rotate(0deg); }
    50% { opacity: 1; transform: scale(1) rotate(45deg); }
  }

  .bang {
    animation: pop 0.5s var(--spring) both, nod 2.4s ease-in-out 0.5s infinite;
    transform-box: fill-box;
    transform-origin: bottom left;
  }
  @keyframes pop {
    from { transform: scale(0); }
    to { transform: scale(1); }
  }
  @keyframes nod {
    0%, 100% { transform: rotate(0); }
    50% { transform: rotate(10deg); }
  }
  .foot { transition: transform 0.3s; }
  .sweeping .foot.l { animation: step 0.35s ease-in-out infinite alternate; }
  .sweeping .foot.r { animation: step 0.35s ease-in-out infinite alternate-reverse; }
  @keyframes step {
    from { transform: translateY(0); }
    to { transform: translateY(-4px); }
  }
</style>
