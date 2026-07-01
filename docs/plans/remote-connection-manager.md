# Plan: Remote Connection Manager (Trait-based Adapters)

**Goal:** Connect CorIn to a remote uteke-serve (VPS) with HTTPS + auth, via a
**generic connection layer** that is built once and extended per-product.

**Approach:** Opsi 2 — Trait-based adapters. Generic connection CRUD/persistence/UI
is built ONCE; each product (uteke, trapfall, rungu, …) is a thin adapter.

**Related issues:** #37 (Connection manager for remote products), #21 (Web API),
#19 (Multi-product dashboard).

---

## Current state (facts from codebase)

- `UtekeClient` has **16 methods** (recall, search, list, get, stats, namespaces,
  graph, rooms, remember, forget, room_recall, room_summary, room_document,
  room_delete, room_create, is_available).
- `AppState.uteke_client: Option<UtekeClient>` — accessed via
  `s.uteke_client.clone()` in **24 call sites** across `commands.rs`.
- `setup()` calls `ensure_uteke_server()` → `UtekeClient::new(&server_url)` → stored.
- Settings: `settings` table `(key, value, updated_at)`, `get_settings` /
  `set_settings` commands operate on `HashMap<String,String>`.
- `ensure_uteke_server()` auto-installs + spawns a LOCAL server (wrong for remote).
- `detect_uteke_serve_url()` hardcodes `http://` (no HTTPS).

---

## Target architecture

```
src-tauri/src/
  connections/
    mod.rs          ← registry, ConnectionConfig, ProductType, capabilities
    traits.rs       ← ProductAdapter + MemoryBackend traits
    store.rs        ← load/save/test connections in DB
    adapters/
      mod.rs
      uteke.rs      ← UtekeAdapter: wraps reqwest HTTP client (replaces UtekeClient)
      (trapfall.rs  ← future)
      (rungu.rs     ← future)
  config.rs         ← resolve_uteke_server (extend: HTTPS + env + DB)
  commands/
    (split if needed)
  lib.rs            ← setup(): resolve primary connection, build adapter
```

```
Frontend:
  src/lib/ts/ipc.ts               ← connection.* IPC wrappers
  src/lib/stores/connections.ts   ← connection store (Svelte)
  src/lib/components/
    ConnectionManager.svelte      ← list/add/remove/test connections
    SettingsView.svelte           ← add "Connection" tab
```

---

## Data model

New `connections` table (additive — does not touch existing tables):

```sql
CREATE TABLE IF NOT EXISTS connections (
    id            TEXT PRIMARY KEY,          -- nanoid
    name          TEXT NOT NULL,             -- "Uteke VPS", "Local dev"
    product_type  TEXT NOT NULL,             -- "uteke" | "trapfall" | "rungu"
    url           TEXT NOT NULL,             -- https://uteke.myvps.com
    auth_type     TEXT,                      -- "bearer" | "apikey" | null
    auth_token    TEXT,                      -- stored (chmod 600 on db file)
    metadata      TEXT DEFAULT '{}',         -- JSON: product-specific config
    status        TEXT DEFAULT 'unknown',    -- "connected"|"offline"|"error"
    is_primary    INTEGER DEFAULT 0,         -- active memory backend
    created_at    TEXT,
    last_tested_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_connections_type ON connections(product_type);
```

**Migration:** `init_schema()` already runs `CREATE TABLE IF NOT EXISTS` on every
startup — adding the table is zero-risk and idempotent.

**Default seed:** on first run, insert a row for the existing local uteke:
`(id=auto, name="Local uteke", product_type="uteke", url=<detected>, is_primary=1)`.

---

## Trait design

```rust
// src-tauri/src/connections/traits.rs

#[async_trait]
pub trait ProductAdapter: Send + Sync {
    fn product_type(&self) -> ProductType;
    fn display_name(&self) -> &'static str;
    fn icon(&self) -> &'static str;             // emoji
    fn capabilities(&self) -> Capabilities;
    async fn health_check(&self, cfg: &ConnectionConfig) -> Result<HealthInfo>;
}

#[derive(Clone, Copy)]
pub struct Capabilities {
    pub read: bool,
    pub write: bool,
    pub search: bool,
    pub realtime: bool,   // websocket
}

// Category-specific traits extend ProductAdapter.
// uteke implements MemoryBackend.
#[async_trait]
pub trait MemoryBackend: ProductAdapter {
    async fn recall(&self, q: &str, ns: Option<&str>) -> Result<Vec<Memory>>;
    async fn search(&self, q: &str, ns: Option<&str>) -> Result<Vec<Memory>>;
    async fn remember(&self, content: &str, ...) -> Result<String>;
    async fn forget(&self, id: &str) -> Result<()>;
    async fn list(&self, ...) -> Result<Vec<Memory>>;
    async fn get(&self, id: &str) -> Result<Memory>;
    async fn stats(&self) -> Result<Stats>;
    async fn namespaces(&self) -> Result<Vec<String>>;
    async fn graph(&self, ns: Option<&str>) -> Result<Graph>;
    // room_* methods...
    async fn rooms(&self, ns: Option<&str>) -> Result<Vec<Room>>;
    async fn room_recall(&self, ...) -> ...;
    async fn room_create(&self, ...) -> ...;
    async fn room_delete(&self, id: &str) -> Result<()>;
    async fn room_document(&self, id: &str) -> Result<Value>;
    async fn room_summary(&self, id: &str) -> Result<Value>;
}
```

