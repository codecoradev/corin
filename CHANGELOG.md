# Changelog

All notable changes to CorIn will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-06-22

### Added

**Core (Phase 1)**
- Memory CRUD: create, read, update, delete memories with tags
- Graph visualization: force-directed canvas with tag-based edges
- Rooms view: read Uteke rooms and room memories
- Settings: theme, namespace, max results, export/import
- Sidebar navigation: Dashboard, Memories, Namespaces, Graph, Rooms, Settings
- Dashboard: stats overview with quick search
- Memory detail: neighbor list with relationship badges
- Memory editor: create/edit with tag input
- Centralized data dir: `~/.codecora/` with auto-init
- Uteke detection: auto-symlink `~/.codecora/uteke/ → ~/.uteke/`
- Release pipeline: multi-platform CI builds (macOS, Linux, Windows)
- Auto-updater: signed installer updates via tauri-plugin-updater

**Uteke Integration (Phase 2)**
- Semantic search (vector + FTS5 hybrid via RRF fusion)
- Cosine auto-linking: `similar_to` (≥0.80) and `possible_duplicate` (≥0.92)
- HTTP client to uteke-serve (`uteke_client.rs`)
- 6 new server commands: recall, remember, forget, graph, stats, status
- Graph: cosine auto-linked edges when server online (tag-based fallback)
- Model path resolution: reuse `~/.uteke/models/` or standalone `~/.codecora/uteke/models/`
- Dashboard: server status badge with live stats
- Settings: Uteke server connection panel
- Memory list: semantic search placeholder indicator

### Security
- Signed installer updates (SHA256 + minisign)
- Read-only Uteke DB access (SQLITE_OPEN_READ_ONLY)
- URL-encoded query parameters
- Atomic file writes (temp + rename)
- File permissions: 0600 for model, 0700 for model dir

### Known Limitations
- Graph edges are tag-based unless uteke-serve is running
- Rooms table may be empty (requires `uteke room create`)
- Document engine not yet integrated (Phase 2.5)
- No multi-product dashboard (Phase 3)
