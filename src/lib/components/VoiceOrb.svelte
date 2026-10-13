<script lang="ts">
  import { onMount } from "svelte";

  /** level: 0..1 microphone loudness; state drives the motion style. */
  let { level = 0, state = "listening", height = 110 }: { level?: number; state?: "listening" | "thinking" | "idle"; height?: number } = $props();

  let canvas: HTMLCanvasElement;
  let smooth = 0;

  // Dusty's palette: lavender, mint, pink, sky.
  const RIBBONS = [
    { color: [157, 140, 255], speed: 1.0, freq: 1.6, amp: 1.0, phase: 0 },
    { color: [79, 227, 193], speed: 1.35, freq: 2.2, amp: 0.75, phase: 1.7 },
    { color: [255, 126, 182], speed: 0.8, freq: 1.2, amp: 0.85, phase: 3.1 },
    { color: [90, 176, 255], speed: 1.6, freq: 2.8, amp: 0.55, phase: 4.4 },
  ];

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    let raf = 0;
    const start = performance.now();
    const draw = (now: number) => {
      const dpr = window.devicePixelRatio || 1;
      const w = canvas.clientWidth * dpr;
      const h = canvas.clientHeight * dpr;
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
      }
      const t = (now - start) / 1000;
      const breathe = 0.16 + 0.07 * Math.sin(t * 2.2);
      const target = state === "listening" ? Math.max(breathe, level) : state === "thinking" ? 0.22 + 0.08 * Math.sin(t * 3) : 0.06;
      smooth += (target - smooth) * 0.18;
      ctx.clearRect(0, 0, w, h);
      ctx.globalCompositeOperation = "lighter";
      const mid = h / 2;
      for (const r of RIBBONS) {
        const amp = mid * 0.85 * smooth * r.amp;
        const grad = ctx.createLinearGradient(0, 0, w, 0);
        const [cr, cg, cb] = r.color;
        grad.addColorStop(0, `rgba(${cr},${cg},${cb},0)`);
        grad.addColorStop(0.5, `rgba(${cr},${cg},${cb},0.55)`);
        grad.addColorStop(1, `rgba(${cr},${cg},${cb},0)`);
        ctx.fillStyle = grad;
        ctx.beginPath();
        // A filled ribbon: upper edge left->right, lower edge back.
        const steps = 80;
        const edge = (x: number, sign: number) => {
          const u = x / w;
          const envelope = Math.sin(Math.PI * u) ** 2; // pinch at the ends
          const wave = Math.sin(u * Math.PI * 2 * r.freq + t * 4 * r.speed + r.phase);
          const shimmer = state === "thinking" ? Math.sin(u * 30 - t * 8) * 0.12 : 0;
          return mid + sign * Math.abs(wave + shimmer) * amp * envelope + sign * 1.5 * dpr;
        };
        for (let i = 0; i <= steps; i++) {
          const x = (i / steps) * w;
          i === 0 ? ctx.moveTo(x, edge(x, -1)) : ctx.lineTo(x, edge(x, -1));
        }
        for (let i = steps; i >= 0; i--) {
          const x = (i / steps) * w;
          ctx.lineTo(x, edge(x, 1));
        }
        ctx.closePath();
        ctx.fill();
      }
      // Soft glow core.
      // Soft elliptical glow behind the ribbons.
      ctx.save();
      ctx.translate(w / 2, mid);
      ctx.scale(w / h / 1.6, 1);
      const glow = ctx.createRadialGradient(0, 0, 0, 0, 0, h * 0.5);
      glow.addColorStop(0, `rgba(200,190,255,${0.16 + smooth * 0.3})`);
      glow.addColorStop(1, "rgba(200,190,255,0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(0, 0, h * 0.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
      ctx.globalCompositeOperation = "source-over";
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  });
</script>

<canvas bind:this={canvas} style="height:{height}px" aria-hidden="true"></canvas>

<style>
  canvas {
    width: 100%;
    display: block;
    filter: blur(0.4px) saturate(1.2);
  }
</style>
