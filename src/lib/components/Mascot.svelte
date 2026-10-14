<script module lang="ts">
  export type Mood = "idle" | "sleeping" | "sweeping" | "happy" | "curious" | "wave";
</script>

<script lang="ts">
  import { onMount } from "svelte";

  let {
    mood: baseMood = "idle",
    size = 160,
    interactive = true,
    lively = true,
  }: { mood?: Mood; size?: number; interactive?: boolean; lively?: boolean } = $props();

  let el: SVGSVGElement;
  let look = $state({ x: 0, y: 0 });
  let blink = $state(false);
  /** Short-lived reactions (petting, idle fidgets) take over the mood briefly. */
  let override = $state<Mood | null>(null);
  let glance = $state<{ x: number; y: number } | null>(null);
  let hop = $state(false);
  let hearts = $state<{ id: number; x: number; d: number }[]>([]);
  const mood = $derived(override ?? baseMood);
  const uid = Math.random().toString(36).slice(2, 8);
  let overrideTimer: ReturnType<typeof setTimeout>;

  function react(m: Mood, ms: number) {
    clearTimeout(overrideTimer);
    override = m;
    overrideTimer = setTimeout(() => (override = null), ms);
  }

  /** Petting Dusty: a happy hop and a few hearts; pet a lot and Dusty twirls. */
  let pets: number[] = [];
  let twirl = $state(false);
  function pet() {
    if (!interactive) return;
    const t0 = Date.now();
    pets = [...pets.filter((p) => t0 - p < 2500), t0];
    if (pets.length >= 5) {
      pets = [];
      twirl = true;
      setTimeout(() => (twirl = false), 1100);
    }
    react("happy", 1800);
    hop = true;
    setTimeout(() => (hop = false), 600);
    const now = Date.now();
    hearts = [...hearts, ...[0, 1, 2].map((i) => ({ id: now + i, x: 30 + Math.random() * 40, d: i * 120 }))];
    setTimeout(() => (hearts = hearts.filter((h) => h.id < now)), 1800);
  }

  onMount(() => {
    let raf = 0;
    const onMove = (e: PointerEvent) => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        if (!el) return;
        const r = el.getBoundingClientRect();
        const dx = e.clientX - (r.left + r.width / 2);
        const dy = e.clientY - (r.top + r.height * 0.45);
        const d = Math.hypot(dx, dy) || 1;
        const k = Math.min(1, d / 420);
        look = { x: (dx / d) * 4.5 * k, y: (dy / d) * 3.5 * k };
      });
    };
    window.addEventListener("pointermove", onMove);
    let t: ReturnType<typeof setTimeout>;
    const loop = () => {
      blink = true;
      setTimeout(() => (blink = false), 130);
      t = setTimeout(loop, 3000 + Math.random() * 4000);
    };
    t = setTimeout(loop, 2200);

    // Idle fidgets: glance around, a quick sweep, a little hop. Feels alive, never busy.
    let f: ReturnType<typeof setTimeout>;
    const fidget = () => {
      if (lively && !override && (baseMood === "idle" || baseMood === "happy" || baseMood === "curious")) {
        const r = Math.random();
        if (r < 0.4) {
          glance = { x: -4.5, y: 0.5 };
          setTimeout(() => (glance = { x: 4.5, y: 0.5 }), 700);
          setTimeout(() => (glance = null), 1500);
        } else if (r < 0.65) {
          react("sweeping", 2400);
        } else if (r < 0.85) {
          hop = true;
          setTimeout(() => (hop = false), 600);
        } else {
          react("wave", 1800);
        }
      }
      f = setTimeout(fidget, 9000 + Math.random() * 9000);
    };
    f = setTimeout(fidget, 6000 + Math.random() * 6000);
    return () => {
      window.removeEventListener("pointermove", onMove);
      clearTimeout(t);
      clearTimeout(f);
      clearTimeout(overrideTimer);
      cancelAnimationFrame(raf);
    };
  });

  const lookX = $derived(glance ? glance.x : mood === "sweeping" || mood === "sleeping" ? 0 : look.x);
  const lookY = $derived(glance ? glance.y : mood === "sweeping" ? 3 : mood === "sleeping" ? 0 : look.y);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="wrap" class:pettable={interactive} style="width:{size}px;height:{size}px" onclick={pet}>
{#each hearts as h (h.id)}
  <span class="heart" style="left:{h.x}%;animation-delay:{h.d}ms">♥</span>
{/each}
<svg
  bind:this={el}
  class="mascot {mood}"
  class:hopping={hop}
  class:twirl
  width={size}
  height={size}
  viewBox="0 0 200 200"
  role="img"
  aria-label="Dusty, feeling {mood}"
>
  <defs>
    <radialGradient id="body-{uid}" cx="38%" cy="32%" r="75%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="38%" stop-color="#e6e0ff" />
      <stop offset="78%" stop-color="#ab9dff" />
      <stop offset="100%" stop-color="#7d6ef2" />
    </radialGradient>
    <radialGradient id="glow-{uid}" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#8b7cff" stop-opacity="0.5" />
      <stop offset="100%" stop-color="#8b7cff" stop-opacity="0" />
    </radialGradient>
    <pattern id="dots-{uid}" width="9" height="9" patternUnits="userSpaceOnUse">
      <circle cx="4.5" cy="4.5" r="1.6" fill="#fff" opacity="0.85" />
    </pattern>
    <filter id="fluff-{uid}" x="-15%" y="-15%" width="130%" height="130%">
      <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="7" result="n" />
      <feDisplacementMap in="SourceGraphic" in2="n" scale="9" xChannelSelector="R" yChannelSelector="G" />
    </filter>
  </defs>

  <circle class="aura" cx="100" cy="112" r="92" fill="url(#glow-{uid})" />
  <ellipse class="shadow" cx="100" cy="184" rx="46" ry="7" fill="#000" opacity="0.25" />

  <g class="bob">
    <!-- broom, held on the right -->
    <g class="broom">
      <line x1="156" y1="54" x2="132" y2="166" stroke="#a8743f" stroke-width="5.5" stroke-linecap="round" />
      <line x1="156" y1="54" x2="132" y2="166" stroke="#c9945e" stroke-width="2" stroke-linecap="round" opacity="0.7" />
      <path d="M118 162 L146 168 L150 192 L104 186 Z" fill="#f4c66b" />
      <path d="M118 162 L146 168 L147 173 L117 167 Z" fill="#c4473a" />
      <g stroke="#d9a441" stroke-width="1.4" stroke-linecap="round">
        <line x1="111" y1="174" x2="108" y2="186" />
        <line x1="119" y1="173" x2="117" y2="187" />
        <line x1="127" y1="174" x2="126" y2="188" />
        <line x1="135" y1="175" x2="135" y2="189" />
        <line x1="143" y1="176" x2="144" y2="190" />
      </g>
    </g>

    <!-- fuzzy body -->
    <circle cx="96" cy="112" r="57" fill="url(#body-{uid})" filter="url(#fluff-{uid})" />
    <circle cx="96" cy="112" r="52" fill="url(#body-{uid})" />
    <ellipse cx="74" cy="90" rx="14" ry="8" fill="#fff" opacity="0.5" transform="rotate(-25 74 90)" />

    <!-- feet -->
    <ellipse class="foot l" cx="80" cy="166" rx="11" ry="6" fill="#8d7ef5" />
    <ellipse class="foot r" cx="112" cy="166" rx="11" ry="6" fill="#8d7ef5" />

    <!-- apron -->
    <path d="M70 128 Q96 122 122 128 L126 150 Q96 170 66 150 Z" fill="#fff8ee" stroke="#ead9c3" stroke-width="1.2" />
    <path d="M84 129 L84 118 Q96 115 108 118 L108 129" fill="#fff8ee" stroke="#ead9c3" stroke-width="1.2" />
    <rect x="88" y="138" width="16" height="11" rx="3" fill="none" stroke="#e7cfae" stroke-width="1.4" />
    <path d="M70 131 Q60 132 56 128" stroke="#ead9c3" stroke-width="2" fill="none" stroke-linecap="round" />
    <path d="M122 131 Q130 131 134 127" stroke="#ead9c3" stroke-width="2" fill="none" stroke-linecap="round" />
    <circle cx="96" cy="124" r="2" fill="#f2a7b8" />

    <!-- hands -->
    <ellipse cx="52" cy="124" rx="7" ry="6" fill="#a091fb" />
    <g class="hand-r"><ellipse cx="141" cy="122" rx="7" ry="6" fill="#a091fb" /></g>

    <!-- head scarf -->
    <g class="scarf">
      <path d="M46 96 Q50 52 96 52 Q142 52 146 96 Q120 80 96 80 Q72 80 46 96 Z" fill="#ff7a90" />
      <path d="M46 96 Q50 52 96 52 Q142 52 146 96 Q120 80 96 80 Q72 80 46 96 Z" fill="url(#dots-{uid})" />
      <path d="M46 96 Q72 80 96 80 Q120 80 146 96" stroke="#e85d76" stroke-width="2" fill="none" />
      <circle cx="142" cy="74" r="5" fill="#e85d76" />
      <path class="tail a" d="M145 72 Q160 62 166 70 Q156 74 147 76 Z" fill="#ff7a90" />
      <path class="tail b" d="M145 77 Q158 84 160 94 Q150 86 144 80 Z" fill="#ff7a90" />
    </g>

    <!-- cheeks -->
    <ellipse cx="66" cy="112" rx="8" ry="5" fill="#ff8fb8" opacity={mood === "happy" ? 0.75 : 0.45} />
    <ellipse cx="126" cy="112" rx="8" ry="5" fill="#ff8fb8" opacity={mood === "happy" ? 0.75 : 0.45} />

    <!-- eyes -->
    <g transform="translate({lookX} {lookY})" class="eyes">
      {#if mood === "sleeping"}
        <path d="M70 101 q8 6 16 0" stroke="#1d1a2e" stroke-width="3.5" fill="none" stroke-linecap="round" />
        <path d="M106 101 q8 6 16 0" stroke="#1d1a2e" stroke-width="3.5" fill="none" stroke-linecap="round" />
      {:else if mood === "happy" || mood === "wave"}
        <path d="M69 103 q9 -11 18 0" stroke="#1d1a2e" stroke-width="4" fill="none" stroke-linecap="round" />
        <path d="M105 103 q9 -11 18 0" stroke="#1d1a2e" stroke-width="4" fill="none" stroke-linecap="round" />
      {:else}
        <g class="lids" class:blink style="transform-origin: 96px 98px">
          <ellipse cx="78" cy="98" rx={mood === "curious" ? 9.5 : 8.5} ry={mood === "curious" ? 12.5 : 11} fill="#1d1a2e" />
          <ellipse cx="114" cy="98" rx={mood === "curious" ? 9.5 : 8.5} ry={mood === "curious" ? 12.5 : 11} fill="#1d1a2e" />
          <circle cx="81" cy="93" r="3.4" fill="#fff" />
          <circle cx="117" cy="93" r="3.4" fill="#fff" />
          <circle cx="75.5" cy="103" r="1.5" fill="#fff" opacity="0.7" />
          <circle cx="111.5" cy="103" r="1.5" fill="#fff" opacity="0.7" />
        </g>
      {/if}
    </g>

    <!-- mouth -->
    {#if mood === "happy" || mood === "wave"}
      <path d="M87 115 q9 11 18 0 z" fill="#1d1a2e" />
      <path d="M91 119 q5 3 10 0" fill="#ff8fb8" />
    {:else if mood === "curious"}
      <ellipse cx="96" cy="117" rx="4" ry="5" fill="#1d1a2e" />
    {:else if mood === "sleeping"}
      <path d="M91 116 q5 3 10 0" stroke="#1d1a2e" stroke-width="3" fill="none" stroke-linecap="round" />
    {:else}
      <path d="M89 114 q7 7 14 0" stroke="#1d1a2e" stroke-width="3.2" fill="none" stroke-linecap="round" />
    {/if}
  </g>

  {#if mood === "sleeping"}
    <g class="zzz" fill="var(--text-2)" font-family="Segoe UI Variable Display, Segoe UI" font-weight="700">
      <text x="160" y="48" font-size="17">z</text>
      <text x="171" y="32" font-size="13">z</text>
      <text x="180" y="18" font-size="10">z</text>
    </g>
  {/if}

  {#if mood === "sweeping"}
    <g class="dust">
      {#each [0, 1, 2, 3, 4] as i}
        <circle cx={98 + i * 12} cy="186" r={2.5 + (i % 3)} fill="#c9c2ff" style="animation-delay: {i * 0.22}s" />
      {/each}
    </g>
  {/if}

  {#if mood === "happy" || mood === "wave"}
    <g class="sparkles" fill="#ffd36e">
      <path d="M30 60 l3 8 8 3 -8 3 -3 8 -3 -8 -8 -3 8 -3z" />
      <path d="M172 112 l2.4 6 6 2.4 -6 2.4 -2.4 6 -2.4 -6 -6 -2.4 6 -2.4z" style="animation-delay: .7s" />
      <path d="M40 150 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" style="animation-delay: 1.4s" />
    </g>
  {/if}

  {#if mood === "curious"}
    <g class="bang">
      <circle cx="40" cy="58" r="14" fill="var(--warn)" />
      <text x="40" y="64" text-anchor="middle" font-size="17" font-weight="800" fill="#1d1a2e" font-family="Segoe UI Variable Display, Segoe UI">?</text>
    </g>
  {/if}
</svg>
</span>

<style>
  .mascot.twirl.twirl {
    animation: twirl 1.1s cubic-bezier(0.34, 1.3, 0.64, 1);
    transform-origin: 50% 55%;
  }
  @keyframes twirl {
    0% {
      transform: rotate(0) scale(1);
    }
    40% {
      transform: rotate(200deg) scale(1.08);
    }
    100% {
      transform: rotate(360deg) scale(1);
    }
  }
  .wrap {
    position: relative;
    display: inline-block;
  }
  .wrap.pettable {
    cursor: pointer;
  }
  .wrap.pettable:hover .mascot {
    filter: drop-shadow(0 0 14px rgba(157, 140, 255, 0.45));
  }
  .heart {
    position: absolute;
    top: 18%;
    z-index: 2;
    color: #ff7eb6;
    font-size: 18px;
    pointer-events: none;
    animation: heart 1.4s ease-out both;
  }
  @keyframes heart {
    0% {
      opacity: 0;
      transform: translateY(10px) scale(0.5);
    }
    20% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateY(-46px) scale(1.2) rotate(-12deg);
    }
  }
  .mascot.hopping {
    animation: pethop 0.6s cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  @keyframes pethop {
    40% {
      transform: translateY(-14px) scale(1.03, 0.97);
    }
  }
  .mascot {
    overflow: visible;
    display: block;
    transition: filter 0.3s;
  }
  .eyes {
    transition: transform 0.35s var(--ease);
  }
  .lids {
    transition: transform 0.07s;
  }
  .lids.blink {
    transform: scaleY(0.1);
  }
  .bob {
    animation: bob 4.2s ease-in-out infinite;
  }
  .shadow {
    animation: shadow 4.2s ease-in-out infinite;
    transform-origin: 100px 184px;
  }
  .aura {
    animation: aura 6s ease-in-out infinite;
  }
  @keyframes bob {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-5px);
    }
  }
  @keyframes shadow {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.25;
    }
    50% {
      transform: scale(0.9);
      opacity: 0.18;
    }
  }
  @keyframes aura {
    0%,
    100% {
      opacity: 0.55;
    }
    50% {
      opacity: 0.85;
    }
  }
  .tail {
    transform-box: fill-box;
    transform-origin: left center;
    animation: flutter 3s ease-in-out infinite;
  }
  .tail.b {
    animation-delay: 0.4s;
  }
  @keyframes flutter {
    0%,
    100% {
      transform: rotate(0deg);
    }
    50% {
      transform: rotate(-8deg);
    }
  }

  .sleeping .bob {
    animation-duration: 6s;
  }
  .zzz text {
    animation: zzz 3.4s ease-in-out infinite;
    opacity: 0;
  }
  .zzz text:nth-child(2) {
    animation-delay: 0.7s;
  }
  .zzz text:nth-child(3) {
    animation-delay: 1.4s;
  }
  @keyframes zzz {
    0% {
      opacity: 0;
      transform: translate(0, 6px);
    }
    30% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(8px, -10px);
    }
  }

  /* Sweeping: the broom swishes, the body sways gently. */
  .sweeping .broom {
    animation: sweep 1.3s ease-in-out infinite;
    transform-origin: 141px 122px;
  }
  .sweeping .bob {
    animation: sway 1.3s ease-in-out infinite;
  }
  @keyframes sweep {
    0%,
    100% {
      transform: rotate(-9deg);
    }
    50% {
      transform: rotate(9deg);
    }
  }
  @keyframes sway {
    0%,
    100% {
      transform: translateX(-1.5px) rotate(-1deg);
    }
    50% {
      transform: translateX(1.5px) rotate(1deg);
    }
  }
  .dust circle {
    animation: puff 1.4s ease-out infinite;
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes puff {
    0% {
      opacity: 0.8;
      transform: translate(0, 0) scale(0.6);
    }
    100% {
      opacity: 0;
      transform: translate(-10px, -18px) scale(1.6);
    }
  }

  /* Happy: a soft, slow bounce. */
  .happy .bob,
  .wave .bob {
    animation: hop 2.2s ease-in-out infinite;
  }
  @keyframes hop {
    0%,
    60%,
    100% {
      transform: translateY(0);
    }
    30% {
      transform: translateY(-9px);
    }
  }
  .wave .hand-r {
    animation: wave 1.6s ease-in-out infinite;
    transform-box: fill-box;
    transform-origin: left bottom;
  }
  @keyframes wave {
    0%,
    100% {
      transform: rotate(0);
    }
    25% {
      transform: rotate(-25deg) translateY(-4px);
    }
    75% {
      transform: rotate(10deg);
    }
  }
  .sparkles path {
    animation: twinkle 2.2s ease-in-out infinite;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes twinkle {
    0%,
    100% {
      opacity: 0;
      transform: scale(0.4);
    }
    50% {
      opacity: 1;
      transform: scale(1) rotate(45deg);
    }
  }
  .bang {
    animation: pop 0.45s ease-out both;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes pop {
    from {
      transform: scale(0);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
