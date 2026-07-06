//! IPC command layer — Tauri commands over uteke-serve HTTP API.
//!
//! Every command is an `async fn` decorated with `#[tauri::command]`.
//! State is held in [`AppState`] behind `tokio::sync::Mutex`.
//!
//! All operations go through the HTTP client (`uteke_client::UtekeClient`).
//! Corin is a pure HTTP client — no native uteke-core dependency.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// CommandError
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("uteke store not initialized — open a data directory first")]
    NotInitialized,
    #[error("memory not found: {0}")]
    NotFound(String),
    #[error("uteke error: {0}")]
    Uteke(String),
    #[error("io error: {0}")]
    Io(String),
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// ---------------------------------------------------------------------------
// Serializable types (frontend-facing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub content_type: Option<String>,
    pub importance: Option<f32>,
    pub namespace: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: Option<i64>,
    pub source: String,
    pub target: String,
    pub weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<MemoryEntry>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEntry {
    pub id: String,
    pub name: String,
    pub participant_count: usize,
    pub memory_count: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsResponse {
    pub total_memories: usize,
    pub total_namespaces: usize,
    pub total_tags: usize,
    pub total_edges: usize,
    pub db_size_bytes: u64,
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

/// Managed state shared across all Tauri commands.
///
/// All memory CRUD and graph operations go through `uteke_client` (HTTP → uteke-serve).
/// Corin is a pure HTTP client with no native uteke-core dependency.
/// CorIn's own SQLite DB is for settings only.
#[derive(Default)]
pub struct AppState {
    /// Path to the CorIn SQLite database (~/.codecora/corin/corin.db).
    pub db_path: Option<PathBuf>,
    /// CorIn SQLite connection for settings only.
    pub conn: Option<rusqlite::Connection>,
    pub data_dir: Option<PathBuf>,
    /// HTTP client for uteke-serve (all memory CRUD + graph operations).
    /// None if server is not running.
    pub uteke_client: Option<crate::uteke_client::UtekeClient>,
}

// ---------------------------------------------------------------------------
// Commands: Memory CRUD
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn remember(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    content: String,
    tags: Vec<String>,
    namespace: Option<String>,
    _content_type: Option<String>,
    _importance: Option<f32>,
) -> Result<String, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    let ns = namespace.as_deref();
    client
        .remember(&content, &tags, ns)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))
}

// NOTE: Auto-edge generation deferred to Phase 2.
// Phase 1 edges are managed via uteke-serve HTTP API (POST /graph/edge).

#[tauri::command]
pub async fn recall(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    let limit = limit.unwrap_or(10);

    let results = client
        .recall(&query, namespace.as_deref(), limit)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            id: r.memory.id,
            content: r.memory.content,
            score: r.score,
            tags: r.memory.tags,
        })
        .collect())
}

#[tauri::command]
pub async fn search(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    // Phase 1: search = alias for recall (FTS5)
    recall(state, query, namespace, limit).await
}

#[tauri::command]
pub async fn list(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    tag: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    let memories = client
        .list(namespace.as_deref(), tag.as_deref(), limit, offset)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(memories
        .into_iter()
        .map(|m| MemoryEntry {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance),
            namespace: Some(m.namespace),
            created_at: Some(m.created_at),
            updated_at: Some(m.updated_at),
        })
        .collect())
}

#[tauri::command]
pub async fn forget(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    client
        .forget(&id)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn get_memory(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<MemoryEntry, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    let m = client
        .get(&id)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(MemoryEntry {
        id: m.id,
        content: m.content,
        tags: m.tags,
        content_type: Some(m.content_type),
        importance: Some(m.importance),
        namespace: Some(m.namespace),
        created_at: Some(m.created_at),
        updated_at: Some(m.updated_at),
    })
}

// ---------------------------------------------------------------------------
// Commands: Graph
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_graph_data(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<GraphData, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    if !client.is_available().await {
        return Ok(GraphData {
            nodes: vec![],
            edges: vec![],
        });
    }

    let limit = limit.unwrap_or(100);
    let gd = client
        .graph(namespace.as_deref())
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Map HTTP graph response to frontend types
    let nodes: Vec<MemoryEntry> = gd
        .nodes
        .into_iter()
        .take(limit)
        .map(|n| MemoryEntry {
            id: n.id,
            content: n.label,
            tags: vec![],
            content_type: n.entity_type,
            importance: None,
            namespace: None,
            created_at: None,
            updated_at: None,
        })
        .collect();

    let edges: Vec<GraphEdge> = gd
        .edges
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(i, e)| GraphEdge {
            id: Some(i as i64),
            source: e.source,
            target: e.target,
            weight: Some(e.weight),
        })
        .collect();

    Ok(GraphData { nodes, edges })
}

#[tauri::command]
pub async fn get_neighbors(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
    depth: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let _depth = depth.unwrap_or(1);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }

    // Use HTTP graph endpoint filtered to neighbors of the given node
    let gd = client
        .graph_neighbors(&id, None)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Collect neighbor node IDs from edges connected to the target node
    let neighbor_ids: std::collections::HashSet<String> = gd
        .edges
        .iter()
        .filter(|e| e.source == id || e.target == id)
        .map(|e| {
            if e.source == id {
                e.target.clone()
            } else {
                e.source.clone()
            }
        })
        .collect();

    let mut entries = Vec::new();
    for nid in neighbor_ids {
        if let Ok(m) = client.get(&nid).await {
            entries.push(MemoryEntry {
                id: m.id,
                content: m.content,
                tags: m.tags,
                content_type: Some(m.content_type),
                importance: Some(m.importance),
                namespace: Some(m.namespace),
                created_at: Some(m.created_at),
                updated_at: Some(m.updated_at),
            });
        }
    }

    Ok(entries)
}

#[tauri::command]
pub async fn add_edge(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    source: String,
    target: String,
    edge_type: Option<String>,
    weight: Option<f32>,
) -> Result<i64, CommandError> {
    // Reject self-loop edges
    if source == target {
        return Err(CommandError::Uteke(
            "self-loop edges are not allowed (source must differ from target)".to_string(),
        ));
    }

    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    // Verify both nodes exist via HTTP
    if client.get(&source).await.is_err() {
        return Err(CommandError::NotFound(source));
    }
    if client.get(&target).await.is_err() {
        return Err(CommandError::NotFound(target));
    }

    // HTTP-only edge mutation via uteke-serve (POST /graph/edge).
    let rel = edge_type.as_deref().unwrap_or("related");
    client
        .add_edge(&source, &target, Some(rel), weight)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(0)
}

#[tauri::command]
pub async fn remove_edge(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    source: String,
    target: String,
    relation: Option<String>,
) -> Result<(), CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    client
        .remove_edge(&source, &target, relation.as_deref())
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))
}

// ---------------------------------------------------------------------------
// Commands: Room
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_rooms(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<RoomEntry>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    let rooms = client
        .rooms(None)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let entries: Vec<RoomEntry> = rooms
        .into_iter()
        .map(|r| RoomEntry {
            id: r.id.clone(),
            name: r.title.unwrap_or_else(|| r.id.clone()),
            participant_count: 0,
            memory_count: 0,
            created_at: Some(r.created_at),
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn get_room_summary(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    }
    .ok_or(CommandError::NotInitialized)?;

    let summary = client
        .room_summary(&room_id)
        .await
        .map_err(CommandError::Uteke)?;

    Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
}

#[tauri::command]
pub async fn create_room(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    name: String,
    namespace: Option<String>,
    _tags: Option<Vec<String>>,
) -> Result<String, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    }
    .ok_or(CommandError::NotInitialized)?;

    let room_id = nanoid::nanoid!(12);
    let ns = namespace.as_deref();
    client
        .room_create(&room_id, Some(&name), ns)
        .await
        .map_err(CommandError::Uteke)?;

    Ok(room_id)
}

