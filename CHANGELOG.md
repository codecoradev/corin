## [0.2.0] — 2026-07-03

### Added

**Connection Edit Form (perf follow-up)**
- Pencil (✎) edit button per connection card → inline form pre-filled with
  name + url. Token field intentionally blank (backend never returns the
  stored token; leaving blank keeps the existing token, typing replaces it).
- Editing the primary connection live-rebuilds the client so URL/token
  changes take effect immediately (no restart).

**Fast Load + Pagination + In-Process Cache (perf)**
- Killed 21-way namespace fan-out: Dashboard recent now uses a single
  cross-namespace `/recall` seed (uteke #448 is fixed for recall); search is
  one call instead of N.
- `stores/cache.svelte.ts` — short-TTL in-process cache (namespaces 30s,
  stats 60s). Invalidated on remember/forget/reconnect/disconnect/set-primary.
  No external infra (Redis) — desktop app stays local-first.
- `stores/pagination.svelte.ts` — generic offset pager; MemoryList +
  NamespacesView use "Load more" instead of fixed-page prev/next.
- NamespacesView: counts are optional ("Load counts" button, limit:1 per ns
  concurrently) instead of sequential limit:500 fetches; detail is paginated.

**Runtime Reconnect + Status Polling (#83)**
- `reconnect_connection(id)` — rebuilds the live `uteke_client` from a connection
  without restarting the app; also health-checks and updates status
- `disconnect_connection` — drops the live client (recall/search fail until
  reconnect); marks primary connection `disconnected`, preserves the row + flag
- `set_primary_connection` now live-swaps the active backend immediately
- "Set Primary" only offered on `connected` connections; "Disconnect" only on
  the connected primary
- `stores/connections.svelte.ts` — reactive store with periodic health polling
  (primary every 15s, all every 60s) so status badges stay fresh
- Reconnect / Disconnect buttons per connection card
- In-app delete confirmation dialog (Tauri webview blocks native `confirm()`)
- Security: db file perms set to `0600` on startup; auth token wiped to NULL
  before row deletion (`clear_token`); tokens masked in logs (`mask_token_log`)

**Remote Connection Manager (#37, #77–#82)**
- `connections` table + trait-based `ProductAdapter` / `MemoryBackend` adapters
- `UtekeAdapter` wraps the HTTP client with bearer auth on every request
- Config resolution priority: DB primary → `UTEKE_SERVER_URL` env → TOML → default
- Local-vs-remote detection skips local auto-start for remote URLs
- Connection Manager UI in Settings: cards, add form, test, set primary, delete

**Namespace Filter for Graph & Memories (#93)**
- Multi-select namespace filter dropdown (checkbox, search, select-all with
  3-state indicator: ☑ all / ☐ none / ⊟ partial) — reusable `NamespaceFilter`
  component shared by Graph and Memories views.
- Per-namespace fan-out on both graph and list endpoints: `null` = all
  namespaces, `[]` = none, `[...]` = explicit selection.
- `/namespaces?with_counts=true` support (uteke #527) with graceful fallback
  to plain `/namespaces` + `count: 0` on older servers.
- Namespaces that error (read-only token, missing, etc.) are silently skipped;
  remaining namespaces still load.
- `AGENTS.md` — project-level pre-push checklist (cargo fmt, clippy, svelte-check).

**Room Management UI (#74)**
- Create room form (name + namespace) with Enter/Escape shortcuts
- 3-tab detail panel: Timeline, Summary, Participants
- Chronological timeline with relative timestamps and namespace badges
- Room document viewer via uteke-serve room/document endpoint
- Participant grouping by namespace with memory counts
- Delete room with confirmation dialog (DELETE /room/delete endpoint)

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
