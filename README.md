# Codecora Hub

> Desktop knowledge workstation powered by [codecora.dev](https://codecora.dev).

Local-first, offline-capable desktop app for managing memories, knowledge graphs, and rooms. Think Obsidian — but with semantic search, auto-linking, and a Rust-native backend.

## Stack

| Layer | Tech |
|-------|------|
| **Desktop shell** | [Tauri 2](https://tauri.app/) |
| **Frontend** | [Svelte 5](https://svelte.dev/) + SvelteKit |
| **Backend** | Rust (uteke-core) |
| **Storage** | SQLite (local-first, zero network) |
| **Graph** | D3 Canvas force-directed |
| **Search** | Semantic embedding + FTS5 |

## Features (Phase 1)

- [x] Memory CRUD (create, read, update, delete)
- [x] Namespace isolation
- [x] Tag filtering
- [x] Semantic search
- [x] Knowledge graph visualization
- [x] Room system (multi-memory documents)
- [x] Dark theme (Catppuccin Mocha)
- [x] Data directory picker

## Roadmap

| Phase | Features | Timeline |
|-------|----------|----------|
| **v0.1** | Memory CRUD, graph, search, rooms | Current |
| **v0.2** | Markdown editor, import/export, uteke-core native | Next |
| **v0.3** | Auto-linking, daily notes, backlinks | Planned |
| **v0.4** | AI assistant (opt-in), summarization | Planned |

## Development

```bash
# Prerequisites: Rust, Node.js 22+, npm

# Install frontend deps
npm install

# Run in dev mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Architecture

```
Svelte 5 Frontend
  └── Tauri IPC (invoke)
       └── Rust Commands (commands.rs)
            └── SQLite (uteke.db)
                 └── memories + graph_edges tables
```

Phase 2 migrates from direct SQLite to `uteke-core` library for full embedding, graph, and FTS5 support.

## License

MIT