#[tauri::command]
pub async fn get_room_document(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    }
    .ok_or(CommandError::NotInitialized)?;

    let doc = client
        .room_document(&room_id)
        .await
        .map_err(CommandError::Uteke)?;

    Ok(format_room_document(&doc))
}

/// Build a human-readable markdown document from the JSON returned by
/// `/room/document`. Defensive: tolerates variant field names
/// (`heading`/`title`/`label`, `items`/`memories`/`entries`).
fn format_room_document(doc: &serde_json::Value) -> String {
    let title = doc
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed");
    let mut out = format!("# Room: {}\n\n", title);

    let sections = doc.get("sections").and_then(|v| v.as_array());
    let is_empty = sections.map(|a| a.is_empty()).unwrap_or(true);
    if is_empty {
        out.push_str("_No memories in this room yet._\n");
    } else {
        for section in sections.unwrap() {
            // Heading: try common keys.
            let heading = section
                .get("heading")
                .or_else(|| section.get("title"))
                .or_else(|| section.get("label"))
                .and_then(|v| v.as_str());
            if let Some(h) = heading {
                out.push_str(&format!("## {}\n\n", h));
            }

            // Items: try common array keys.
            let items = section
                .get("items")
                .or_else(|| section.get("memories"))
                .or_else(|| section.get("entries"))
                .and_then(|v| v.as_array());
            if let Some(items) = items {
                for item in items {
                    let text = item.as_str().map(String::from).unwrap_or_else(|| {
                        // Object item: extract its main text field.
                        item.get("content")
                            .or_else(|| item.get("text"))
                            .or_else(|| item.get("body"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    });
                    if !text.is_empty() {
                        out.push_str(&format!("- {}\n", text));
                    }
                }
                out.push('\n');
            }
        }
    }

    out
}

/// Delete a room by ID.
#[tauri::command]
pub async fn delete_room(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<(), CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    }
    .ok_or(CommandError::NotInitialized)?;

    client
        .room_delete(&room_id)
        .await
        .map_err(CommandError::Uteke)
}

// ---------------------------------------------------------------------------
// Commands: Uteke Integration (read-only)
// ---------------------------------------------------------------------------

/// Check if Uteke server is available (HTTP health check).
#[tauri::command]
pub async fn uteke_available(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(false);
    };
    Ok(client.is_available().await)
}

/// List memories via HTTP (always fresh).
///
/// Supports a multi-namespace filter via `namespaces`. When provided, the
/// command fans out `/list` across the selected namespaces, merges the
/// results (dedup by id), and sorts newest-first. This works on legacy
/// servers (per-namespace `/list`) and lets the UI show memories from
/// several namespaces at once.
#[tauri::command]
pub async fn uteke_list(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    namespaces: Option<Vec<String>>,
    tag: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }

    // Multi-namespace mode: fan out and merge.
    // An explicit empty list means "none" → empty result.
    if let Some(ns_list) = namespaces {
        return list_multi_namespace(&client, &ns_list, tag.as_deref(), limit, offset).await;
    }

    // "All" mode (no namespace scope): legacy /list with no namespace only
    // returns the "default" namespace. Fan out across ALL namespaces so the
    // memories list reflects everything (works on any server version).
    if namespace.is_none() {
        let all_ns = client.namespaces().await.unwrap_or_default();
        if !all_ns.is_empty() {
            return list_multi_namespace(&client, &all_ns, tag.as_deref(), limit, offset).await;
        }
    }

    let memories = client
        .list(namespace.as_deref(), tag.as_deref(), limit, offset)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(memories
        .into_iter()
        .map(|m| MemoryEntry {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance),
            namespace: Some(m.namespace),
            created_at: Some(m.created_at),
            updated_at: Some(m.updated_at),
        })
        .collect())
}

/// Fan out `/list` across multiple namespaces, merge, dedup, sort newest-first.
///
/// To support offset pagination across namespaces without server-side
/// cross-namespace paging (uteke #526), we over-fetch from each namespace
/// and slice after merge. Capped so this stays cheap for a desktop app.
async fn list_multi_namespace(
    client: &crate::uteke_client::UtekeClient,
    namespaces: &[String],
    tag: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<MemoryEntry>, CommandError> {
    // Fetch enough from each namespace to satisfy offset + limit after merge.
    // Cap to avoid huge payloads on large servers.
    const PER_NAMESPACE_CAP: usize = 200;
    let fetch = (offset + limit).min(PER_NAMESPACE_CAP).max(limit);

    let mut all: Vec<crate::uteke_client::UtekeMemory> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ns in namespaces {
        match client.list(Some(ns.as_str()), tag, fetch, 0).await {
            Ok(list) => {
                for m in list {
                    if seen.insert(m.id.clone()) {
                        all.push(m);
                    }
                }
            }
            // Skip inaccessible namespaces (read-only token, missing, etc).
            Err(e) => {
                eprintln!("CorIn: list: skipping namespace '{ns}': {e}");
            }
        }
    }

    // Sort newest-first (created_at is ISO-8601 UTC, so lexical sort works).
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(all
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|m| MemoryEntry {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance),
            namespace: Some(m.namespace),
            created_at: Some(m.created_at),
            updated_at: Some(m.updated_at),
        })
        .collect())
}

/// Get a single memory by ID via HTTP.
#[tauri::command]
pub async fn uteke_get(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<MemoryEntry, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::Uteke("Uteke server not running".into()));
    };
    let m = client
        .get(&id)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(MemoryEntry {
        id: m.id,
        content: m.content,
        tags: m.tags,
        content_type: Some(m.content_type),
        importance: Some(m.importance as f32),
        namespace: Some(m.namespace),
        created_at: Some(m.created_at),
        updated_at: Some(m.updated_at),
    })
}

/// FTS5 keyword search via HTTP.
#[tauri::command]
pub async fn uteke_search(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    let limit = limit.unwrap_or(20);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }
    let results = client
        .search(&query, namespace.as_deref(), limit)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            id: r.memory.id,
            content: r.memory.content,
            score: r.score,
            tags: r.memory.tags,
        })
        .collect())
}

/// Graph data via HTTP (cosine edges from memory_edges).
#[tauri::command]
pub async fn uteke_graph(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    _limit: Option<usize>,
) -> Result<GraphData, CommandError> {
    // ── HTTP-only via uteke-serve ──
    {
        let client = {
            let s = state.lock().await;
            s.uteke_client.clone()
        };
        if let Some(client) = client
            && client.is_available().await
        {
            let graph = client
                .graph(namespace.as_deref())
                .await
                .map_err(|e| CommandError::Uteke(e.to_string()))?;
            return Ok(GraphData {
                nodes: graph
                    .nodes
                    .into_iter()
                    .map(|n| MemoryEntry {
                        id: n.id,
                        content: n.label,
                        tags: vec![],
                        content_type: n.entity_type,
                        importance: None,
                        namespace: None,
                        created_at: None,
                        updated_at: None,
                    })
                    .collect(),
                edges: graph
                    .edges
                    .into_iter()
                    .enumerate()
                    .map(|(i, e)| GraphEdge {
                        id: Some(i as i64),
                        source: e.source,
                        target: e.target,
                        weight: Some(e.weight),
                    })
                    .collect(),
            });
        }
    }

    // No uteke-serve available — return empty graph
    Ok(GraphData {
        nodes: vec![],
        edges: vec![],
    })
}

