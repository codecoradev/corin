# Codecora Hub — Feature Roadmap

> Reference: Uteke features that Hub should integrate with or expose via UI.

## 🔍 Search & Retrieval

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Hybrid Search | Semantic (meaning) + keyword (text). Results merged via RRF algorithm. | Phase 2: use uteke-core recall instead of LIKE |
| Recall Cache | Same query = no re-embed. LRU cache returns previous results instantly. | Phase 2: automatic via uteke-core |
| Time-travel | Recall memory as it was at a past date — "what did I know on June 1?" | Phase 3: timeline UI |

## 🏠 Organisasi Memory

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Namespaces | Isolation per agent. Agent A can't read Agent B's memories. Like separate folders. | ✅ Sidebar → Namespaces view |
| Rooms | Shared workspace for multi-agent. Cross-namespace with author attribution. | ✅ Rooms view (Uteke rooms table) |
| Memory Types | Every memory has a type: fact, procedure, decision, preference, etc. Auto-detected. | Phase 2: show type badge in list/detail |
| Tags & Metadata | Tags for categorization, entity/category for filtering, key:value for structured data. | ✅ Tag display + filter |
| Tiered Memory | Old, rarely-accessed memories auto-become "cold". Cleanup via aging. | Phase 3: aging settings |

## 🔗 Graph & Linking

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Relationship Graph | Memories linked with edges: references, supersedes, contradicts, replies_to. | ✅ Canvas force-directed (tag-based for now) |
| Backlinks | If A references B, B auto-gets backlink to A. Bidirectional. | Phase 2: uteke-core graph API |
| Cosine Auto-Linking | On save, auto-find similar memories and create similar_to edge. | Phase 2: uteke-core integration (#17) |
| Orphan Detection | Find memories with no connections and low importance — cleanup candidates. | Phase 3: maintenance UI |

## 📊 Quality & Intelligence

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Smart Decay | Old memories auto-lose importance. Important ones can be pinned. | Phase 3: decay settings |
| Salience + Recency | Recall results boosted by type (decision > fact > note) and age (new > old). | Phase 2: sort options |
| Citations | Every memory can have source: URL, file, user, or import. Know where it came from. | Phase 2: source field in editor |
| Timeline Events | Audit log per memory: when created, updated, recalled, tagged, etc. | Phase 3: timeline view (#20) |
| Dream Cycle | One-command maintenance: lint → fix backlinks → dedup → find orphans. Like "sleep" for brain. | Phase 3: maintenance button |

## 📝 Document Engine (Wiki)

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Document Engine | Store full markdown documents (not snippets). Auto-chunk per heading, each chunk gets own embedding. Like Obsidian/Outline. | Phase 2: markdown editor (#18) |
| Markdown Chunker | Split documents by # heading. Code blocks not split. | Phase 2: CodeMirror integration |
| Embed-aware Chunking | Chunk size adjusted to embedder token limit (ONNX: 1024 chars, OpenAI: 32K chars). | Phase 2: automatic via uteke-core |

## 🔌 Integration & Server

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| MCP Server | JSON-RPC protocol for AI agents (Claude Desktop, Cursor, Hermes). Via stdio or HTTP. | Phase 3: Hub as MCP viewer |
| Server Mode | Persistent daemon in memory. Recall ~42ms vs ~3s cold start CLI. 75x faster. | Phase 2: Hub reads via server mode |
| Graph API | GET /graph endpoint — returns JSON nodes + edges for graph visualization. | Phase 2: replace direct DB read |
| View-Only API Keys | Read-only tokens for clients that should only read, not write. | Phase 3: settings UI |
| Hermes Plugin | Auto-install plugin to Hermes Agent. Includes room operations. | Phase 3: auto-detect Hermes |

## ⚙️ Infrastructure

| Feature | Description | Hub Integration |
|---------|-------------|-----------------|
| Pluggable Embeddings | Choose backend: ONNX (default, offline), OpenAI, or Ollama. Via config. | Phase 2: settings dropdown |
| Fully Offline | No internet needed. ONNX model local. No telemetry. | ✅ Hub is local-first |
| Single Binary | One binary file. No Docker, Python, database server, or API key needed. | ✅ Hub bundles via Tauri |
| Import/Export | Backup/restore via JSONL. | ✅ Settings → Export JSON/Markdown |
| Configurable Limits | All limits (content length, tags, payload) overridable via env vars or config. | Phase 2: advanced settings |

---

## Phase Mapping

| Phase | Focus | Issues |
|-------|-------|--------|
| **Phase 1 (current)** | Hub MVP: CRUD, graph (tag-based), namespaces, rooms, settings | ✅ Done |
| **Phase 2** | uteke-core integration: hybrid search, real graph edges, auto-linking, document engine | #16, #17, #18 |
| **Phase 3** | Advanced: timeline, maintenance, multi-product dashboard, Hermes plugin | #19, #20, #37 |
