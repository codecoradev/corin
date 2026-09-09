# Corin

> Corin — Codecora Desktop Knowledge Workstation.

Local-first desktop app for managing memories, knowledge graphs, rooms, and documents. Connects to [Uteke](https://github.com/codecoradev/uteke) via HTTP for semantic search, auto-linking, and graph visualization.

## Install

Download the latest build for your platform from [Releases](https://github.com/codecoradev/corin/releases/latest):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `CorIn_<version>_aarch64.dmg` |
| macOS (Intel) | `CorIn_<version>_x64.dmg` |
| Windows | `CorIn_<version>_x64-setup.exe` |
| Linux (Debian/Ubuntu) | `CorIn_<version>_amd64.deb` |
| Linux (Fedora/RHEL) | `CorIn_<version>-1.x86_64.rpm` |

### macOS: opening the first time (Gatekeeper)

Corin for macOS is signed but **not yet notarized** with an Apple Developer ID, so macOS
Gatekeeper may block the first launch with *"CorIn can't be opened because it is from an
unidentified developer"* — this is expected for apps without notarization and easy to resolve.

**Option 1 — Terminal (fastest, recommended).** After installing the app to `/Applications`,
run this once:

```bash
xattr -rd com.apple.quarantine /Applications/CorIn.app
```

This removes the quarantine flag that Gatekeeper added on download. After that, Corin opens
normally — double-click and go. You only need to do this once per install (and again after an
update if the app was re-downloaded through the browser).

**Option 2 — System Settings (no Terminal).**

1. In **Finder**, locate `CorIn.app` in `/Applications` (don't open it yet)
2. **Control-click** the app → choose **Open** from the menu
3. Click **Open** in the dialog that appears
4. From now on, Corin opens like any other app

If macOS (Sequoia or newer) shows no Open button and says the app "cannot be verified":
**System Settings → Privacy & Security** → scroll to the **Security** section → find the
message about CorIn → click **Open Anyway**, then authenticate with your password or Touch ID.

> **Why this happens:** Gatekeeper quarantines every app downloaded through a browser.
> Notarized apps are verified silently by Apple; Corin's builds ship with an ad-hoc
> signature while official notarization is still on the roadmap, so the first open needs
> one of the steps above. The app itself runs 100% locally — no network service, no
> telemetry. Auto-updates (once installed) do not re-trigger Gatekeeper.

## Stack

| Layer | Tech |
|-------|------|
| **Desktop shell** | [Tauri 2](https://tauri.app/) |
| **Frontend** | [Svelte 5](https://svelte.dev/) + Tailwind CSS |
| **Backend** | Rust |
| **Memory engine** | [Uteke](https://github.com/codecoradev/uteke) (HTTP API via `uteke-serve`) |
| **Local storage** | SQLite (app settings, connection configs only) |
| **Graph** | Canvas force-directed |
| **Search** | Semantic (via Uteke) + FTS5 |
| **Editor** | [CodeMirror 6](https://codemirror.net/) (documents) |

## Features

### Core (v0.1.0)

- [x] Memory CRUD (create, read, update, delete)
- [x] Namespace isolation
- [x] Semantic search (via Uteke)
- [x] Knowledge graph visualization (force-directed canvas)
- [x] Room system (multi-memory shared workspace)
- [x] Dark theme (Catppuccin Mocha)

### Knowledge Engine (v0.2.0)

- [x] Document engine — wiki-style viewer with tree nav, CodeMirror 6 editor, search, CRUD
- [x] Connection manager — connect to multiple Uteke instances (trait-based adapter layer)
- [x] Native import/export (JSON & Markdown via Uteke HTTP API)
- [x] Dream cycle — one-command maintenance (lint, dedup, orphan detection)
- [x] Namespace filter — multi-select dropdown with 3-state checkbox
- [x] Room management — create and delete rooms
- [x] Dashboard stats — namespace overview, recent memories, server status
- [x] Auto-updater (signed platform updates)
- [x] AI agent detection — detect and generate agent metadata

### UI/UX Overhaul (v0.3.0)

- [x] **Design system** — 11 reusable components (`src/lib/ui/`): Button, Card, Modal, Badge, Input, Spinner, EmptyState, ConfirmDialog, Notification + toastStore
- [x] **Animation system** — 8 transition presets, view cross-fades, tree slide-expand, micro-interactions (press-scale, hover-lift, chevron-rotate)
- [x] **Tailwind CSS v4** — CSS-first config with Catppuccin Mocha theme mapping
- [x] **Component refactoring** — monoliths split (SettingsModal 828→233, RoomsView, GraphView, DocumentsView)
- [x] **Shared utils** — `format.ts` + `markdown.ts` with 29 unit tests
- [x] **Documents page** — 3-mode toggle (Edit/Split/Preview), markdown rendering with GFM breaks, internal/external link navigation, tree auto-expand, native `.md` export
- [x] **Auto-update** — silent check on startup, one-click download + install
- [x] **Unified search** — memories + documents in a single search (v0.3.3, requires uteke 0.9.0+)
- [x] **HTTP-only version gate** — no CLI probes, reads server `/health` directly (v0.3.2+)

### Pipeline (next)

- [ ] Complete Uteke client coverage (tags, pin, timeline, edges UI)
- [ ] Adopt uteke 0.8-0.10 endpoints (trust feedback, room/remember, cross-entity) (#207)

## Development

```bash
# Prerequisites: Rust (stable), Node.js 22+, bun
# Also: uteke-serve running (auto-started by CorIn)

# Install frontend deps
bun install

# Run in dev mode
bun run tauri dev

# Build for production
bun run tauri build
```

See [AGENTS.md](./AGENTS.md) for pre-push checklist and conventions.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Svelte 5 Frontend (Tailwind CSS)                       │
│  └── invoke() → Tauri IPC                                │
├─────────────────────────────────────────────────────────┤
│  Rust Backend (commands.rs — 62 Tauri commands)          │
│  ├── Local SQLite (rusqlite) — app settings, connections │
│  ├── UtekeClient (reqwest) → uteke-serve :8767          │
│  │    └── Memory, graph, rooms, docs, tags, timeline    │
│  └── Connection Manager — trait-based multi-product     │
└─────────────────────────────────────────────────────────┘
```

All memory CRUD flows through [Uteke HTTP API](https://codecora.dev/docs/uteke). Local SQLite stores only app settings and connection configs.

## License

MIT
