## [0.3.4] — 2026-07-21

Hotfix release: makes auto-update functional. v0.3.3's release pipeline had
bugs that shipped `latest.json` with an empty `platforms` object, so every
"Check for Updates" errored with "None of the fallback platforms were found."

### Fixed
- **Auto-update manifest** — the release pipeline now correctly uploads each
  platform's signed `.sig` (the artifact glob `target*/release/...` didn't
  cross directory separators, so it missed the macOS cross-compile path;
  switched to `target/**/bundle/...`) and finds it recursively, so
  `latest.json` ships with populated `platforms` for darwin-aarch64,
  darwin-x86_64, and windows-x86_64. The manifest job also fails loud if no
  platforms assemble (#190, #192).

The app code is unchanged from v0.3.3 — this release exists to ship a
working auto-update manifest. All v0.3.3 features (unified search across
memories + documents, HTTP-only version gate, uteke server version in
Settings, clickable codecora.dev, bun tooling) are included.

---

## [0.3.3] — 2026-07-21

Patch release: **auto-update now works**, unified search across memories + documents, and the version gate is fully HTTP-only.

### Added
- **Unified search across memories and documents** — a new "Memories | All" scope toggle in the Memories search bar. "All" runs unified recall (`search_type=all`, uteke 0.9.0+) and shows a mixed list of memory and document hits, each badged; document hits open in the Documents view (#184, #185).

### Fixed
- **Auto-update works again** — every release since v0.2.0 was missing the `latest.json` manifest (tauri-action#1098: "Signature not found for the updater JSON"), so the in-app updater errored with "Could not fetch a valid release JSON from the remote." The release pipeline now generates + uploads `latest.json` itself (each platform build exports its `.sig`; a final job assembles the Tauri v2 manifest) (#187).

### Changed
- **Version detection is fully HTTP-only** — the gate no longer probes the `uteke` CLI; it reads the connected server's `/health` `version` field for both local and remote servers. Completes the HTTP-first direction that fixed the Windows #171 bug in v0.3.2 (#183).
- **uteke server version shown in Settings → Connections** — the health line now displays the server version from `/health`, e.g. `✓ Healthy · uteke v0.9.0 — 12ms` (#186).
- **Tooling switched to bun** — `tauri.conf.json` build hooks and the release runner now use bun instead of npm (#186, #187).

---

## [0.3.2] — 2026-07-20

Patch release: Windows Documents-tab fix, uteke 0.8.x compatibility, and a full dependency sweep (including vite 6→8).

### Fixed
- **Windows: Documents tab always showed "uteke CLI not found"** — the version gate probed `uteke --version` via the CLI, and binary discovery (`find_in_path` / `find_uteke_cli`) did not handle the `.exe` extension or `PATHEXT`, so the gate read "unknown" and blocked Documents even with `uteke.exe` correctly installed. The gate is now **HTTP-first**: it reads the connected server's `GET /health` `version` field (works on every platform, no CLI lookup needed for uteke ≥ 0.7.2). The CLI fallback path is also fixed — `find_in_path` / `find_uteke_cli` / `find_uteke_serve` now try every `PATHEXT` extension on Windows, with cross-platform unit tests for the resolution logic (#171, #180).
- **Uteke 0.8.x room-document compat** — `room_document()` now prefers `POST /room/summary-document` (canonical since uteke #735) and falls back to the legacy `POST /room/document` on HTTP 404, so CorIn works against both the released uteke 0.8.0 and newer (#178).

### Changed
- **Version gate is now HTTP-first** — `resolve_uteke_version` reads the connected uteke-serve's self-reported version via `/health` for local servers too (previously remote-only), aligning the gate with CorIn's HTTP-only architecture. The cached `uteke --version` CLI output remains as a fallback for servers older than 0.7.2 (#180).

### Dependencies
- **vite 6.4.3 → 8.1.5** and **@sveltejs/vite-plugin-svelte 5.1.1 → 7.2.0** — coupled major bump (the plugin peers vite ^8); vite 8 now builds with rolldown, and the production build is faster (#179).
- **tokio** 1.52.3 → 1.53.0 (#175)
- **serde** 1.0.228 → 1.0.229 (#176), **thiserror** 2.0.18 → 2.0.19 (#174), **toml** 1.1.2 → 1.1.3 (#173), **tauri-plugin-dialog** 2.7.1 → 2.7.2 (#177)
- **@codemirror/language** 6.12.3 → 6.12.4 (#169), **dompurify** 3.4.11 → 3.4.12 (#167), **marked** 18.0.5 → 18.0.6 (#166)
- **actions/setup-node** 6 → 7 (#172)

---

## [0.3.1] — 2026-07-09

Stable patch release. Promotes v0.3.1-beta.1 (identical contents) to a full release.

### Added
- **Document tree UX overhaul** — full hierarchy view with folder/file icons, recursive tree, and slide transitions (#157)
- **Clickable breadcrumbs** — every ancestor crumb navigates to its parent/sub-doc; resolves the materialized path client-side with no extra requests (#161)

### Fixed
- **Document tree only showed ~5 docs** — `doc_list` now defaults `limit` to 1000 (uteke-serve's `/doc/list` default of 5 was capping the client-side tree) (#159)
- **Folders couldn't be expanded/collapsed** — Svelte 5 reactivity bug: in-place `Set` mutation + same-ref reassign did not re-render `{@const}` reads; switched to immutable updates (#160)
- **Delete failed with `400 Bad Request`** — require uteke ≥ 0.7.1 (0.7.0 lacks `/doc/delete`); the version gate now rejects 0.7.0 up front with an upgrade prompt (#159)
- **False "uteke upgrade required" for remote users** — version gate used the local CLI version; now probes the remote server via `GET /health` and treats unknown remote versions leniently (#159)
- View scroll reset + document namespace/version gate for uteke v0.7.0 (#154)

### Changed
- Removed the Participants tab from Rooms (#158)
- Release workflow now marks hyphenated tags (e.g. `-beta.1`, `-rc.1`) as GitHub prereleases automatically

---

## [0.3.0] — 2026-07-07

### Added

**Documents Page Overhaul**
- 3-mode toggle: Edit / Split / Preview with live markdown rendering (marked + DOMPurify)
- GFM line breaks enabled (single `\n` → `<br>`)
- Tree sidebar: auto-expand root nodes, recursive tree, Lucide chevron icons, slide transition
- Internal document links (by slug) navigate in-app; external links open in system browser via `@tauri-apps/plugin-shell`
- Save flow: try `/doc/update` first (uteke 0.6.7+), fall back to `/doc/create` (upsert) on 404
- Default to Preview mode for existing docs; Edit mode for new docs
- Word count + reading time in meta bar
- Ctrl+S save shortcut, delete confirmation dialog
- Success/error notification bars (green/red)
- Export `.md` via native OS save dialog (Tauri dialog + fs plugin)
- Scroll reset to top on document switch

**Design System — Shared Component Library (`src/lib/ui/`)**
- Button (4 variants × 3 sizes, press-scale micro-interaction)
- IconButton, Card, Badge (7 Catppuccin colors), Input, Spinner, EmptyState
- Modal (svelte:transition fade/scale, esc close, backdrop)
- ConfirmDialog (reusable, built on Modal + Button)
- Notification (toast system with fly transition) + toastStore
- 31 unit tests (Button, Badge, Spinner, EmptyState, toastStore)

**Animation System (`src/lib/transitions.ts`)**
- 8 shared transition presets: fadeQuick, fadeUp, slideDown, modalScale, dialogPop, backdropFade, expandSlide, slideInRight
- Custom easing: easeOut, easeInOut, springOut
- View switching: cross-fade between views
- Overlay transitions: DetailPanel/SettingsModal fade, MemoryEditor fly-up
- Tree expand: children slide down (svelte/transition)
- Chevron rotate animation, card hover lift, skeleton shimmer

**Tailwind CSS v4 Integration**
- CSS-first config (no tailwind.config.js)
- Catppuccin Mocha palette mapped to `@theme` → Tailwind utilities
- Backward compatible: existing CSS variables preserved

**Component Refactoring (Phase 4)**
- `src/lib/utils/format.ts` — formatDate, formatDuration, relativeTime, getWordCount, getReadingTime (29 tests)
- `src/lib/utils/markdown.ts` — renderMarkdown with GFM + breaks + external link target
- SettingsModal split: 828 → 233 lines (AgentsSection + UpdatesSection extracted)
- RoomsView: extract RoomCreateForm, use shared utils
- GraphView: extract graph-utils.ts (pickColor, buildTagEdges)
- DocumentsView: use shared markdown + format utils

**Dynamic Version + Auto-Update**
- App version display is now dynamic (reads from `@tauri-apps/api/app`)
- Auto-check for updates on Settings open (silent, 3s delay)
- One-click download + install via Tauri updater plugin

**Namespaces Full-Height Layout**
- Tree + detail panels now fill full viewport height (was `max-height: 70vh`)

**Testing Infrastructure**
- Vitest configured with @testing-library/svelte + jest-dom
- Auto-cleanup DOM after each test
- Wrapper component pattern for Svelte 5 snippet children

### Fixed

- `UtekeDoc.title` missing `#[serde(default)]` — `/doc/create` response (`{id, slug}`) failed deserialization
- Layout bug: views used `height: 100%` inside `overflow-y: auto` container → collapsed to 0px
- Nested `<button>` in document tree (invalid HTML, click swallowed)
- External links not opening browser (Tauri webview ignores `<a download>`)
- Save error on older uteke servers (no `/doc/update` route → fallback to upsert)

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

### Changed
- **uteke-core v0.6.4** — pinned to crates.io with `default-features = false` (ONNX gate). Resolves CI build failures on all platforms. No longer uses git branch pin.

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
