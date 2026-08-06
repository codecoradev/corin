# Corin — Feature Roadmap

> Reference: Uteke features that Corin integrates with or exposes via UI.
> Updated: 2026-07-21 (Uteke v0.9.0, CorIn v0.3.4)

## 🔍 Search & Retrieval

| Feature | Description | Status |
|---------|-------------|--------|
| Hybrid Search | Semantic (meaning) + keyword (text). Results merged via RRF algorithm. | ✅ via Uteke HTTP |
| Recall Cache | Same query = no re-embed. LRU cache returns previous results instantly. | ✅ automatic via uteke-serve |
| Time-travel | Recall memory as it was at a past date. | ⬚ Not yet — needs timeline UI |

## 🏠 Memory Organization

| Feature | Description | Status |
|---------|-------------|--------|
| Namespaces | Isolation per agent/namespace. | ✅ Namespaces view + multi-select filter |
| Rooms | Shared workspace for multi-agent. Cross-namespace with author attribution. | ✅ Rooms view (list, detail, create, delete) |
| Memory Types | Every memory has a type: fact, procedure, decision, preference. Auto-detected. | ⬚ Not yet — needs type badge in UI |
| Tags & Metadata | Tags for categorization. | ✅ Client methods exist, UI pending |
| Tiered Memory | Old, rarely-accessed memories auto-become "cold". | ⬚ Not yet |

## 🔗 Graph & Linking

| Feature | Description | Status |
|---------|-------------|--------|
| Relationship Graph | Memories linked with edges: references, supersedes, contradicts, replies_to. | ✅ Force-directed canvas + namespace filter |
| Backlinks | If A references B, B auto-gets backlink to A. Bidirectional. | ✅ via Uteke graph API |
| Cosine Auto-Linking | On save, auto-find similar memories and create `similar_to` edge. | ✅ automatic via uteke-serve |
| Orphan Detection | Find memories with no connections and low importance. | ✅ via Dream cycle |

## 📊 Quality & Intelligence

| Feature | Description | Status |
|---------|-------------|--------|
| Smart Decay | Old memories auto-lose importance. Important ones can be pinned. | ⬚ Pin/unpin client methods exist, UI pending |
| Salience + Recency | Recall results boosted by type and age. | ✅ via Uteke recall |
| Citations | Every memory has source: URL, file, user, or import. | ⬚ Not yet |
| Timeline Events | Audit log per memory: created, updated, recalled, tagged. | ⬚ Client method exists, UI pending |
| Dream Cycle | One-command maintenance: lint → fix backlinks → dedup → find orphans. | ✅ Dream cycle button + auto-schedule |

## 📝 Document Engine (Wiki)

| Feature | Description | Status |
|---------|-------------|--------|
| Document Engine | Store full markdown documents. Auto-chunk per heading, each chunk gets own embedding. | ✅ Tree nav + CodeMirror 6 + CRUD |
| Markdown Chunker | Split documents by heading. Code blocks not split. | ✅ via uteke-serve |
| Document Update | Update existing document content. | ✅ Save flow tries `POST /doc/update` first, falls back to `/doc/create` (upsert) on 404 for older uteke servers |
| Document Move | Move documents between tree locations. | ✅ `doc_move` command |

## 🔌 Integration & Server

| Feature | Description | Status |
|---------|-------------|--------|
| MCP Server | JSON-RPC for AI agents (Claude Desktop, Cursor, Hermes). | ✅ See `docs/mcp-integration.md` |
| Uteke Server | Persistent daemon. Corin auto-starts `uteke-serve` on launch. | ✅ Auto-start + status indicator |
| Graph API | JSON nodes + edges for graph visualization. | ✅ via Uteke HTTP |
| Connection Manager | Connect to multiple Uteke instances. Trait-based adapter layer. | ✅ Full CRUD + health polling |
| Hermes Dashboard | Kanban integration via REST API. | ⬚ KanbanClient exists, not wired to UI |
| Import/Export | Backup/restore via JSON & Markdown. | ✅ Native import/export UI |

## ⚙️ Infrastructure

| Feature | Description | Status |
|---------|-------------|--------|
| Fully Offline | No internet needed for core features. | ✅ Local-first |
| Single Binary | One binary per platform. No Docker, Python, or DB server needed. | ✅ Tauri bundles |
| Auto-updater | Signed platform updates. | ✅ Tauri updater |
| Configurable Limits | Override content length, tags, payload limits. | ⬚ Via uteke config, not Corin UI |

---

## Phase Mapping

| Phase | Focus | Status |
|-------|-------|--------|
| **Phase 1** (v0.1.0) | MVP: Memory CRUD, graph (tag-based), namespaces, rooms, CI, centralized storage | ✅ Done |
| **Phase 2** (v0.2.0) | Uteke HTTP integration: hybrid search, graph edges, auto-linking, document engine, connection manager, import/export, dream cycle | ✅ Done |
| **Phase 3** (v0.3.0) | UI/UX overhaul: design system, animation, Tailwind v4, component refactoring, document engine, auto-update, unified search | ✅ Done |
| **Phase 4** | Pipeline: tags/pin/timeline UI, kanban integration, multi-product dashboard enhancements, advanced settings | 🔄 Planned |
| **Phase 5** | Mobile companion app | ⬚ Planned |

---

## GitHub Issues

Active development tracked via [codecoradev/corin issues](https://github.com/codecoradev/corin/issues).

| Issue | Title | Phase |
|-------|-------|-------|
| #207 | Adopt uteke 0.8-0.10 endpoints (trust feedback, room/remember, room-doc junction, cross-entity, context) | 4 |
| #206 | `UtekeClient::graph_stats()` dead code calling non-existent endpoint | 4 |
| #116 | Kanban integration via Hermes dashboard | 4 |
| #25 | Mobile companion app | 5 |
| #21 | Web API for remote/mobile access | 5 |
| #20 | Cora review history viewer | 4 |
