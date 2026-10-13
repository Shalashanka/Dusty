# Dusty: plan

> A tiny housekeeper that lives in the tray, wanders the filesystem when you're
> not looking, and taps you on the shoulder when it finds something worth your
> attention. It never throws anything away without your say-so, unless you
> told it that it may.

## 1. The problem

Hyperfocus cycles produce a predictable mess:

- Downloads fills with installers, zips, PDFs and datasets used once.
- The desktop becomes a dumping ground.
- Old projects keep gigabytes of `node_modules` / `target` / `.venv`.
- Backups-of-backups (`*-backup-*.bundle`, `_old`, `copy (2)`) accumulate.
- Temp folders, caches and crash dumps grow silently.
- Every few months the only "fix" is a full reinstall.

Dusty makes cleaning **continuous, small and low-friction**: a few
decisions every few days instead of a format once a year.

## 2. Design principles

1. **Never lose data by surprise.** Default action is Recycle Bin. Every action
   is logged with an Undo. Permanent deletion only for regenerable stuff
   (caches, build output), or when explicitly chosen.
2. **Ask little, ask well.** Group findings, explain *why* ("downloaded from
   github.com 5 months ago, never touched since"), make decisions one keypress.
3. **Remember decisions.** "Keep" means keep, and Dusty never asks again about the
   same file. "Snooze" means come back later.
4. **Invisible when working.** Scans run only when the PC is idle, at
   background CPU *and I/O* priority, never during fullscreen/games/presentations.
5. **Earned autonomy.** Each rule can be promoted to *Autopilot*: Dusty acts
   on its own and just reports what it did.
6. **Feels alive.** A mascot with moods (sleeping / sweeping / proud / curious),
   fluid animations, instant UI.

## 3. Architecture

```
┌────────────────────────── Dusty.exe (Tauri 2) ──────────────────────────┐
│                                                                            │
│  Svelte 5 UI (WebView2, Mica)          Rust core                           │
│  ┌──────────────────────┐   IPC    ┌───────────────────────────────────┐   │
│  │ Home · Triage · Space│◄────────►│ commands.rs (Tauri commands)      │   │
│  │ History · Rules      │  events  ├───────────────────────────────────┤   │
│  └──────────────────────┘          │ scheduler   idle-aware, adaptive  │   │
│                                    │ watcher     ReadDirectoryChangesW │   │
│  Tray icon + toasts ◄──────────────│ engine      parallel walk + rules │   │
│                                    │ actions     recycle/move/delete   │   │
│                                    │ store       SQLite (decisions,    │   │
│                                    │             history, first-seen)  │   │
│                                    │ sys         idle, power, disks,   │   │
│                                    │             installed apps, ADS   │   │
│                                    └───────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Why this stack
- **Rust**: fast, safe, direct Win32 access (`windows` crate). Scanning
  uses `jwalk` (rayon-parallel `FindFirstFileEx`) and `xxh3` hashing.
- **Tauri 2 + WebView2**: native window with Mica backdrop, tray, toasts,
  autostart, single instance, ~10 MB binary, near-zero idle CPU. A web UI
  layer is the most practical way to get 2026-grade motion design.
- **Svelte 5**: compiled, tiny runtime, first-class transitions/springs.
- **SQLite (bundled rusqlite)**: one file in `%LOCALAPPDATA%\Dusty`.

### Scheduler ("time-dilated" checks)
- Base tick every 60 s, cheap: reads idle time (`GetLastInputInfo`),
  user notification state (`SHQueryUserNotificationState`: fullscreen, presentation,
  Focus Assist), AC/battery.
- A scan runs when `now >= next_due` **and** the user has been idle ≥ 3 min
  (or the app window is open).
- Interval adapts: after a scan with nothing new, interval ×1.6
  (cap 12 h); filesystem activity in watched folders (watcher) or new
  findings shrink it back toward the floor (30 min).
- Scan threads enter `THREAD_MODE_BACKGROUND_BEGIN`, which lowers CPU, I/O
  and memory priority, so a scan never makes the PC stutter.
- Notifications are rate-limited (default ≤ 1 nudge / 6 h, quiet hours
  22:00–09:00), and only for *new* or *grown* findings.

### Watcher
- `notify` (ReadDirectoryChangesW) on Downloads & Desktop records a
  **first-seen timestamp** per file, which is the real "arrived" date (archives and
  browsers often preserve an old mtime). It also marks the scheduler dirty.

## 4. Detectors (rules)

| # | Detector | What it finds | Default action | Safety |
|---|----------|---------------|----------------|--------|
| 1 | **Stale downloads** | Files in Downloads untouched > 30 d | Recycle | ask |
| 2 | **Used installers** | `.exe/.msi/.msix` in Downloads > 3 d, boosted when a matching program is in the Uninstall registry | Recycle | ask |
| 3 | **Already-extracted archives** | `foo.zip` next to a `foo/` folder | Recycle archive | ask |
| 4 | **Duplicates** | Identical files across Downloads/Desktop/Documents/Pictures (size → 64 KiB xxh3 → full xxh3) | Recycle extras | ask |
| 5 | **Dev junk** | `node_modules`, `target`, `.venv`, `__pycache__`, `.next`, `dist`, `.gradle`… inside projects in `C:\dev` idle > 45 d | Delete (regenerable) | ask / autopilot |
| 6 | **Forgotten backups** | `*.bak`, `*backup*`, `*.bundle`, `*_old`, `copy (n)` | Recycle | ask |
| 7 | **Big & dormant** | Files > 500 MB in the user profile untouched > 90 d | Move to archive | ask |
| 8 | **Desktop clutter** | Desktop with > 15 loose items | Sweep into `Desktop\Swept\YYYY-MM` | ask |
| 9 | **System junk** | `%TEMP%`, `Windows\Temp`, crash dumps, WER reports, shader caches, thumbnail cache (files > 2 d) | Delete | autopilot-able |
| 10 | **Recycle Bin** | Bin > 2 GB or items > 30 d | Empty | ask |
| 11 | **Empty folders** | Empty dirs in watched folders | Delete | autopilot-able |
| 12 | **Old screenshots** | `Pictures\Screenshots` > 60 d | Move to archive | ask |

Every finding carries a **reason** string, **confidence** (0–1), size,
age, and for downloads the **origin URL** read from the
`Zone.Identifier` alternate data stream.

## 5. Actions & safety net
- **Recycle** via `IFileOperation` (shell Recycle Bin). Undo restores it.
- **Move** to a user-chosen archive root (`…\Archive\2026-10\`). Undo moves back.
- **Delete** permanently, only offered for regenerable categories.
- **Keep**: fingerprint (path + size + mtime) stored; never flagged again.
- **Snooze**: 7 / 30 days.
- Every action → `history` row with an undo payload.

## 6. UI / UX

- Frameless window, custom titlebar, Mica backdrop, dark/light follow system,
  accent gradient, Inter/Segoe UI Variable typography, 60 fps motion.
- **Home**: mascot with mood + one-sentence status ("I tidied 12 min ago.
  23 things, 14.2 GB, could go."), drive capacity rings, category cards
  with animated counters, one primary CTA: *Start a 2-minute tidy*.
- **Triage**: a card stack, one decision at a time, made for ADHD:
  `←`/`D` recycle · `→`/`K` keep · `S` snooze · `M` move · `O` open
  folder · `Z` undo. Progress ring, "freed so far" counter, confetti
  at the end. A list mode supports bulk selection.
- **Space**: squarified treemap of any folder with drill-down, colored
  by file type, plus a breadcrumb.
- **History**: timeline of everything done, with Undo buttons.
- **Rules**: per-detector on/off, thresholds, ask vs. autopilot;
  watched folders; schedule & quiet hours; startup with Windows.
- **Tray**: left-click opens, menu: Tidy now / Pause 1 h / Quit. Tray tooltip shows
  pending count.
- **Toasts**: Windows notifications with buttons ("Review", "Later");
  clicking opens straight into Triage, filtered to that finding.
- **Command palette** (`Ctrl+K`) for every action.

## 7. Roadmap

**v0.1 — done**
- Engine with the core detectors, Zone.Identifier origin, installed-apps lookup.
- Idle-aware adaptive scheduler, background I/O priority, watcher + first-seen.
- SQLite store, actions with undo, Home / Tidy / Space / History / Rules, tray, toasts, autostart.

**v0.2 — done**
- Renamed to **Dusty** (mascot with broom, apron and headscarf).
- **Whole-drive file index**: parallel walk of every drive into a compact
  arena (~24 bytes + name per entry), saved to disk, refreshed at idle,
  kept current between rebuilds by recursive watchers. 3.1M files in ~28 s,
  searches in 5–20 ms.
- **Global search popup** (`Ctrl+Alt+Space`, configurable) with a
  natural-language parser: types, sizes, relative/absolute dates, locations.
- **Space** rebuilt as a nested SpaceSniffer-style map, instant from the
  index, with right-click actions and knowledge-base labels.
- **Knowledge base** of 90+ app cache/temp/log locations with safety levels;
  drives the junk detector and Space annotations.
- **Speed**: startup apps (Task Manager's StartupApproved switches, admin via
  UAC for machine-wide ones), live process monitor with leftover/hog flags,
  drive optimize + fragmentation analysis, hibernation file, Delivery
  Optimization, DISM component cleanup, power plans, preinstalled-app removal.
- **Folder policies** (watch / keep empty / keep sorted) with an editable
  organizing plan, file classifier (scripts, installers, screenshots…), and
  learned destination overrides.
- **Project hibernation**: zip without rebuildable junk, verify every entry's
  CRC, replace with a note; undo extracts it back.
- **Unsure ("What's this?")** questions for big forgotten folders; **stale
  projects**; **near-identical photos** (dHash of shell thumbnails, cached).
- **Protection**: don't touch a file/folder for a day/week/month/forever.
- **Learning** from keep/remove decisions (per extension, origin, folder).
- Toast **quick actions** ("Clean the sure ones"), **weekly digest**, new
  startup-app and background-hog alerts.
- Drive selector on Home, grouped list view with smart bulk removal,
  previews (image/video/audio/PDF/text + shell thumbnails), Open / Show in
  folder, interactive onboarding, settings tabs, About, developer mode.

**v0.3 — next**
- **NTFS MFT / USN journal** fast path when Dusty runs with an elevated
  helper service: index rebuilds in seconds and exact incremental updates.
  (The current index works without admin rights.)
- **Ask Dusty**: a tiny local language model (e.g. a ~0.5–1B instruct model
  via llama.cpp, strictly read-only tools over the index) for questions like
  “find the contract I signed with a PDF tool sometime last spring”. The
  rule-based parser stays as the instant first pass.
- Lower index memory (front-coded names, mmap'd arena).
- Scheduled Tasks and services review (third-party auto-start services).
- SMART / drive-health warnings, temperature-aware scanning on laptops.
- Browser download history correlation (what page a file came from).
- Duplicate *folders* (whole trees copied twice), old VM/WSL disk images.
- Per-app "uninstall leftovers" (AppData folders of apps no longer installed).
- Restore-point and shadow-copy size report.
- Quiet "focus" detection (IDE/game in foreground → defer everything).
- Localization (Italian first).
