# Design philosophy

> The house style behind Dusty, written so any future app (and any agent working on one) can
> reuse it. Dusty is the reference implementation; every rule below exists somewhere in its code.

**In one sentence:** small, specialized software that feels *alive* and *kind*, respects
attention like it's scarce (because it is), runs entirely on your machine, and is honest about what it does.

---

## 1. Product principles

1. **Specialized beats general.** Do one job extremely well and say so. Dusty is not an
   assistant that does everything; it's a housekeeper that knows your files. A small, focused
   tool that gives a true answer fast beats a big one that pretends.
2. **Calm technology.** The product lives in the periphery and steps forward only when it has
   something worth your attention. Default to silence. Batch, rate-limit, respect quiet hours,
   fullscreen and Focus Assist.
3. **One decision at a time.** Big piles are paralysing (especially for ADHD brains). Present
   one card, one clear recommendation, one keypress to act. Show progress and celebrate the end.
4. **Explain everything.** Every suggestion says *why* ("Ubisoft Connect is already installed")
   and *how sure* it is. No black boxes, no dark patterns, no fake urgency.
5. **Reversible by default.** Undo for everything that can be undone; the Recycle Bin before
   deletion; a history you can browse. Irreversible actions are labelled plainly
   ("Delete for good") and need a second, deliberate click.
6. **Earned autonomy.** Start by asking. Let the user promote individual behaviours to
   *autopilot* once trust exists. Learn from their decisions, visibly ("you usually keep these").
7. **Local, private, open.** Everything runs on the user's machine: indexing, speech, search.
   No telemetry, no accounts, no cloud round-trips. Updates are signed. Source is open.
   Say this out loud in the UI where it matters ("100% on this PC").
8. **Fast is a feature.** Perceived instant (< 100 ms) for anything interactive. Heavy work runs
   in the background at low CPU/IO priority and never blocks the UI thread.
9. **Delight in the details.** Personality, motion and small surprises make maintenance feel
   like care, not chores. But delight never costs speed or clarity.

## 2. Character

Products with a recurring presence get a **character**. For Dusty: a fluffy lavender dust sprite
with a coral polka-dot headscarf, an apron and a broom: a hard-working little housekeeper.

- **Personality:** diligent, cheerful, modest, a bit shy, never smug or naggy. Works hard while
  you're away; gives a gentle tap on the shoulder when it's worth it.
- **Presence, not interruption:** like noticing the cleaner quietly going about their work. A
  stroll along the bottom of the screen every few hours, a thought bubble on the dashboard, a
  sweeping broom in the tray while busy. Never blocks, never steals focus, never in fullscreen.
- **Moods map to state:** `idle`, `curious` (found things), `sweeping` (working), `happy`
  (all clean / after a win), `sleeping` (paused), `wave` (greeting). The character is a status
  indicator people *like* looking at.
- **Alive, not hyper:** slow breathing (4–6 s cycles), blinks every 3–7 s, eyes follow the
  pointer, an idle fidget every 9–18 s (glance, short sweep, hop, wave). Responds to being petted
  (click → hop + hearts). **Never frantic**: no fast loops, no constant shimmer, no jittery noise.
- **Rendered as SVG** (or layered PNG), so moods are parts swaps, not separate drawings.

## 3. Voice & copy

- First person for the character ("I found…", "I'll tidy while you're away"), second person for the user.
- Short, warm, concrete. Lead with the fact, then the reason, then the offer:
  *"Untouched for 5 months. Downloads you still need usually get opened or moved."*
- Plain words over jargon. "Delete for good" not "Permanently purge". Numbers are human
  (`3.2 GB`, `5 months`), never raw bytes or timestamps.
- Questions when unsure ("What's this? Do you still need it?"), statements when sure.
- Friendly, never cutesy in warnings. Destructive copy is calm and exact.
- Celebrate real wins ("We freed 8 GB this week. Proud of us!"); never guilt-trip.
- Localize the *meaning*, not just the words (Dusty understands Italian queries too).

## 4. Visual language

**Materials.** Native Windows 11 feel: Mica for the main window, Acrylic (subtle blur) for
floating surfaces (search bar, tray menu). Surfaces are translucent tints over the backdrop,
never flat grey slabs.

**Color tokens** (dark first, light derived; see `src/app.css`):

| Token | Dark | Use |
|---|---|---|
| `--bg` / `--bg-solid` | `#16141f` family | window & popups |
| `--surface 1/2/3` | white at 4.5 / 7.5 / 11 % | cards, inputs, hover |
| `--border` / `--border-strong` | white at 7.5 / 14 % | 1 px hairlines |
| `--text / -2 / -3` | `#eeecf6 / #a9a5bd / #6f6b85` | primary / secondary / faint |
| `--accent` | `#8b7cff` lavender | focus, selection, primary info |
| `--accent-2` | `#4fe3c1` mint | success, positive |
| `--grad` | lavender → indigo → mint | the one hero gradient (primary CTA, highlights) |
| `--danger` | `#ff6b81` | removal, errors (soft background by default) |
| `--warn` | `#ffb547` | caution |

- Each category gets **one** hue (used for its icon tile, dot and glow), never full-colour cards.
- Gradients are rare and meaningful: the primary button, the progress bar, the "Autopilot" state.
- Contrast: text on surfaces ≥ 4.5:1; never rely on colour alone (icons + words).

**Typography.** Segoe UI Variable (Display for headings, Text for body); Cascadia for paths.
Sizes: 28–32 page titles, 17 section titles, 14 body, 12–12.5 meta. Tabular numbers for
anything that counts. Units are de-emphasised (`73.6 GB` → number big, unit small).