/// List namespaces via HTTP.
#[tauri::command]
pub async fn uteke_namespaces(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }
    client
        .namespaces()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))
}

/// List namespaces with memory counts (backward-compatible).
///
/// Uses `/namespaces?with_counts=true` on newer uteke servers; falls back
/// to plain `/namespaces` with `count: 0` (unknown) on older ones.
#[tauri::command]
pub async fn uteke_namespaces_with_counts(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<crate::uteke_client::NamespaceCount>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }
    client
        .namespaces_with_counts()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))
}

/// List rooms via HTTP.
///
/// Enriches each room with `memory_count` and `participant_count` via
/// `POST /room/stats` (uteke >= 0.2.1). Falls back to `0` on older
/// servers that lack the endpoint.
#[tauri::command]
pub async fn uteke_rooms(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
) -> Result<Vec<serde_json::Value>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }
    let rooms = client
        .rooms(namespace.as_deref())
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Enrich each room with memory_count and participant_count.
    // Fan-out is fine for a desktop app (rooms are typically < 50).
    let mut enriched = Vec::with_capacity(rooms.len());
    for r in rooms {
        let (mem_count, part_count) = match client.room_stats(&r.id).await {
            Ok(stats) => {
                let mc = stats
                    .get("memory_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                // participant_count: try as u64 first, then as array length
                let pc = stats
                    .get("participant_count")
                    .and_then(|v| v.as_u64())
                    .or_else(|| {
                        stats
                            .get("participant_namespaces")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len() as u64)
                    })
                    .unwrap_or(0) as usize;
                (mc, pc)
            }
            Err(_) => (0, 0), // older server without /room/stats
        };
        enriched.push(serde_json::json!({
            "id": r.id,
            "title": r.title,
            "namespace": r.namespace,
            "memory_count": mem_count,
            "participant_count": part_count,
            "created_at": r.created_at,
            "updated_at": r.updated_at,
        }));
    }
    Ok(enriched)
}

/// Find neighbors via semantic recall (query with memory content).
#[tauri::command]
pub async fn uteke_neighbors(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, CommandError> {
    let limit = limit.unwrap_or(10);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }
    // Get the memory first, use its content as recall query
    let memory = client
        .get(&id)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    let results = client
        .recall(&memory.content, Some(&memory.namespace), limit + 1)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    // Filter out self, map to neighbor format
    Ok(results
        .into_iter()
        .filter(|r| r.memory.id != id)
        .take(limit)
        .map(|r| {
            serde_json::json!({
                "id": r.memory.id,
                "content": r.memory.content,
                "tags": r.memory.tags,
                "namespace": r.memory.namespace,
                "importance": r.memory.importance,
                "content_type": r.memory.content_type,
                "created_at": r.memory.created_at,
                "relationship": if r.score >= 0.92 { "possible_duplicate" } else { "similar_to" },
                "score": r.score,
                "shared_tags": [],
            })
        })
        .collect())
}

/// Room recall via HTTP.
#[tauri::command]
pub async fn uteke_room_recall(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
    limit: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let limit = limit.unwrap_or(20);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    let results = client
        .room_recall(&room_id, limit)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(results
        .into_iter()
        .map(|r| MemoryEntry {
            id: r.memory.id,
            content: r.memory.content,
            tags: r.memory.tags,
            content_type: Some(r.memory.content_type),
            importance: Some(r.memory.importance),
            namespace: Some(r.memory.namespace),
            created_at: Some(r.memory.created_at),
            updated_at: Some(r.memory.updated_at),
        })
        .collect())
}

/// Room stats via HTTP (POST /room/stats).
///
/// Returns authoritative memory_count and participant_count for a room.
#[tauri::command]
pub async fn uteke_room_stats(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<serde_json::Value, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(serde_json::json!({"memory_count": 0, "participant_count": 0}));
    };
    if !client.is_available().await {
        return Ok(serde_json::json!({"memory_count": 0, "participant_count": 0}));
    }
    client
        .room_stats(&room_id)
        .await
        .map_err(CommandError::Uteke)
}

/// Chronological room memories via HTTP (uteke >= 0.6.7, GET /room/memories).
///
/// Returns memories in a room ordered chronologically (newest first).
/// Optional `author` filter restricts to memories from a specific namespace/agent.
/// Falls back to `room_recall` with empty query on older servers that lack the endpoint.
#[tauri::command]
pub async fn uteke_room_memories(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
    limit: Option<usize>,
    author: Option<String>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let limit = limit.unwrap_or(50);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(vec![]);
    };
    if !client.is_available().await {
        return Ok(vec![]);
    }

    // Try GET /room/memories first (uteke >= 0.6.7). Falls back to
    // POST /room/recall with empty query on older servers.
    match client
        .room_memories(&room_id, limit, author.as_deref())
        .await
    {
        Ok(memories) => Ok(memories
            .into_iter()
            .map(|m| MemoryEntry {
                id: m.id,
                content: m.content,
                tags: m.tags,
                content_type: Some(m.content_type),
                importance: Some(m.importance),
                namespace: Some(m.namespace),
                created_at: Some(m.created_at),
                updated_at: Some(m.updated_at),
            })
            .collect()),
        Err(_) => {
            // Fallback: use room_recall with empty query
            let results = client
                .room_recall(&room_id, limit)
                .await
                .map_err(|e| CommandError::Uteke(e.to_string()))?;
            Ok(results
                .into_iter()
                .map(|r| MemoryEntry {
                    id: r.memory.id,
                    content: r.memory.content,
                    tags: r.memory.tags,
                    content_type: Some(r.memory.content_type),
                    importance: Some(r.memory.importance),
                    namespace: Some(r.memory.namespace),
                    created_at: Some(r.memory.created_at),
                    updated_at: Some(r.memory.updated_at),
                })
                .collect())
        }
    }
}

/// Stats via HTTP (always fresh).
#[tauri::command]
pub async fn uteke_stats(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<StatsResponse, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Ok(StatsResponse::default());
    };
    if !client.is_available().await {
        return Ok(StatsResponse::default());
    }
    let client_stats = client
        .stats()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Fetch namespace count separately (not included in /stats response)
    let total_namespaces = client.namespaces().await.unwrap_or_default().len();

    Ok(StatsResponse {
        total_memories: client_stats.total_memories,
        total_namespaces,
        total_tags: client_stats.unique_tags,
        total_edges: 0,
        db_size_bytes: client_stats.db_size_bytes,
    })
}

// ---------------------------------------------------------------------------
// Commands: System
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn stats(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<StatsResponse, CommandError> {
    // HTTP-only via uteke-serve
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    let server_stats = client
        .stats()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    let total_namespaces = client.namespaces().await.unwrap_or_default().len();

    Ok(StatsResponse {
        total_memories: server_stats.total_memories,
        total_namespaces,
        total_tags: server_stats.unique_tags,
        total_edges: 0,
        db_size_bytes: server_stats.db_size_bytes,
    })
}

#[tauri::command]
pub async fn list_namespaces(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };
    let namespaces = client
        .namespaces()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(namespaces)
}

