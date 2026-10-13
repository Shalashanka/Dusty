# Dusty: mascot brief & image prompts

Use these with an image model (Midjourney, GPT-image, Flux, Ideogram…). Generate the **hero**
first, then the **layered parts** in the same style, so the app can animate Dusty (blink,
breathe, sweep, wave) instead of showing a static picture.

## Character sheet

- **Who:** a tiny, fluffy dust sprite who keeps your PC clean. Diligent, cheerful, a little shy.
- **Body:** round, soft and fuzzy like a dust bunny / cotton ball, slightly wider at the bottom.
  Pale lavender with a white highlight top-left, shading to periwinkle `#7d6ef2` at the edges.
  Fluffy outline (wispy fibres), not a hard circle.
- **Face:** two big glossy black oval eyes with white catchlights, rosy pink cheeks `#ff8fb8`,
  a small happy mouth. Kawaii, but not baby-like.
- **Outfit:** coral-pink headscarf / bandana `#ff7a90` with small white polka dots, tied in a
  little knot on the right side with two flapping tails; a cream apron `#fff8ee` with a tiny pocket.
- **Prop:** a classic straw broom (honey-wood handle `#a8743f`, golden bristles `#f4c66b`,
  red binding), held in the right hand.
- **Little stubby hands and feet**, same lavender.
- **Style:** soft 3D / clay-like render, gentle subsurface glow, studio lighting, no outlines,
  smooth gradients. Friendly, premium, Pixar-ish but minimal. Readable at 32 px.

## 1. Hero image (for the app, store page, README)

```
A cute tiny mascot character named Dusty: a round fluffy dust-bunny sprite made of soft
lavender fuzz, wispy fibers on the silhouette, white highlight on the upper left fading to
periwinkle at the edges. Big glossy black oval eyes with white catchlights, rosy pink cheeks,
small happy smile. Wearing a coral-pink polka-dot headscarf tied with a knot on the right side,
and a cream apron with a little pocket. Holding a classic straw broom with a honey-colored
wooden handle, golden bristles and a red binding. Tiny stubby lavender hands and feet.
Soft 3D clay render, gentle subsurface glow, studio lighting, minimal and premium, Pixar-like
charm, front three-quarter view, centered, full body, transparent background, 4096x4096.
```

Negative / avoid: *scary, realistic dust, dirty, grey, outlines, text, watermark, extra limbs,
human features, busy background.*

## 2. Expressions (same character, same framing)

Append one of these to the hero prompt (keep everything else identical, same seed if possible):

- `idle, calm content smile, eyes open looking at the viewer`
- `happy, eyes closed in upward crescents, open smiling mouth, tiny sparkles around`
- `curious, slightly bigger eyes, small round "o" mouth, a small yellow question-mark bubble`
- `sweeping, leaning into the broom, small dust puffs at the bristles, focused cute expression`
- `sleeping, eyes closed as gentle downward arcs, tiny "z z z" floating, broom leaning against it`
- `waving, right hand raised waving, broom in the left hand, big smile`

## 3. Layered parts for animation (most useful for the app)

Generate on a transparent background, **same camera, same scale, same position**, so they stack:

1. `…only the body (fluffy lavender sphere with cheeks and apron), no eyes, no mouth, no scarf, no broom`
2. `…only the headscarf, positioned exactly on top of the head`
3. `…only the eyes open` and separately `…only the eyes closed (blink frame)`
4. `…only the mouth: smile` / `open happy smile` / `small "o"`
5. `…only the broom held at a 20° angle` (the app rotates it for the sweeping motion)
6. `…only the soft ground shadow ellipse`

With these layers, Dusty can blink, breathe (scale the body 1–2 %), sweep (rotate the broom
around the hand), and switch expressions without a new drawing.

## 4. Sprite sheet (alternative to layers)

```
Sprite sheet, 8 frames in a single row, evenly spaced, same character Dusty (described above)
walking to the right while sweeping with the broom, a smooth looping walk cycle, side three-
quarter view, consistent size and baseline, transparent background, 4096x512.
```

## 5. Hand-off

Drop the files into `src/lib/assets/mascot/` (`hero.png`, `body.png`, `scarf.png`,
`eyes-open.png`, `eyes-closed.png`, `mouth-*.png`, `broom.png`, `shadow.png`, or `walk.png`
sprite sheet). The current SVG mascot (`src/lib/components/Mascot.svelte`) is built from the
same parts, so swapping in the painted layers is a contained change.