```rust
// src-tauri/src/connections/mod.rs

#[derive(Clone, Copy, PartialEq)]
pub enum ProductType { Uteke, /* TrapFall, Rungu (future) */ }

pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub url: String,
    pub auth_token: Option<String>,
    pub metadata: serde_json::Value,
}

pub fn build_adapter(cfg: &ConnectionConfig) -> Box<dyn ProductAdapter> {
    match cfg.product_type {
        ProductType::Uteke => Box::new(UtekeAdapter::new(cfg)),
    }
}

/// Returns the primary uteke adapter (the active memory backend).
pub fn build_memory_backend(cfg: &ConnectionConfig) -> Arc<dyn MemoryBackend> {
    Arc::new(UtekeAdapter::new(cfg))
}
```

---

## Migration strategy (do NOT break existing commands)

The 24 call sites do `s.uteke_client.clone()`. Strategy:

1. **Phase 1 — Refactor internally, keep public API:**
   - Rename `UtekeClient` → `UtekeAdapter`, move into `connections/adapters/uteke.rs`.
   - Add `auth_token: Option<String>` field + `.bearer_auth()` on all requests.
   - Keep the SAME method signatures so `commands.rs` call sites compile unchanged.
   - `AppState.uteke_client` stays (rename field optional, do later).

2. **Phase 2 — AppState gets the active connection:**
   - Add `AppState.active_uteke: Option<Arc<dyn MemoryBackend>>`.
   - `setup()` resolves the primary connection from DB → builds adapter → stores.
   - New commands use `active_uteke`; old `uteke_client` call sites migrate
     gradually (or via a thin shim that delegates).

3. **Backward compat:** if no connection row exists yet (fresh install / existing
   user), `setup()` auto-seeds a local uteke connection from the detected URL,
   preserving today's behavior.

---

## Task breakdown (phased)

### Phase 0 — Data layer (no behavior change)
**Files:** `lib.rs` (schema), new `connections/store.rs`
- [ ] Add `connections` table to `init_schema()`.
- [ ] Implement `store.rs`: `list()`, `get(id)`, `insert()`, `update()`, `delete()`,
      `set_primary(id)`, `get_primary()`.
- [ ] Auto-seed local uteke connection on first run if table empty.
- **Accept:** app boots, table exists, one row seeded, existing features unchanged.

### Phase 1 — Trait + UtekeAdapter (internal refactor)
**Files:** new `connections/traits.rs`, `connections/mod.rs`,
`connections/adapters/uteke.rs`; delete/replace `uteke_client.rs`
- [ ] Define `ProductAdapter`, `MemoryBackend`, `Capabilities`, `ProductType`,
      `ConnectionConfig`.
- [ ] Move `UtekeClient` → `UtekeAdapter`, impl `MemoryBackend`.
- [ ] Add `auth_token` field + bearer auth on every HTTP call.
- [ ] Ensure HTTPS works (reqwest default-tls; verify Cargo features).
- **Accept:** `cargo check` passes, all 24 call sites still compile
  (re-export `UtekeClient` as type alias if needed for minimal diff).

### Phase 2 — Config resolution (HTTPS + env + DB)
**Files:** `config.rs`
- [ ] `resolve_uteke_server(conn)` — priority: primary DB connection →
      `UTEKE_SERVER_URL` env → `~/.uteke/*.toml` → default.
- [ ] Read `UTEKE_AUTH_TOKEN` env for token.
- [ ] Keep `detect_uteke_serve_url()` as legacy fallback.
- **Accept:** remote HTTPS URL from DB is used; token attached.

### Phase 3 — Skip local auto-start when remote
**Files:** `lib.rs` (`ensure_uteke_server` / `setup`)
- [ ] If resolved URL host ≠ localhost/127.0.0.1 → skip install+spawn, just
      return the remote URL.