#[tauri::command]
pub async fn list_tags(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    _namespace: Option<String>,
) -> Result<HashMap<String, usize>, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    // No direct HTTP endpoint for tags_with_counts.
    // Approximate with namespaces_with_counts (namespace names as "tags").
    let ns_counts = client.namespaces_with_counts().await.unwrap_or_default();

    let tag_counts: HashMap<String, usize> = ns_counts
        .into_iter()
        .map(|nc| (nc.name, nc.count))
        .collect();

    Ok(tag_counts)
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<HashMap<String, String>, CommandError> {
    let s = state.lock().await;
    let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let settings: HashMap<String, String> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(settings)
}

#[tauri::command]
pub async fn set_settings(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    settings: HashMap<String, String>,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
    let now = chrono::Utc::now().to_rfc3339();
    for (key, value) in settings {
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            rusqlite::params![key, value, now],
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn export_data(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    format: String,
    namespace: Option<String>,
) -> Result<String, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    let ns = namespace.as_deref();
    let memories = client
        .list(ns, None, 100_000, 0)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    match format.as_str() {
        "json" => {
            // Collect edges for exported memories
            let memory_ids: Vec<&str> = memories.iter().map(|m| m.id.as_str()).collect();
            let mut edges_json = Vec::new();
            let mut rooms_json = Vec::new();

            // Fetch edges per memory (batch)
            for id in &memory_ids {
                if let Ok(edge_list) = client.edges(id).await {
                    for e in edge_list.outgoing {
                        if memory_ids.contains(&e.target_id.as_str()) {
                            edges_json.push(serde_json::json!({
                                "source": e.source_id,
                                "target": e.target_id,
                                "type": e.edge_type,
                                "created_at": e.created_at,
                            }));
                        }
                    }
                }
            }

            // Fetch rooms in namespace
            if let Ok(rooms) = client.rooms(ns).await {
                for r in rooms {
                    rooms_json.push(serde_json::json!({
                        "id": r.id,
                        "title": r.title,
                        "namespace": r.namespace,
                        "created_at": r.created_at,
                        "updated_at": r.updated_at,
                    }));
                }
            }

            let export = serde_json::json!({
                "version": "2.0",
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "memories": memories.iter().map(|m| serde_json::json!({
                    "id": m.id,
                    "content": m.content,
                    "tags": m.tags,
                    "namespace": m.namespace,
                    "importance": m.importance,
                    "memory_type": m.memory_type,
                    "content_type": m.content_type,
                    "pinned": m.pinned,
                    "created_at": m.created_at,
                    "updated_at": m.updated_at,
                })).collect::<Vec<_>>(),
                "edges": edges_json,
                "rooms": rooms_json,
            });
            Ok(serde_json::to_string_pretty(&export).unwrap_or_default())
        }
        "markdown" => {
            // Obsidian-compatible: each memory is a separate .md file
            // We return a single string with file boundaries so the frontend
            // can split and save individual files.
            let mut parts = Vec::new();
            for m in &memories {
                let safe_id = m.id.replace(['/', '\\', ':'], "-");
                let mut frontmatter = String::from("---\n");
                frontmatter.push_str(&format!("id: {}\n", m.id));
                if !m.tags.is_empty() {
                    frontmatter.push_str(&format!(
                        "tags: [{}]\n",
                        m.tags
                            .iter()
                            .map(|t| format!("\"{}\"", t))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if !m.namespace.is_empty() {
                    frontmatter.push_str(&format!("namespace: {}\n", m.namespace));
                }
                if !m.created_at.is_empty() {
                    frontmatter.push_str(&format!("created: {}\n", m.created_at));
                }
                if !m.updated_at.is_empty() {
                    frontmatter.push_str(&format!("updated: {}\n", m.updated_at));
                }
                if m.importance > 0.0 {
                    frontmatter.push_str(&format!("importance: {:.2}\n", m.importance));
                }
                if m.pinned {
                    frontmatter.push_str("pinned: true\n");
                }
                frontmatter.push_str("---\n\n");
                parts.push(format!(
                    "<<< FILE:{} >>>\n{}{}\n",
                    safe_id, frontmatter, m.content
                ));
            }
            Ok(parts.join("\n"))
        }
        "csv" => {
            // Flat table: id, content, tags, namespace, importance, type, pinned, created_at
            let mut csv = String::from(
                "id,content,tags,namespace,importance,memory_type,content_type,pinned,created_at,updated_at\n",
            );
            for m in &memories {
                let content = m.content.replace('"', "\"\"");
                let tags = m.tags.join(";").replace('"', "\"\"");
                csv.push_str(&format!(
                    "\"{}\",\"{}\",\"{}\",\"{}\",{:.2},\"{}\",\"{}\",{},\"{}\",\"{}\"\n",
                    m.id,
                    content,
                    tags,
                    m.namespace,
                    m.importance,
                    m.memory_type,
                    m.content_type,
                    m.pinned,
                    m.created_at,
                    m.updated_at,
                ));
            }
            Ok(csv)
        }
        _ => Err(CommandError::Uteke(format!(
            "unsupported export format: {} (use 'json', 'markdown', or 'csv')",
            format
        ))),
    }
}

#[tauri::command]
pub async fn import_preview(
    _state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    format: String,
    data: String,
) -> Result<serde_json::Value, CommandError> {
    match format.as_str() {
        "json" => {
            let parsed: serde_json::Value =
                serde_json::from_str(&data).map_err(|e| CommandError::Uteke(e.to_string()))?;
            let memories = parsed
                .get("memories")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let edges = parsed
                .get("edges")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let rooms = parsed
                .get("rooms")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            // Extract unique namespaces from memories
            let mut namespaces: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            if let Some(arr) = parsed.get("memories").and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(ns) = m.get("namespace").and_then(|v| v.as_str())
                        && !ns.is_empty()
                    {
                        namespaces.insert(ns.to_string());
                    }
                }
            }
            Ok(serde_json::json!({
                "format": "json",
                "memories": memories,
                "edges": edges,
                "rooms": rooms,
                "namespaces": namespaces.into_iter().collect::<Vec<_>>(),
            }))
        }
        "markdown" => {
            // Parse Obsidian-style frontmatter blocks
            let mut count = 0usize;
            let namespaces: std::collections::HashSet<String> = std::collections::HashSet::new();
            let tags: std::collections::HashSet<String> = std::collections::HashSet::new();
            for block in data.split("<<< FILE:") {
                let block = block.trim();
                if block.is_empty() {
                    continue;
                }
                // Find frontmatter between --- delimiters
                if let Some(rest) = block.strip_prefix("---\n")
                    && let Some(_fm_end) = rest.find("\n---\n")
                {
                    count += 1;
                }
            }
            Ok(serde_json::json!({
                "format": "markdown",
                "memories": count,
                "edges": 0,
                "rooms": 0,
                "namespaces": namespaces.into_iter().collect::<Vec<_>>(),
                "tags": tags.into_iter().collect::<Vec<_>>(),
            }))
        }
        _ => Err(CommandError::Uteke(format!(
            "unsupported import format: {}",
            format
        ))),
    }
}

#[tauri::command]
pub async fn import_data(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    format: String,
    data: String,
) -> Result<usize, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };
    let Some(client) = client else {
        return Err(CommandError::NotInitialized);
    };

    match format.as_str() {
        "json" => {
            let parsed: serde_json::Value =
                serde_json::from_str(&data).map_err(|e| CommandError::Uteke(e.to_string()))?;

            let memories = parsed
                .get("memories")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    CommandError::Uteke("invalid import: missing 'memories' array".to_string())
                })?;

            let mut count = 0usize;
            for m in memories {
                let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
                if content.is_empty() {
                    continue;
                }
                let tags: Vec<String> = m
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let namespace = m
                    .get("namespace")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                client
                    .remember(content, &tags, namespace.as_deref())
                    .await
                    .map_err(|e| CommandError::Uteke(e.to_string()))?;
                count += 1;
            }

            // Import edges after memories
            if let Some(edges) = parsed.get("edges").and_then(|v| v.as_array()) {
                for e in edges {
                    let source = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
                    let target = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    if source.is_empty() || target.is_empty() {
                        continue;
                    }
                    let relation = e.get("type").and_then(|v| v.as_str());
                    let weight = e.get("weight").and_then(|v| v.as_f64()).map(|w| w as f32);
                    let _ = client.add_edge(source, target, relation, weight).await;
                }
            }

            // Import rooms after memories
            if let Some(rooms) = parsed.get("rooms").and_then(|v| v.as_array()) {
                for r in rooms {
                    let id = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let title = r.get("title").and_then(|v| v.as_str());
                    let ns = r
                        .get("namespace")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty());
                    if id.is_empty() {
                        continue;
                    }
                    let _ = client.room_create(id, title, ns).await;
                }
            }

            Ok(count)
        }
        "markdown" => {
            // Parse Obsidian-style markdown with YAML frontmatter
            let mut count = 0usize;
            for block in data.split("<<< FILE:") {
                let block = block.trim();
                if block.is_empty() {
                    continue;
                }
                // Split at frontmatter end
                if let Some(rest) = block.strip_prefix("---\n")
                    && let Some(idx) = rest.find("\n---\n")
                {
                    let fm = &rest[..idx];
                    let body = rest[idx + 5..].trim();

                    if body.is_empty() {
                        continue;
                    }

                    // Parse frontmatter fields
                    let mut tags = Vec::new();
                    let mut namespace: Option<String> = None;
                    for line in fm.lines() {
                        if let Some(rest) = line.strip_prefix("tags:") {
                            // Parse [tag1, "tag2"] or tag1, tag2
                            let cleaned = rest.trim();
                            if cleaned.starts_with('[') && cleaned.ends_with(']') {
                                let inner = &cleaned[1..cleaned.len() - 1];
                                for tag in inner.split(',') {
                                    let t = tag.trim().trim_matches('"').trim();
                                    if !t.is_empty() {
                                        tags.push(t.to_string());
                                    }
                                }
                            } else {
                                for tag in cleaned.split(',') {
                                    let t = tag.trim().trim_matches('"').trim();
                                    if !t.is_empty() {
                                        tags.push(t.to_string());
                                    }
                                }
                            }
                        } else if let Some(rest) = line.strip_prefix("namespace:") {
                            let ns = rest.trim().trim_matches('"');
                            if !ns.is_empty() {
                                namespace = Some(ns.to_string());
                            }
                        }
                    }

                    client
                        .remember(body, &tags, namespace.as_deref())
                        .await
                        .map_err(|e| CommandError::Uteke(e.to_string()))?;
                    count += 1;
                }
            }
            Ok(count)
        }
        _ => Err(CommandError::Uteke(format!(
            "unsupported import format: {}",
            format
        ))),
    }
}

