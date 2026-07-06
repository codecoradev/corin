# Contributing to Corin

Thank you for your interest in contributing! This guide covers the essentials.

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | stable (latest) | `rustup update` |
| Node.js | 22+ | `nvm install 22` |
| npm | bundled with Node.js | |
| Tauri CLI | 2.x | `cargo install tauri-cli` |
| uteke-serve | latest | Auto-started by Corin; install via `cargo install uteke --features serve` |

**Platform dependencies:** Tauri requires platform-specific system libraries (webkit2gtk on Linux, etc.). See [Tauri Prerequisites](https://tauri.app/start/prerequisites/) for your platform.

## Setup

```bash
# Clone
git clone https://github.com/codecoradev/corin.git
cd corin
git checkout develop

# Install frontend deps
npm install

# Run in dev mode (starts uteke-serve automatically)
npm run tauri dev
```

## Project Structure

```
corin/
├── src/                     # Svelte 5 frontend
│   └── lib/
│       ├── components/      # UI components
│       ├── stores/          # Svelte 5 reactive stores
│       └── ts/              # TypeScript types + IPC wrappers
├── src-tauri/               # Rust backend
│   └── src/
│       ├── commands.rs      # Tauri IPC command handlers
│       ├── uteke_client.rs   # Uteke HTTP API client
│       ├── connections/      # Connection manager module
│       └── config.rs         # App configuration
├── docs/                    # Documentation
│   ├── architecture.md      # System architecture
│   ├── roadmap.md           # Feature roadmap
│   └── mcp-integration.md   # MCP setup guide
├── AGENTS.md                # Pre-push checklist (for AI agents & humans)
└── CHANGELOG.md             # Release changelog
```

## Branch Strategy

| Branch | Purpose | Protection |
|--------|---------|------------|
| `develop` | Default branch, all work merges here | PR + CI checks required |
| `main` | Release branch, tags cut from here | PR + CI checks required |
| `feat/<issue>-<name>` | Feature branches | Created from `develop` |

**Never push directly to `develop` or `main`.** Always use a branch + PR.

## Pre-push Checklist

Run ALL checks locally before pushing. See [AGENTS.md](./AGENTS.md) for the full checklist.

```bash
# Rust (from src-tauri/)
cargo fmt --all -- --check      # format
cargo clippy -- -D warnings     # lint
cargo test                      # unit tests

# Frontend (from project root)
npx svelte-check --tsconfig ./tsconfig.json  # type check
npm run test -- --run           # vitest
npm run build                   # production build
```

**If ANY check fails — do NOT push. Fix locally first.**

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add document tree navigation
fix: resolve namespace filter state leak
perf: cache namespace counts with 30s TTL
docs: update architecture diagram
refactor: extract connection adapter trait
```

## Pull Request Process

1. Create a feature branch from `develop`: `git checkout -b feat/123-my-feature`
2. Implement + test locally (run full pre-push checklist)
3. Push and open a PR targeting `develop`
4. CI must pass: type check, build, test, format, clippy, Cora review
5. PR is mergeable when all checks are green

## Adding a New Feature (5-File Pattern)

When adding a new API surface (e.g., documents, tags, kanban), update all 5 files:

| # | File | What to add |
|---|------|-------------|
| 1 | `uteke_client.rs` | Structs + HTTP client methods |
| 2 | `commands.rs` | `#[tauri::command]` wrappers |
| 3 | `lib.rs` | Register in `generate_handler![]` |
| 4 | `types.ts` | TypeScript interfaces |
| 5 | `ipc.ts` | invoke wrappers |

## Release Process

Managed by maintainers. Tags cut from `main` only via release workflow.

## Questions?

Open an [issue](https://github.com/codecoradev/corin/issues) on GitHub.
