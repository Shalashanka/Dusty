# Dusty

A tiny housekeeper that lives in your Windows tray. It looks around your PC
while you're away, notices what's gathering dust, and asks before touching anything.

- **Finds**: stale downloads, installers for apps you already have, zips you already
  extracted, duplicates, forgotten backups, `node_modules`/`target`/venvs of idle
  projects, temp files & caches, big dormant files, desktop piles, empty folders,
  a bloated Recycle Bin.
- **Explains** every finding: why, how sure it is, where a download came from.
- **Asks nicely**: a toast now and then, never during games, presentations or quiet hours.
- **Triage** one card at a time with the keyboard (`←` remove, `→` keep, `↓` later,
  `Z` undo) or in bulk.
- **Safe**: Recycle Bin by default, full history with undo, permanent delete only
  for things that rebuild themselves.
- **Autopilot** per rule, once you trust it.
- **Space**: treemap explorer for any folder or drive.

## Develop

```sh
pnpm install
pnpm tauri dev          # app
pnpm dev                # UI only, in a browser with a mocked backend: http://127.0.0.1:1420/?view=triage
cd src-tauri && cargo test
cargo test real_scan -- --ignored --nocapture   # read-only scan of this machine
```

## Build

```sh
pnpm tauri build        # NSIS installer in src-tauri/target/release/bundle/nsis
```

See [docs/PLAN.md](docs/PLAN.md) for the design and roadmap.