#[tauri::command]
pub async fn init_data_dir(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<String, CommandError> {
    let mut s = state.lock().await;

    // If already initialized, return existing path
    if let Some(ref dir) = s.data_dir {
        return Ok(dir.to_string_lossy().to_string());
    }

    // Auto-initialize ~/.codecora/ environment for settings DB
    let (db_path, _config) =
        crate::config::init_environment().map_err(|e| CommandError::Io(e.to_string()))?;

    // Open CorIn DB for settings only (not memories)
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| CommandError::Uteke(e.to_string()))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT
        );",
    )
    .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let corin_dir = crate::config::corin_dir().map_err(|e| CommandError::Io(e.to_string()))?;

    s.data_dir = Some(corin_dir.clone());
    s.db_path = Some(db_path);
    s.conn = Some(conn);

    Ok(corin_dir.to_string_lossy().to_string())
}

// ─── Uteke Server Integration (HTTP API) ───────────────────────────────

/// Check if uteke-serve is running and accessible.
/// Returns server health status + stats if available.
#[tauri::command]
pub async fn uteke_server_status(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Ok(serde_json::json!({
            "available": false,
            "hint": "Uteke client not initialized",
        }));
    };

    if !client.is_available().await {
        return Ok(serde_json::json!({
            "available": false,
            "hint": "Run 'uteke-serve' to enable semantic search and auto-linking",
        }));
    }

    let stats = client
        .stats()
        .await
        .unwrap_or(crate::uteke_client::UtekeStats {
            total_memories: 0,
            unique_tags: 0,
            db_size_bytes: 0,
            hot: 0,
            warm: 0,
            cold: 0,
        });

    Ok(serde_json::json!({
        "available": true,
        "url": crate::config::detect_uteke_serve_url(),
        "stats": stats,
    }))
}

/// Semantic recall via uteke-serve (vector + FTS5 hybrid search).
/// Falls back to uteke_search (FTS5 only) if server is not running.
#[tauri::command]
pub async fn uteke_recall(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, CommandError> {
    let limit = limit.unwrap_or(20);
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Ok(vec![]);
    };

    if !client.is_available().await {
        return Ok(vec![]);
    }

    let results = client
        .recall(&query, namespace.as_deref(), limit)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(results
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.memory.id,
                "content": r.memory.content,
                "tags": r.memory.tags,
                "namespace": r.memory.namespace,
                "importance": r.memory.importance,
                "memory_type": r.memory.memory_type,
                "content_type": r.memory.content_type,
                "created_at": r.memory.created_at,
                "updated_at": r.memory.updated_at,
                "pinned": r.memory.pinned,
                "score": r.score,
            })
        })
        .collect())
}

/// Semantic remember via uteke-serve.
/// Pre-checks for duplicates via recall before inserting.
/// Returns the new ID, or a duplicate warning.
#[tauri::command]
pub async fn uteke_remember(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    content: String,
    tags: Option<Vec<String>>,
    namespace: Option<String>,
) -> Result<serde_json::Value, CommandError> {
    let tags = tags.unwrap_or_default();
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Err(CommandError::Uteke(
            "Uteke server not initialized.".to_string(),
        ));
    };

    if !client.is_available().await {
        return Err(CommandError::Uteke(
            "Uteke server not running. Start it with 'uteke-serve'.".to_string(),
        ));
    }

    // Pre-check: search for duplicates in the target namespace.
    // If score >= 0.92, flag as possible duplicate.
    let dup_check_ns = namespace.as_deref().unwrap_or("default");
    if let Ok(existing) = client.recall(&content, Some(dup_check_ns), 3).await
        && let Some(dup) = existing.iter().find(|r| r.score >= 0.92)
    {
        return Ok(serde_json::json!({
            "duplicate": true,
            "existing_id": dup.memory.id,
            "existing_content": dup.memory.content,
            "score": dup.score,
            "hint": "This memory appears to be a duplicate of an existing one.",
        }));
    }

    // No duplicate found — insert.
    let id = client
        .remember(&content, &tags, namespace.as_deref())
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(serde_json::json!({
        "id": id,
        "duplicate": false,
    }))
}

