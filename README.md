<div align="center">

<img src="docs/logo.png" width="112" alt="Dusty" />

# Dusty

**A tiny housekeeper that lives in your Windows tray.**
It quietly looks around while you're away, notices what's gathering dust, and asks before touching anything.

![Windows 10/11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows11&logoColor=white)
![Rust](https://img.shields.io/badge/core-Rust-000000?logo=rust)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/UI-Svelte%205-FF3E00?logo=svelte&logoColor=white)

<img src="docs/screenshots/home.png" width="860" alt="Dusty home" />

</div>

---

## Why

Every few months the same thing happens: Downloads fills with installers and zips used once, the desktop
turns into a pile, old projects hoard gigabytes of `node_modules`, caches grow silently — and the only
"fix" left is reinstalling Windows.

Dusty makes cleaning **continuous, small and painless**: a couple of one-key decisions now and then,
instead of a reinstall once a year. It's designed for people whose attention is elsewhere —
discreet, never naggy, and blazing fast.

## What Dusty does

### 🧹 Finds what you don't need
| | |
|---|---|
| **Stale downloads** | Things nobody opened or moved in a while, in Downloads and any folder you watch |
| **Old installers** | Setup files for apps it can see are already installed |
| **Extracted archives** | `foo.zip` sitting next to the `foo/` folder it was unpacked into |
| **Duplicates** | Byte-identical copies across your folders *and* data drives (hashes cached) |
| **Near-identical photos** | Resized or re-saved copies, via perceptual hashes of Windows thumbnails |
| **Idle project junk** | `node_modules`, `target`, venvs, conda envs, `.gradle`, build output… |
| **Sleepy projects** | Projects untouched for months → **hibernate**: zipped & verified to cold storage, one click to wake |
| **Temp & caches** | 90+ known locations from a built-in knowledge base (Chrome, Premiere, Gradle, pip, NVIDIA, Steam…) with safety notes |
| **"What's this?"** | Big folders in odd places nobody touched in years — Dusty doesn't know, so it asks |
| **Forgotten backups, empty folders, desktop piles, a bloated Recycle Bin** | |

Every finding explains *why*, *how sure* Dusty is, and where a download came from.

### 🗂️ Keeps folders the way you want
Give a folder a **policy** — *Downloads stays empty*, *keep the Desktop sorted* — and Dusty proposes an
**organizing plan**: every item gets a destination (scripts → `Documents\Scripts`, installers → recycled
if already installed, screenshots → `Pictures\Screenshots`…). Edit anything, then one click.
Dusty learns your corrections.

<img src="docs/screenshots/plan.png" width="760" alt="Organizing plan" />

### ⚡ Makes the PC itself lighter
- **Startup apps** — toggle them exactly like Task Manager does; get told when a new one sneaks in
- **Background hogs** — leftover dev servers and build daemons, apps that burn CPU while you're away
- **Drives** — optimize (defrag HDDs, re-trim SSDs), check fragmentation
- **Tune-ups** — hibernation file, Delivery Optimization cache, component store cleanup, power plans
- **Preinstalled apps** — remove the bloat that came with Windows

### 🔎 Finds anything, instantly
A whole-drive index (millions of files, searched in milliseconds) behind a **global search bar**
(`Ctrl+Alt+Space`, configurable). Ask it like a person:

> *"pdf with invoice from about 5 months ago"* · *"big videos on d:"* · *"setup exe in downloads"*

<img src="docs/screenshots/search.png" width="560" alt="Search" />

### 🗺️ Shows where the space went
A SpaceSniffer-style nested map of any drive or folder — instant from the index, with right-click
actions and labels for caches Dusty knows are safe to clear.

<img src="docs/screenshots/space.png" width="860" alt="Space" />

## Designed not to bother you

- Scans only when you're idle, at **background CPU + I/O priority**; check intervals stretch out when nothing changes
- Never pings during games, presentations, Focus Assist or quiet hours; at most one nudge every few hours
- **Quick actions** in notifications ("Clean the sure ones") — no need to open the app
- Default action is the **Recycle Bin**; everything goes into a history with **Undo**
- **"Don't touch this"** for a day, a week, a month or forever — files or whole folders
- Per-rule **Autopilot** once you trust it; **learns** from your keep/remove decisions

<table>
<tr>
<td><img src="docs/screenshots/tidy.png" alt="Tidy, one card at a time" /></td>
<td><img src="docs/screenshots/speed.png" alt="Speed" /></td>
</tr>
<tr>
<td><img src="docs/screenshots/list.png" alt="Grouped list" /></td>
<td><img src="docs/screenshots/onboarding.png" alt="Onboarding" /></td>
</tr>
</table>

## Keyboard

| Where | Keys |
|---|---|
| Anywhere in Windows | `Ctrl+Alt+Space` search |
| App | `1`–`6` switch views · `Ctrl+K` command palette · `Z` undo |
| Tidy | `←` remove · `→` keep · `↓` later · `Enter` open · `O` show in folder · `L` list view |
| Search | `Enter` open · `Ctrl+Enter` show in folder · `Ctrl+C` copy path |

## Architecture

```
Rust core (src-tauri)                         Svelte 5 UI (src)
├─ index      whole-drive file index          ├─ Home, Tidy, Space, Speed, History, Settings
├─ nlq        natural-language queries        ├─ search popup (separate window)
├─ detect/*   detectors                       └─ preview mode with a mocked backend
├─ kb.json    app cache knowledge base
├─ organize   classifier + plans
├─ hibernate  verified zip / restore
├─ health/*   startup, processes, tune-ups
├─ learn      decision learning
├─ scheduler  idle-aware, adaptive, background priority
└─ store      SQLite: history, decisions, caches
```

Data lives in `%LOCALAPPDATA%\com.griz.dusty`.

## Build

Requirements: Rust (stable, MSVC), Node 20+, pnpm, WebView2 (ships with Windows 11).

```sh
pnpm install
pnpm tauri dev            # run the app
pnpm dev                  # UI only, in a browser with a mocked backend:
                          #   http://127.0.0.1:1420/?view=triage   (also: home, space, speed, history, rules, /search)
pnpm tauri build          # installer → src-tauri/target/release/bundle/nsis

cd src-tauri
cargo test                                           # unit tests
cargo test --release real_scan  -- --ignored --nocapture   # read-only scan of this PC
cargo test --release real_index -- --ignored --nocapture   # index all drives, time searches
```

**Buy me a coffee link:** set `coffeeUrl` in [`src/lib/config.ts`](src/lib/config.ts) and rebuild.

See [docs/PLAN.md](docs/PLAN.md) for the design and roadmap, and [docs/TESTING.md](docs/TESTING.md) for the manual test checklist.

---

<div align="center">Made with ♥ by <b>Griseld Islamaj</b></div>
