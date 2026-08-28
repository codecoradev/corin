# Corin — AGENTS.md

> Codecora Corin Desktop. Tauri 2 + Svelte 5 + Rust + Uteke.

## Pre-push Checklist (MANDATORY)

Run ALL checks locally before pushing. Never rely on CI to catch issues.

```bash
# Rust
cargo fmt --all -- --check      # format
cargo clippy -- -D warnings     # lint (from src-tauri/)
cargo test                      # tests (from src-tauri/)

# Frontend
npx svelte-check --tsconfig ./tsconfig.json  # type check
npm run test -- --run           # vitest
npm run build                   # production build
```

**If ANY check fails — do NOT push. Fix locally first.**

## Repo Structure

```
src/                  # Svelte 5 frontend (Vite + Svelte 5, no SvelteKit)
src-tauri/            # Rust backend (Tauri)
  src/
    commands.rs       # Tauri commands (IPC handlers)
    uteke_client.rs   # Uteke HTTP API client
    config.rs         # Server URL resolution + general config
    connections/      # Connection profiles (SQLite store, traits, adapters/)
    lib.rs / main.rs  # App setup, command registration, server lifecycle
```

## Conventions

- Default branch: `develop` (protected — PR + checks required)
- Rust edition: 2024
- Commit style: conventional commits (`feat:`, `fix:`, `perf:`, etc.)
- Always format with `cargo fmt --all` — do not manually style Rust code