/// Delete memory via uteke-serve.
#[tauri::command]
pub async fn uteke_forget(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Err(CommandError::Uteke("Uteke client not initialized.".into()));
    };

    if !client.is_available().await {
        return Err(CommandError::Uteke("Uteke server not running.".into()));
    }

    client
        .forget(&id)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))
}

/// Get graph data from uteke-serve (real cosine edges, not tag-based).
///
/// If the server returns 0 edges (cosine auto-link not yet generated),
/// falls back to a tag-based graph: memories sharing at least one tag
/// are connected. This ensures the graph view is never empty when the
/// server has memories but no cosine edges yet.
#[tauri::command]
pub async fn uteke_server_graph(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    namespaces: Option<Vec<String>>,
) -> Result<serde_json::Value, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Ok(serde_json::json!({
            "nodes": [],
            "edges": [],
            "stats": {"node_count": 0, "edge_count": 0, "relation_types": []},
            "hint": "Uteke client not initialized",
        }));
    };

    if !client.is_available().await {
        return Ok(serde_json::json!({
            "nodes": [],
            "edges": [],
            "stats": {"node_count": 0, "edge_count": 0, "relation_types": []},
            "hint": "Start uteke-serve for cosine auto-linked graph",
        }));
    }

    // Multi-namespace mode: user selected specific namespaces in the filter.
    // An explicit empty list means "none" → empty graph. A non-empty list
    // fans out /list per namespace, merges, and builds a tag-based graph.
    if let Some(ns_list) = namespaces {
        return build_multi_namespace_graph(&client, &ns_list).await;
    }

    // "All" mode (no namespace scope): legacy /list and /graph with no
    // namespace only return the "default" namespace, so the graph would
    // miss every other namespace. Fan out across ALL namespaces instead.
    // Try the server's cosine /graph first (real edges if available).
    if namespace.is_none() {
        let graph = client
            .graph(None)
            .await
            .map_err(|e| CommandError::Uteke(e.to_string()))?;
        if !graph.edges.is_empty() {
            return Ok(serde_json::json!({
                "nodes": graph.nodes,
                "edges": graph.edges,
                "stats": graph.stats,
            }));
        }
        let all_ns = client.namespaces().await.unwrap_or_default();
        return build_multi_namespace_graph(&client, &all_ns).await;
    }

    // Single namespace scoped (legacy path).
    let graph = client
        .graph(namespace.as_deref())
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // If server already has cosine edges, return them as-is.
    if !graph.edges.is_empty() {
        return Ok(serde_json::json!({
            "nodes": graph.nodes,
            "edges": graph.edges,
            "stats": graph.stats,
        }));
    }

    // Fallback: build a tag-based graph from memories fetched via /list.
    // Cosine auto-linking may not be active on the installed uteke version,
    // so we generate edges from shared tags to keep the graph useful.
    let memories = client
        .list(namespace.as_deref(), None, 150, 0)
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    if memories.is_empty() {
        return Ok(serde_json::json!({
            "nodes": [],
            "edges": [],
            "stats": {"node_count": 0, "edge_count": 0, "relation_types": []},
            "hint": "No memories yet — create some to see the graph",
        }));
    }

    let (nodes, edges, relation_types) = build_tag_graph(&memories);
    let node_count = nodes.len();
    let edge_count = edges.len();

    Ok(serde_json::json!({
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "node_count": node_count,
            "edge_count": edge_count,
            "relation_types": relation_types,
        },
        "hint": "Tag-based fallback graph (cosine auto-link not active on uteke v0.3.2)",
    }))
}

/// Build a tag-based graph (nodes + edges) from a slice of memories.
/// Edges connect memories that share a tag (capped per tag to avoid clutter).
///
/// Returns `(nodes, edges, relation_types)` as JSON values ready to embed
/// in the command response.
fn build_tag_graph(
    memories: &[crate::uteke_client::UtekeMemory],
) -> (
    Vec<serde_json::Value>,
    Vec<serde_json::Value>,
    Vec<&'static str>,
) {
    let nodes: Vec<serde_json::Value> = memories
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "label": m.content.chars().take(60).collect::<String>(),
                "entity_type": m.tags.first().cloned(),
            })
        })
        .collect();

    let mut tag_to_memories: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for m in memories {
        for tag in &m.tags {
            tag_to_memories.entry(tag.as_str()).or_default().push(&m.id);
        }
    }

    let mut edges: Vec<serde_json::Value> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for ids in tag_to_memories.values() {
        let max_pairs = 5;
        let mut count = 0;
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                if count >= max_pairs {
                    break;
                }
                let key = (ids[i].to_string(), ids[j].to_string());
                if seen.insert(key.clone()) {
                    edges.push(serde_json::json!({
                        "source": ids[i],
                        "target": ids[j],
                        "relation": "shared_tag",
                        "weight": 0.5,
                    }));
                    count += 1;
                }
            }
            if count >= max_pairs {
                break;
            }
        }
    }

    let relation_types: Vec<&'static str> = if edges.is_empty() {
        vec![]
    } else {
        vec!["shared_tag"]
    };

    (nodes, edges, relation_types)
}

/// Multi-namespace graph: fetch memories from each selected namespace via
/// `/list`, merge (dedup by id), and build a tag-based graph.
///
/// This is the path used by the namespace filter UI. It works on legacy
/// servers (per-namespace `/list` is always available) and on newer ones.
async fn build_multi_namespace_graph(
    client: &crate::uteke_client::UtekeClient,
    namespaces: &[String],
) -> Result<serde_json::Value, CommandError> {
    // Cap memories per namespace to keep the graph performant on large
    // servers (e.g. 19k memories on the VPS).
    const PER_NAMESPACE_LIMIT: usize = 60;

    let mut memories: Vec<crate::uteke_client::UtekeMemory> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for ns in namespaces {
        match client
            .list(Some(ns.as_str()), None, PER_NAMESPACE_LIMIT, 0)
            .await
        {
            Ok(list) => {
                for m in list {
                    if seen_ids.insert(m.id.clone()) {
                        memories.push(m);
                    }
                }
            }
            // A namespace may be inaccessible (read-only token, missing, etc).
            // Skip it but keep the other namespaces.
            Err(e) => {
                eprintln!("CorIn: graph: skipping namespace '{ns}': {e}");
            }
        }
    }

    if memories.is_empty() {
        return Ok(serde_json::json!({
            "nodes": [],
            "edges": [],
            "stats": {"node_count": 0, "edge_count": 0, "relation_types": []},
            "hint": "No memories in the selected namespace(s)",
        }));
    }

    let (nodes, edges, relation_types) = build_tag_graph(&memories);
    let node_count = nodes.len();
    let edge_count = edges.len();

    Ok(serde_json::json!({
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "node_count": node_count,
            "edge_count": edge_count,
            "relation_types": relation_types,
        },
        "hint": format!(
            "Tag-based graph across {} namespace(s)",
            namespaces.len()
        ),
    }))
}