- [ ] `setup()` builds `UtekeAdapter` from resolved config (incl. token).
- **Accept:** with a remote primary connection, no local uteke-serve spawned.

### Phase 4 — Connection commands (generic CRUD)
**Files:** `commands.rs` (or `commands/connections.rs`)
- [ ] `list_connections() -> Vec<ConnectionInfo>`
- [ ] `add_connection(name, product_type, url, auth_token, metadata)`
- [ ] `update_connection(id, ...)`
- [ ] `delete_connection(id)`
- [ ] `test_connection(id) -> HealthInfo { success, latency_ms, version, error }`
- [ ] `set_primary_connection(id)`
- [ ] Register all in `invoke_handler`.
- **Accept:** frontend can list/add/test/set-primary via IPC.

### Phase 5 — Frontend: Connection Manager UI
**Files:** `ipc.ts` (connection.*), new `stores/connections.ts`,
new `ConnectionManager.svelte`, `SettingsView.svelte`
- [ ] `ipc.ts`: `connection.list/add/update/delete/test/setPrimary`.
- [ ] `stores/connections.ts`: reactive list + active connection.
- [ ] `ConnectionManager.svelte`: list cards (icon, name, url, status badge),
      add/edit form (name, type, url, token masked), Test button, Set primary,
      delete with confirm.
- [ ] Add "Connection" tab to `SettingsView.svelte`.
- [ ] Error handling via existing `reportError()` pattern.
- **Accept:** user can add a VPS uteke connection, test it, set as primary,
      and CorIn uses it as the memory backend after restart (or reconnect).

### Phase 6 — Reconnect at runtime + polish
- [ ] `reconnect_connection(id)` command — rebuild adapter live without restart.
- [ ] Status polling (periodic health check → update status badge).
- [ ] Security: confirm db file perms 600; mask token in logs; clear on delete.
- [ ] Docs: `.agent.md` (connection layer rule), `CHANGELOG.md`.
- **Accept:** switch backends without restart; status stays fresh.

---

## Risk analysis

| Risk | Mitigation |
|---|---|
| Breaking 24 `uteke_client` call sites | Phase 1 keeps method signatures identical; type alias shim |
| Token security at rest | Store in sqlite db (file already under ~/.codecora); document chmod 600; defer OS keychain to later |
| HTTPS/TLS not enabled in reqwest | Verify `reqwest = { features = ["default-tls"] }`; test against https endpoint |
| Migration for existing users | Auto-seed local connection on first run; behavior unchanged |
| Concurrent adapter access | `Arc<dyn MemoryBackend>` (cheap clone) — same pattern as current `clone()` |
| Health check blocking UI | `test_connection` is async; UI shows spinner |

---

## Testing strategy

- **Unit:** adapter health_check against mock URLs; config resolution priority.
- **Integration:** local uteke connection works end-to-end (regression); remote
  https+token connection tested against a staging VPS.
- **Manual checklist:**
  - [ ] Fresh install → local connection seeded, recall works.
  - [ ] Add remote connection with wrong token → test fails with clear error.
  - [ ] Set remote as primary → recall hits VPS (verify via server logs).
  - [ ] Switch back to local without restart.
  - [ ] Offline status shown when server down.

---

## Implementation order & effort

| Phase | Effort | Can ship independently? |
|---|---|---|
| 0. Data layer | 🟢 1h | no (foundation) |
| 1. Trait + UtekeAdapter + auth | 🟡 3h | no (internal) |
| 2. Config resolution | 🟡 2h | no (internal) |
| 3. Skip auto-start remote | 🟢 1h | bundles with 4-5 |
| 4. Connection commands | 🟡 2h | no (needs UI) |
| 5. Connection Manager UI | 🟡 3h | **yes — MVP ship** |
| 6. Reconnect + polish | 🟢 2h | yes — follow-up |

**MVP (remote VPS usable):** Phases 0–5 ≈ 11h (~1.5 days).
**Full polish:** + Phase 6 ≈ 2h.

Recommended PR split:
- **PR 1:** Phases 0–3 (backend foundation + remote support, no UI).
- **PR 2:** Phases 4–5 (Connection Manager UI).
- **PR 3:** Phase 6 (reconnect + polish).

---

## What "adding a new product" looks like (future)

To add TrapFall (issue tracker), after this layer exists:

1. Add `ProductType::TrapFall` enum variant — 1 line.
2. Implement `TrapFallAdapter: ProductAdapter` (+ optionally an `IssueTracker`
   category trait) in `connections/adapters/trapfall.rs` — ~1-2h.
3. Register in `build_adapter()` match — 1 line.
4. Done. No changes to connection CRUD, persistence, auth, or UI plumbing.

That is the payoff: ~2h per new product instead of rebuilding the whole stack.