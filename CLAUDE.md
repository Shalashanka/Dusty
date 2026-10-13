# Dusty: agent notes

## Hard rules
- NEVER add `Co-Authored-By` AI trailers to commits; never add "Generated with ..." to PRs.
- NEVER mention AI/assistant contribution anywhere (commits, PRs, README, docs, comments).
- Commits are authored solely by Griseld Islamaj.

## Project
- Tauri 2 (Rust, `src-tauri/`) + SvelteKit/Svelte 5 SPA (`src/`), pnpm.
- Plan / roadmap: `docs/PLAN.md`.
- Dev: `pnpm tauri dev`. Release: `pnpm tauri build` (NSIS installer).
- Rust checks: `cargo check` / `cargo test` in `src-tauri/`. Frontend: `pnpm check`.
- Data lives in `%LOCALAPPDATA%\Dusty\dusty.db` (SQLite).
- Safety: default action is Recycle Bin; every action is logged with undo info.