/// Get server stats via HTTP.
#[tauri::command]
pub async fn uteke_server_stats(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    let Some(client) = client else {
        return Ok(serde_json::json!({
            "available": false,
            "hint": "Uteke client not initialized",
        }));
    };

    if !client.is_available().await {
        return Ok(serde_json::json!({
            "available": false,
            "hint": "Start uteke-serve",
        }));
    }

    let stats = client
        .stats()
        .await
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(serde_json::json!({
        "total_memories": stats.total_memories,
        "unique_tags": stats.unique_tags,
        "db_size_bytes": stats.db_size_bytes,
        "hot": stats.hot,
        "warm": stats.warm,
        "cold": stats.cold,
    }))
}

// ─── AI Agent Integration (#55) ───────────────────────────────────────

/// Detected AI agent installation.
#[derive(Debug, Clone, Serialize)]
pub struct DetectedAgent {
    pub name: String,
    pub config_path: String,
    pub found: bool,
}

/// Detect installed AI coding agents on this machine.
#[tauri::command]
pub async fn detect_agents() -> Result<Vec<DetectedAgent>, CommandError> {
    let home = dirs::home_dir().ok_or_else(|| CommandError::Io("no home dir".into()))?;

    let agents = vec![
        ("Claude Code", home.join(".claude")),
        ("Pi Dev", home.join(".pi")),
        ("OpenCode", home.join(".opencode")),
        ("Cursor", home.join(".cursor")),
    ];

    let detected: Vec<DetectedAgent> = agents
        .into_iter()
        .map(|(name, path)| {
            let found = path.is_dir();
            DetectedAgent {
                name: name.to_string(),
                config_path: path.to_string_lossy().to_string(),
                found,
            }
        })
        .collect();

    Ok(detected)
}

/// Generate .agent.md for the current project.
#[tauri::command]
pub async fn generate_agent_md(project_dir: Option<String>) -> Result<String, CommandError> {
    let dir = project_dir
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| CommandError::Io("no working dir".into()))?;

    let agent_md = r#"# Agent Instructions

## Memory Protocol
- **Before starting any task**: call `uteke_recall` with a query about the project context, conventions, and past decisions.
- **After learning something new**: call `uteke_remember` with appropriate tags and namespace.
- Use namespaces to isolate: "corin", "uteke", "trapfall", etc.

## Stack
- Desktop shell: Tauri 2
- Frontend: Svelte 5 (runes: `$state`, `$derived`, `$effect`, `$props`)
- Backend: Rust (edition 2024)
- Storage: uteke-core (native embedding + cosine auto-link + graph)
- Graph: Canvas 2D force-directed

## Key Paths
- Rust commands: `src-tauri/src/commands.rs`
- Frontend components: `src/lib/components/`
- IPC layer: `src/lib/ts/ipc.ts`
"#;

    let path = dir.join(".agent.md");
    std::fs::write(&path, agent_md).map_err(|e| CommandError::Io(e.to_string()))?;

    Ok(path.to_string_lossy().to_string())
}

/// Run uteke dream cycle (maintenance pipeline).
///
/// Prefers HTTP POST /dream on uteke-serve when available.
/// Falls back to CLI `uteke dream --json --quiet` when server is not running
/// (e.g. remote connection without a running server, or disconnected mode).
#[tauri::command]
pub async fn run_dream_cycle(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    dry_run: Option<bool>,
) -> Result<serde_json::Value, CommandError> {
    let client = {
        let s = state.lock().await;
        s.uteke_client.clone()
    };

    // ── Path A: HTTP (preferred) ──
    if let Some(ref client) = client
        && client.is_available().await
    {
        let report = client
            .dream(namespace.as_deref(), dry_run.unwrap_or(false))
            .await
            .map_err(CommandError::Uteke)?;

        // Persist run to history.
        let s = state.lock().await;
        if let Some(conn) = s.conn.as_ref() {
            let _ = save_dream_run(conn, &report);
        }

        return Ok(serde_json::json!({
            "success": report.total_errors == 0,
            "phases": report.phases,
            "total_changes": report.total_changes,
            "total_warnings": report.total_warnings,
            "total_errors": report.total_errors,
            "dry_run": report.dry_run,
            "duration_ms": report.duration_ms,
        }));
    }

    // ── Path B: CLI fallback ──
    let uteke_bin = find_in_path("uteke")
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/bin/uteke")))
        .filter(|p| p.is_file());

    let Some(bin) = uteke_bin else {
        return Ok(serde_json::json!({
            "success": false,
            "hint": "uteke binary not found and server unavailable. Install: curl -fsSL https://raw.githubusercontent.com/codecoradev/uteke/main/install.sh | sh"
        }));
    };

    let mut cmd = std::process::Command::new(&bin);
    cmd.arg("dream").arg("--json").arg("--quiet");
    if dry_run == Some(true) {
        cmd.arg("--dry-run");
    }
    if let Some(ref ns) = namespace {
        cmd.arg("--namespace").arg(ns);
    }

    let output = cmd.output().map_err(|e| CommandError::Io(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let result = serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|_| {
        serde_json::json!({
            "raw_stdout": stdout,
            "raw_stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        })
    });

    // Merge CLI output into flat response matching HTTP path shape.
    let mut resp = serde_json::json!({
        "success": output.status.success(),
        "phases": [],
        "total_changes": 0,
        "total_warnings": 0,
        "total_errors": 0,
        "dry_run": dry_run.unwrap_or(false),
        "duration_ms": 0,
    });
    if let Some(phases) = result.get("phases").and_then(|v| v.as_array()) {
        resp["phases"] = serde_json::json!(phases);
    }
    if let Some(c) = result.get("total_changes").and_then(|v| v.as_u64()) {
        resp["total_changes"] = serde_json::json!(c);
    }
    if let Some(w) = result.get("total_warnings").and_then(|v| v.as_u64()) {
        resp["total_warnings"] = serde_json::json!(w);
    }
    if let Some(e) = result.get("total_errors").and_then(|v| v.as_u64()) {
        resp["total_errors"] = serde_json::json!(e);
    }
    if let Some(d) = result.get("duration_ms").and_then(|v| v.as_u64()) {
        resp["duration_ms"] = serde_json::json!(d);
    }
    if !output.status.success() {
        resp["hint"] = serde_json::json!(format!(
            "uteke dream CLI exited with code {}. Check server connectivity.",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(resp)
}

/// Save a dream cycle result to the run history table.
fn save_dream_run(
    conn: &rusqlite::Connection,
    report: &crate::uteke_client::DreamReport,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let phases_json = serde_json::to_string(&report.phases).unwrap_or_default();
    conn.execute(
        "INSERT INTO dream_run_history (ran_at, success, total_changes, total_warnings, total_errors, duration_ms, phases_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            now,
            report.total_errors == 0,
            report.total_changes as i64,
            report.total_warnings as i64,
            report.total_errors as i64,
            report.duration_ms as i64,
            phases_json,
        ],
    )?;
    // Keep only the last 50 runs.
    conn.execute(
        "DELETE FROM dream_run_history WHERE id NOT IN (
             SELECT id FROM dream_run_history ORDER BY ran_at DESC LIMIT 50
         )",
        [],
    )?;
    Ok(())
}

/// Get dream cycle run history (last N runs).
#[tauri::command]
pub async fn get_dream_history(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, CommandError> {
    let limit = limit.unwrap_or(10) as i64;
    let s = state.lock().await;
    let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, ran_at, success, total_changes, total_warnings, total_errors, duration_ms, phases_json
             FROM dream_run_history
             ORDER BY ran_at DESC
             LIMIT ?1",
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let rows = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "ran_at": row.get::<_, String>(1)?,
                "success": row.get::<_, bool>(2)?,
                "total_changes": row.get::<_, i64>(3)?,
                "total_warnings": row.get::<_, i64>(4)?,
                "total_errors": row.get::<_, i64>(5)?,
                "duration_ms": row.get::<_, i64>(6)?,
                "phases": serde_json::from_str::<Vec<crate::uteke_client::PhaseResult>>(
                    &row.get::<_, String>(7).unwrap_or_default()
                ).unwrap_or_default(),
            }))
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    Ok(rows)
}

// ---------------------------------------------------------------------------
// Connection commands (Phase 4)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_connections(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<crate::connections::ConnectionInfo>, CommandError> {
    let s = state.lock().await;
    let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;
    crate::connections::store::list(conn).map_err(CommandError::Uteke)
}

#[tauri::command]
pub async fn add_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    name: String,
    product_type: String,
    url: String,
    auth_token: Option<String>,
    auth_type: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<String, CommandError> {
    let pt = match product_type.as_str() {
        "uteke" => crate::connections::ProductType::Uteke,
        other => {
            return Err(CommandError::Uteke(format!(
                "unknown product type: {other}"
            )));
        }
    };
    let id = nanoid::nanoid!(12);
    let mut s = state.lock().await;
    let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
    crate::connections::store::insert(
        conn,
        &id,
        &name,
        pt,
        &url,
        auth_type.as_deref(),
        auth_token.as_deref(),
        metadata.as_ref(),
    )
    .map_err(CommandError::Uteke)?;
    Ok(id)
}

#[tauri::command]
pub async fn update_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
    name: Option<String>,
    url: Option<String>,
    auth_token: Option<String>,
    auth_type: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
    crate::connections::store::update(
        conn,
        &id,
        name.as_deref(),
        url.as_deref(),
        auth_type.as_deref(),
        auth_token.as_deref(),
        metadata.as_ref(),
    )
    .map_err(CommandError::Uteke)
}

