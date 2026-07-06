# Corin

> Corin — Codecora Desktop Knowledge Workstation.

Local-first desktop app for managing memories, knowledge graphs, rooms, and documents. Connects to [Uteke](https://github.com/codecoradev/uteke) via HTTP for semantic search, auto-linking, and graph visualization.

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
- [x] Multi-product dashboard — stats overview across connections
- [x] Auto-updater (signed platform updates)
- [x] AI agent detection — detect and generate agent metadata

### Pipeline (v0.3.0)

- [ ] `POST /doc/update` endpoint for document editing
- [ ] Complete Uteke client coverage (tags, pin, timeline, edges UI)
- [ ] Kanban integration (via Hermes dashboard REST API)

## Development

```bash
# Prerequisites: Rust (stable), Node.js 22+, npm
# Also: uteke-serve running (auto-started by Corin)

# Install frontend deps
npm install

# Run in dev mode
npm run tauri dev

# Build for production
npm run tauri build
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
