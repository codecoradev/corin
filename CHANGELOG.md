# Changelog

All notable changes to CorIn will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] — 2026-06-22

### Changed

- **Binary size optimized**: 19 MB → 6.5 MB (-66%) via aggressive Cargo release profile
  - `strip = true` — remove debug symbols
  - `lto = true` — link-time optimization (cross-crate dead code elimination)
  - `codegen-units = 1` — maximum optimization
  - `opt-level = "z"` — optimize for size
  - `panic = "abort"` — no unwinding machinery
- **AppImage dropped from CI** — 81 MB bundle too large. Linux users use `.deb` or `.rpm`.

### Installer sizes (before → after)

| Platform | v0.1.0 | v0.1.1 |
|----------|--------|--------|
| macOS ARM DMG | 8.8 MB | ~4 MB |
| macOS Intel DMG | 9.2 MB | ~4.5 MB |
| Linux DEB | 7.5 MB | ~3.5 MB |
| Windows MSI | 6.8 MB | ~3 MB |
| Windows NSIS | 4.6 MB | ~2.5 MB |

---

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
- Semantic search (vector + FTS5 hybrid via RRF fusion) — default when server online
- Cosine auto-linking: `similar_to` (≥0.80) and `possible_duplicate` (≥0.92)
- HTTP client to uteke-serve (`uteke_client.rs`)
- 6 server commands: recall, remember, forget, graph, stats, status
- Graph: cosine auto-linked edges when server online (tag-based fallback)
- Model path resolution: reuse `~/.uteke/models/` or standalone `~/.codecora/uteke/models/`
- Dashboard: server status badge with live stats
- Settings: Uteke server connection panel
- Memory list: semantic search with top 5 results + score badges

**Dedup Protection**
- Pre-check before insert: recall target namespace, if score ≥ 0.92 → flag as duplicate
- MemoryEditor: orange warning with existing content preview
- "Save anyway" (force insert) or "Cancel" options

**AI Agent Integration**
- MCP integration guide (`docs/mcp-integration.md`)
- `.mcp.json` project config (HTTP endpoint to uteke-serve)
- 5 MCP tools: remember, recall, list, forget, stats
- Sidebar: server status indicator (green pulse = online, gray = offline)
- Auto-refresh every 30 seconds

**Sidebar**
- Server status always visible (above Settings)
- Green pulsing dot + "Semantic Search" when server online
- Gray dot + "uteke-serve offline" when server not running
- Works on collapsed sidebar (just the dot)

### Security
- Signed installer updates (SHA256 + minisign)
- Read-only Uteke DB access (SQLITE_OPEN_READ_ONLY)
- URL-encoded query parameters (no string interpolation)
- Atomic file writes (temp + rename)
- File permissions: 0600 for model, 0700 for model dir

### Known Limitations
- Graph edges are tag-based unless uteke-serve is running
- Rooms table may be empty (requires `uteke room create`)
- Document engine not yet integrated (Phase 2.5)
- No multi-product dashboard (Phase 3)
- Uteke namespace search bug (#448): recall without namespace only searches "default"