#[tauri::command]
pub async fn delete_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
    // Security: clear (overwrite) the auth token before deleting the row,
    // so it cannot be recovered from freed sqlite pages.
    crate::connections::store::clear_token(conn, &id).map_err(CommandError::Uteke)?;
    crate::connections::store::delete(conn, &id).map_err(CommandError::Uteke)
}

#[tauri::command]
pub async fn test_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<crate::connections::HealthInfo, CommandError> {
    let s = state.lock().await;
    let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;
    let row = crate::connections::store::get(conn, &id).map_err(CommandError::Uteke)?;
    let cfg = crate::connections::ConnectionConfig::from(&row);

    use crate::connections::traits::ProductAdapter;
    let adapter = crate::connections::adapters::uteke::UtekeAdapter::new(&cfg);
    let health = adapter
        .health_check(cfg.clone())
        .await
        .map_err(CommandError::Uteke)?;

    // Update status in DB.
    let status = if health.success { "connected" } else { "error" };
    drop(s); // release lock before mutable DB write
    let mut s = state.lock().await;
    let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
    let _ = crate::connections::store::update_status(conn, &id, status, &health);

    Ok(health)
}

#[tauri::command]
pub async fn set_primary_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    {
        let mut s = state.lock().await;
        let conn = s.conn.as_mut().ok_or(CommandError::NotInitialized)?;
        crate::connections::store::set_primary(conn, &id).map_err(CommandError::Uteke)?;
    }
    // Rebuild the live client so the new primary takes effect immediately
    // (no app restart required).
    rebuild_active_client(&state, &id).await
}

/// Reconnect to a connection at runtime — rebuilds the live memory backend
/// from the connection's config without restarting the app.
#[tauri::command]
pub async fn reconnect_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<crate::connections::HealthInfo, CommandError> {
    rebuild_active_client(&state, &id).await?;
    // Run a health check so the caller gets immediate feedback.
    let cfg = {
        let s = state.lock().await;
        let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;
        let row = crate::connections::store::get(conn, &id).map_err(CommandError::Uteke)?;
        crate::connections::ConnectionConfig::from(&row)
    };
    use crate::connections::traits::ProductAdapter;
    let adapter = crate::connections::adapters::uteke::UtekeAdapter::new(&cfg);
    let health = adapter
        .health_check(cfg)
        .await
        .map_err(CommandError::Uteke)?;
    let status = if health.success { "connected" } else { "error" };
    {
        let mut s = state.lock().await;
        if let Some(conn) = s.conn.as_mut() {
            let _ = crate::connections::store::update_status(conn, &id, status, &health);
        }
    }
    Ok(health)
}

/// Rebuild the live `uteke_client` from a connection's config.
///
/// Reads the connection row by id, resolves local-vs-remote, and swaps
/// `AppState.uteke_client` in place — no app restart required.
/// For local URLs, ensures the uteke-serve process is running first.
async fn rebuild_active_client(
    state: &tauri::State<'_, Arc<Mutex<AppState>>>,
    id: &str,
) -> Result<(), CommandError> {
    // 1. Read the connection row (hold lock briefly, then release).
    let row = {
        let s = state.lock().await;
        let conn = s.conn.as_ref().ok_or(CommandError::NotInitialized)?;
        crate::connections::store::get(conn, id).map_err(CommandError::Uteke)?
    };

    // 2. For local URLs, ensure server is running (may install/spawn).
    //    Remote URLs skip auto-start.
    let url = if crate::config::is_remote_url(&row.url) {
        row.url.clone()
    } else {
        crate::ensure_uteke_server()
    };

    // 3. Build the new client and swap it in.
    let client = crate::uteke_client::UtekeClient::with_auth(&url, row.auth_token.clone());
    let masked = mask_token_log(row.auth_token.as_deref());
    {
        let mut s = state.lock().await;
        s.uteke_client = Some(client);
    }
    eprintln!(
        "CorIn: reconnected active client to '{}' ({}) token={}",
        row.name, url, masked
    );
    Ok(())
}

/// Disconnect the active memory backend.
///
/// Drops `AppState.uteke_client` (sets it to `None`) and marks the primary
/// connection as `disconnected`. Recall/search will fail until a reconnect.
/// The connection row and primary flag are preserved.
#[tauri::command]
pub async fn disconnect_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    s.uteke_client = None;
    if let Some(conn) = s.conn.as_mut()
        && let Ok(Some(primary)) =
            crate::connections::store::get_primary(conn, crate::connections::ProductType::Uteke)
    {
        let _ = crate::connections::store::set_status(conn, &primary.id, "disconnected");
    }
    eprintln!("CorIn: disconnected active memory backend");
    Ok(())
}

/// Mask an auth token for safe logging.
/// Returns `"none"` when absent, or `"<redacted>"` with a short prefix.
fn mask_token_log(token: Option<&str>) -> String {
    match token {
        None => "none".to_string(),
        Some(t) if t.len() <= 6 => "<redacted>".to_string(),
        Some(t) => format!("<redacted:{}…>", &t[..6]),
    }
}

/// Find a binary in PATH.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