**Shape.** Radii 8 / 12 / 18 / 26 (`--r-sm` … `--r-xl`). Pills for chips, toggles and buttons.
Cards: 1 px hairline + translucent fill, shadow only when floating.

**Icons.** Lucide, 1.8 px stroke (2.2 when active), 15–20 px. Icon tiles: tinted rounded
square with the category hue at ~16 %.

## 5. Layout & components

- **Shell:** slim icon rail (labels under icons, sliding active pill), frameless window, custom
  title strip (32 px, clickable only where the controls are, fades into content).
- **Home:** character + one-sentence status + one primary action; supporting stats below; then
  categories as cards. Scope selectors (e.g. drive chips) instead of extra pages.
- **Cards for decisions:** header (category chip, confidence), identity (preview + name +
  path), facts as chips, reason in plain sentence, preview area, footer with primary (left),
  secondary actions (centre), keep/positive (right). Keyboard hints on buttons.
- **Lists for bulk:** group by meaningful keys (type, reason, folder, drive), group checkboxes,
  a floating bulk bar that *always* offers the sensible actions (no disappearing buttons).
- **Context menus everywhere:** right-click any object for the same actions as its buttons.
- **Snackbar for results** with Undo; toasts only when the window isn't focused.
- **Empty states are rewards:** the character celebrates, numbers summarise, one next step.
- **Modals** only for multi-step decisions (organizing plans, mirrors). Dimmed, blurred
  backdrop; Escape closes; primary action bottom-right.
- **Floating bar** (Spotlight-like): one input, chips that show how the query was understood,
  results + preview, keyboard footer, a privacy note. Modes (search / commands / voice) share it.

## 6. Motion

| What | Duration | Easing |
|---|---|---|
| Hover / press feedback | 120–200 ms | ease-out; press scales to 0.95–0.97 |
| View change | 360 ms in / 80 ms out | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| Cards entering, staggered | 350–500 ms, +40 ms each | ease-out |
| Card leaving (decision) | 420 ms | flies in the decision's direction + dust puff |
| Selection pill, toggles | 350–450 ms | gentle overshoot allowed (`--spring`) for tiny elements only |
| Character idle | 4–6 s loops | ease-in-out |
| Numbers | 900 ms tween | ease-out |

- Motion explains change (where something went, what is new); it is never decoration for its own sake.
- Large or continuous motion is slow; fast motion is small and brief.
- Honour `prefers-reduced-motion`: drop ambient motion, keep state changes instant.
- Performance budget: compositor-only properties (transform, opacity), cap DOM nodes in
  visualisations (~900), do hover effects in CSS rather than reactive state.

## 7. Interaction

- **Keyboard-first:** every common action has a key (← remove, → keep, ↓ later, Z undo,
  Enter open, O show in folder, 1–6 views, Ctrl+K palette). Global shortcuts are editable.
- **Global reach:** the most useful entry points work without opening the app (search bar,
  command palette, voice, tray menu, toast buttons).
- **Swipe/drag** mirrors keyboard actions with visible stamps ("RECYCLE" / "KEEP").
- **Two-step for permanence:** "Sure? Click again" within 3 s, or a confirm for destructive bulk.
- **Protect & ignore are first-class:** "Don't touch this" for a day/week/month/forever, mirrors, exclusions.
- **Voice:** push a shortcut, speak, see the transcript and how it was understood, get results.
  Voice never *acts*; it searches. Talking back is opt-in.

## 8. Notifications etiquette

- Only for *new* information, rate-limited (default ≥ 6 h apart), never in quiet hours,
  fullscreen, presentations or Focus Assist, never while the user is looking at the app.
- Title states the fact, body gives one example, buttons offer the next step
  ("Clean the sure ones (3, 1.2 GB)" · "Let's look" · "Not now").
- Weekly digest instead of daily noise. Autopilot reports *what it did*, after the fact.

## 9. Performance as design

- UI thread does UI only; every backend call is async/off-thread.
- Background work: idle-aware scheduling, adaptive intervals (stretch when nothing changes),
  background CPU **and** I/O priority, budgets per run (e.g. GB hashed), caches for everything expensive.
- Instant first paint; data streams in; never a blank window (show the shell, retry, explain errors).
- Indexes over scans; incremental updates over rebuilds; measure with real data, not mocks.

## 10. Privacy, security, trust

- Local-first: models, indexes and data stay on the device. No telemetry by default.
- Signed updates only; refuse anything that fails verification.
- Least privilege: ask for admin (UAC) only for the specific action that needs it.
- Show users where their data lives and let them wipe it.
- Open source; the README states exactly what the app reads and writes.

## 11. Accessibility

- Every control reachable by keyboard with visible focus (2 px accent outline).
- Text never below 11 px; meta at 12 px+. Contrast as above.
- Motion reductions honoured. Colour never the only signal. Native-like semantics (roles, labels).

## 12. Checklist for a new app in this family

- [ ] One job, stated in one sentence. What does it *not* do?
- [ ] Character or not? If yes: moods mapped to states, calm idle loop, reacts to the user.
- [ ] Tokens copied from `src/app.css`; Mica/Acrylic; one category hue each; one hero gradient.
- [ ] Home = status sentence + one primary action.
- [ ] Every action explained, reversible or clearly marked, available by key and right-click.
- [ ] Global entry points (hotkey bar / tray menu) for the top 3 actions.
- [ ] Background work idle-aware and low priority; UI never blocks.
- [ ] Notifications rate-limited and respectful; weekly summary over daily nagging.
- [ ] Local-first, no telemetry, signed updates, open source.
- [ ] Reduced motion, keyboard, contrast checked.
- [ ] Copy reviewed: warm, short, concrete, honest.
