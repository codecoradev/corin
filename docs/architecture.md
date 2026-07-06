# Corin — Architecture

> Corin is a Tauri 2 + Svelte 5 + Rust desktop app. All memory data flows through [Uteke](https://github.com/codecoradev/uteke) HTTP API. Local SQLite stores only app settings and connection configs.

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Corin Desktop                             │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Svelte 5 Frontend (Tailwind CSS, CodeMirror 6)           │  │
│  │  Views: Dashboard, MemoryList, GraphView, RoomsView,      │  │
│  │         DocumentsView, ConnectionManager, SettingsModal    │  │
│  │  Stores: memories, ui, cache, connections, pagination      │  │
│  └──────────────┬───────────────────────────────────────────┘  │
│                 │ invoke() — Tauri IPC                          │
│  ┌──────────────▼───────────────────────────────────────────┐  │
│  │  Rust Backend (commands.rs — 62 registered commands)       │  │
│  │                                                             │  │
│  │  ┌─────────────┐  ┌─────────────────────────────────────┐ │  │
│  │  │ Local SQLite │  │ UtekeClient (reqwest HTTP)          │ │  │
│  │  │ (rusqlite)   │  │ → uteke-serve :8767                  │ │  │
│  │  │             │  │ 37 methods: memory, graph, rooms,     │ │  │
│  │  │ App settings│  │ docs, tags, pin, timeline, dream     │ │  │
│  │  │ Connections │  └─────────────────────────────────────┘ │  │
│  │  └─────────────┘                                           │  │
│  │                                                             │  │
│  │  ┌─────────────────────────────────────────────────────┐ │  │
│  │  │ Connection Manager (trait-based adapter layer)         │ │  │
│  │  │ ProductAdapter trait → UtekeAdapter (bearer auth)      │ │  │
│  │  │ 8 commands: list, add, update, delete, test, primary   │ │  │
│  │  └─────────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────┐  ┌──────────────────────────────┐ │
│  │ uteke-serve :8767        │  │ KanbanClient (not wired)      │ │
│  │ Memory + Graph + Docs    │  │ → Hermes dashboard :9119       │ │
│  │ Rooms + Tags + Timeline  │  │ 10 methods, 0 Tauri commands   │ │
│  └──────────────────────────┘  └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Memory CRUD

```
User action (Svelte)
  → ipc.ts invoke wrapper
    → Tauri IPC
      → commands.rs (lock AppState → clone UtekeClient)
        → UtekeClient HTTP request (reqwest → uteke-serve)
          → uteke-serve processes (embed, store, search)
            → Response → CommandResult → IPC → UI update
```

### Connection Manager

```
User adds connection
  → ConnectionManager.svelte form
    → ipc.ts → add_connection command
      → store.rs inserts to local SQLite
        → test_connection validates via HTTP health check
          → Connection stored with status (connected/error)
```

### Document Engine

```
User opens document
  → DocumentsView.svelte → doc_list (tree structure)
    → User clicks doc → doc_get (fetch content)
      → CodeMirror 6 renders markdown
        → User edits → Save → doc_create (upsert by slug)
```

## Backend Modules

| Module | File | Purpose | LOC |
|--------|------|---------|-----|
| Commands | `commands.rs` | 62 Tauri IPC handlers | ~2,844 |
| Uteke Client | `uteke_client.rs` | HTTP client for uteke-serve (37 methods) | ~1,078 |
| Config | `config.rs` | App config, path resolution, uteke URL detection | ~565 |
| Lib | `lib.rs` | Tauri setup, state init, command registration | ~525 |
| Kanban Client | `kanban_client.rs` | HTTP client for Hermes dashboard (10 methods, unused) | ~436 |
| Connections | `connections/` | Trait-based connection manager (4 files) | ~940 |

## Frontend Structure

```
src/
├── lib/
│   ├── ts/
│   │   ├── ipc.ts              # Tauri invoke wrappers (typed)
│   │   └── types.ts            # TypeScript interfaces (mirrors Rust)
│   ├── stores/
│   │   ├── memories.svelte.ts  # Memory state (Svelte 5 runes)
│   │   ├── ui.svelte.ts        # UI state (sidebar, theme)
│   │   ├── cache.svelte.ts     # TTL cache (30s namespaces, 60s stats)
│   │   ├── connections.svelte.ts # Connection list + health polling
│   │   └── pagination.svelte.ts # Generic offset pager
│   └── components/
│       ├── Dashboard.svelte     # Stats overview + quick search
│       ├── MemoryList.svelte    # Memory list + search + pagination
│       ├── MemoryDetail.svelte  # Memory detail view
│       ├── MemoryEditor.svelte  # Create/edit memory modal
│       ├── GraphView.svelte      # Force-directed canvas graph
│       ├── RoomsView.svelte      # Room list + detail (3 tabs)
│       ├── DocumentsView.svelte  # Tree nav + CodeMirror 6 + CRUD
│       ├── ConnectionManager.svelte # Connection cards + CRUD
│       ├── ImportExport.svelte  # Native import/export UI
│       ├── NamespaceFilter.svelte # Multi-select namespace dropdown
│       ├── SettingsModal.svelte  # Popup modal settings
│       ├── Sidebar.svelte        # Nav + server status
│       └── DetailPanel.svelte    # Universal slide-in detail
├── App.svelte                   # App shell + routing
└── app.css                      # Catppuccin Mocha theme
```

## Tauri Command Groups

| Group | Commands | Transport |
|-------|----------|-----------|
| Memory | `remember`, `recall`, `search`, `list`, `forget`, `get_memory` | → Uteke HTTP |
| Graph | `get_graph_data`, `get_neighbors`, `add_edge`, `remove_edge` | → Uteke HTTP |
| Rooms | `list_rooms`, `get_room_summary`, `create_room`, `get_room_document`, `delete_room` | → Uteke HTTP |
| Documents | `doc_list`, `doc_get`, `doc_create`, `doc_search`, `doc_delete`, `doc_move` | → Uteke HTTP |
| System | `stats`, `list_namespaces`, `list_tags`, `get_settings`, `set_settings`, `export_data`, `import_preview`, `import_data`, `init_data_dir` | Mixed |
| AI Agent | `detect_agents`, `generate_agent_md`, `run_dream_cycle`, `get_dream_history` | → Uteke HTTP |
| Uteke Legacy | 14 commands (uteke_available, uteke_get, etc.) | → Uteke HTTP |
| Uteke Server | `uteke_server_status`, `uteke_recall`, `uteke_remember`, `uteke_forget`, `uteke_server_graph`, `uteke_server_stats` | → Uteke HTTP |
| Connection Manager | `list_connections`, `add_connection`, `update_connection`, `delete_connection`, `test_connection`, `set_primary_connection`, `reconnect_connection`, `disconnect_connection` | → Local SQLite |

## Storage Split

| Storage | Engine | Data | Path |
|---------|--------|------|------|
| **Uteke** | uteke-serve (SQLite) | Memories, graph edges, rooms, documents, tags, timeline | `~/.uteke/uteke.db` |
| **Corin local** | rusqlite | App settings, connection configs | `~/.codecora/corin/corin.db` |

## Key Design Decisions

1. **HTTP-only (not in-process library)** — Decoupled releases, shared server, future remote support. Trade-off: ~50ms latency per call.
2. **Trait-based connection manager** — `ProductAdapter` + `MemoryBackend` traits allow future products (Cora, Trapfall) to connect via their own adapters.
3. **serde_json::Value returns** — Commands return generic JSON to avoid complex serialization boilerplate. Frontend types.ts provides type safety.
4. **In-process cache** — Short-TTL cache (30s namespaces, 60s stats) reduces redundant HTTP calls. Invalidated on mutations.
5. **Auto-start uteke-serve** — Corin detects and spawns uteke-serve if not running. Falls back to read-only if unavailable.

## Adding a New Feature (5-File Pattern)

Every new API surface requires changes in all 5 files:

1. **`uteke_client.rs`** — Add structs + HTTP methods on `UtekeClient`
2. **`commands.rs`** — Add `#[tauri::command]` wrappers
3. **`lib.rs`** — Register in `generate_handler![]`
4. **`types.ts`** — TypeScript interfaces mirroring Rust structs
5. **`ipc.ts`** — invoke wrappers in namespace object
