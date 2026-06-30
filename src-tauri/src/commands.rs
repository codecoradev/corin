//! IPC command layer — thin Tauri command wrappers over uteke-core.
//!
//! Every command is an `async fn` decorated with `#[tauri::command]`.
//! State is held in [`AppState`] behind `tokio::sync::Mutex`.

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
/// Uses uteke-core as the primary store (native embedding, graph,
/// cosine auto-linking). CorIn's own SQLite DB is kept only for
/// app-level settings (not memories).
#[derive(Default)]
pub struct AppState {
    /// Path to the CorIn SQLite database (~/.codecora/corin/corin.db).
    pub db_path: Option<PathBuf>,
    /// CorIn SQLite connection for settings only.
    pub conn: Option<rusqlite::Connection>,
    pub data_dir: Option<PathBuf>,
    /// Uteke-core native store (embedding + graph + cosine auto-link).
    pub uteke: Option<uteke_core::Uteke>,
    /// HTTP client for uteke-serve (semantic search, cosine auto-link).
    /// None if server is not running.
    pub uteke_client: Option<crate::uteke_client::UtekeClient>,
}

impl AppState {
    /// Check if the uteke-core native store is available.
    fn ensure_uteke(&self) -> Result<&uteke_core::Uteke, CommandError> {
        self.uteke.as_ref().ok_or(CommandError::NotInitialized)
    }
}

// ---------------------------------------------------------------------------
// Commands: Memory CRUD
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn remember(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    content: String,
    tags: Vec<String>,
    namespace: Option<String>,
    _content_type: Option<String>,
    _importance: Option<f32>,
) -> Result<String, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let tag_refs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
    let ns = namespace.as_deref();

    let id = uteke
        .remember(&content, &tag_refs, None, ns)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(id)
}

// NOTE: Auto-edge generation deferred to Phase 2 when uteke-core library
// is integrated. Phase 1 edges are managed via uteke CLI import or
// the graph_edges table directly.

#[tauri::command]
pub async fn recall(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    let s = state.lock().await;
    let limit = limit.unwrap_or(10);

    // ── API-first: try uteke-serve ──
    if let Some(client) = s.uteke_client.clone()
        && client.is_available().await
        && let Ok(results) = client.recall(&query, namespace.as_deref(), limit).await
    {
        return Ok(results
            .into_iter()
            .map(|r| SearchResult {
                id: r.memory.id,
                content: r.memory.content,
                score: r.score,
                tags: r.memory.tags,
            })
            .collect());
    }

    // ── Fallback: uteke-core native recall ──
    let uteke = s.ensure_uteke()?;
    let results = uteke
        .recall(&query, limit, None, namespace.as_deref(), 0.0)
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
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    // ── API-first: try uteke-serve ──
    {
        let client = {
            let s = state.lock().await;
            s.uteke_client.clone()
        };
        if let Some(client) = client
            && client.is_available().await
            && let Ok(memories) = client
                .list(namespace.as_deref(), tag.as_deref(), limit, offset)
                .await
        {
            return Ok(memories
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
                .collect());
        }
    }

    // ── Fallback: uteke-core native list ──
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;
    let memories = uteke
        .store()
        .list(tag.as_deref(), namespace.as_deref(), limit, offset)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(memories
        .into_iter()
        .map(|m| MemoryEntry {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance as f32),
            namespace: Some(m.namespace.clone()),
            created_at: Some(m.created_at.to_rfc3339()),
            updated_at: Some(m.updated_at.to_rfc3339()),
        })
        .collect())
}

#[tauri::command]
pub async fn forget(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;
    uteke
        .store()
        .delete(&id)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn get_memory(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<MemoryEntry, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;
    let m = uteke
        .store()
        .get_by_id(&id)
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .ok_or(CommandError::NotFound(id))?;

    Ok(MemoryEntry {
        id: m.id,
        content: m.content,
        tags: m.tags,
        content_type: Some(m.content_type),
        importance: Some(m.importance as f32),
        namespace: Some(m.namespace),
        created_at: Some(m.created_at.to_rfc3339()),
        updated_at: Some(m.updated_at.to_rfc3339()),
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
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;
    let limit = limit.unwrap_or(100);

    let gd = uteke
        .graph_data(namespace.as_deref())
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Convert uteke-core GraphData → CorIn GraphData
    let nodes: Vec<MemoryEntry> = gd
        .nodes
        .into_iter()
        .take(limit)
        .filter_map(|n| {
            // GraphNode has memory_id; fetch memory to get content
            let mid = n.memory_id.as_deref()?;
            let m = uteke.get_by_id(mid).ok()??;
            Some(MemoryEntry {
                id: m.id,
                content: m.content,
                tags: m.tags,
                content_type: Some(m.content_type),
                importance: Some(m.importance as f32),
                namespace: Some(m.namespace),
                created_at: Some(m.created_at.to_rfc3339()),
                updated_at: Some(m.updated_at.to_rfc3339()),
            })
        })
        .collect();

    let node_id_set: std::collections::HashSet<String> =
        nodes.iter().map(|n| n.id.clone()).collect();

    let edges: Vec<GraphEdge> = gd
        .edges
        .into_iter()
        .filter(|e| node_id_set.contains(&e.source_id) && node_id_set.contains(&e.target_id))
        .map(|e| GraphEdge {
            id: None,
            source: e.source_id,
            target: e.target_id,
            weight: Some(e.weight as f32),
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
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    // Use graph_store to find neighbors via edges
    let gd = uteke
        .graph_data(None)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let neighbor_ids: std::collections::HashSet<String> = gd
        .edges
        .iter()
        .filter(|e| e.source_id == id || e.target_id == id)
        .map(|e| {
            if e.source_id == id {
                e.target_id.clone()
            } else {
                e.source_id.clone()
            }
        })
        .collect();

    let mut entries = Vec::new();
    for nid in neighbor_ids {
        if let Ok(Some(m)) = uteke.get_by_id(&nid) {
            entries.push(MemoryEntry {
                id: m.id,
                content: m.content,
                tags: m.tags,
                content_type: Some(m.content_type),
                importance: Some(m.importance as f32),
                namespace: Some(m.namespace),
                created_at: Some(m.created_at.to_rfc3339()),
                updated_at: Some(m.updated_at.to_rfc3339()),
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

    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    // Verify both nodes exist
    if uteke
        .get_by_id(&source)
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .is_none()
    {
        return Err(CommandError::NotFound(source));
    }
    if uteke
        .get_by_id(&target)
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .is_none()
    {
        return Err(CommandError::NotFound(target));
    }

    let rel = edge_type.as_deref().unwrap_or("related");
    let w = weight.unwrap_or(1.0) as f64;

    let gs = uteke_core::graph::GraphStore::new(uteke.graph_store());
    gs.add_edge(&source, &target, rel, w)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(0) // uteke-core edges use string IDs, not auto-increment i64
}

#[tauri::command]
pub async fn remove_edge(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    _id: i64,
) -> Result<(), CommandError> {
    // Edge removal by numeric ID is no longer supported with uteke-core
    // (edges use string-based source/target keys). Frontend should
    // use source+target to identify edges for deletion.
    // This command is kept for backward compatibility but is a no-op.
    let _ = state.lock().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands: Room
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_rooms(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<RoomEntry>, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let rooms = uteke
        .store()
        .list_rooms(None)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let entries: Vec<RoomEntry> = rooms
        .into_iter()
        .map(|r| {
            let stats = uteke.store().room_stats(&r.id).unwrap_or(None);
            RoomEntry {
                id: r.id.clone(),
                name: r.title.unwrap_or_else(|| r.id.clone()),
                participant_count: stats.as_ref().map(|s| s.participant_count).unwrap_or(0),
                memory_count: stats.as_ref().map(|s| s.memory_count).unwrap_or(0),
                created_at: Some(r.created_at.clone()),
            }
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn get_room_summary(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let memories = uteke
        .store()
        .recall_room(&room_id, None, 100)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let summary = memories
        .iter()
        .map(|m| format!("- {}", m.content.chars().take(80).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
pub async fn create_room(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    name: String,
    _namespace: Option<String>,
    _tags: Option<Vec<String>>,
) -> Result<String, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let room_id = nanoid::nanoid!(12);
    let ns = _namespace.as_deref().unwrap_or("default");
    uteke
        .store()
        .create_room(&room_id, Some(&name), ns)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(room_id)
}

#[tauri::command]
pub async fn get_room_document(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let room = uteke
        .store()
        .get_room(&room_id)
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .ok_or_else(|| CommandError::NotFound(room_id.clone()))?;

    let room_name = room.title.as_deref().unwrap_or("unnamed");
    let memories = uteke
        .store()
        .recall_room(&room_id, None, 100)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let mut doc = format!("# Room: {}\n\n", room_name);
    if memories.is_empty() {
        doc.push_str("_No memories in this room yet._\n");
    } else {
        for m in &memories {
            doc.push_str(&m.content);
            doc.push_str("\n\n---\n\n");
        }
    }

    Ok(doc)
}

/// Delete a room by ID.
#[tauri::command]
pub async fn delete_room(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<(), CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    uteke
        .store()
        .delete_room(&room_id)
        .map_err(|e| CommandError::Uteke(e.to_string()))
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
#[tauri::command]
pub async fn uteke_list(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    namespace: Option<String>,
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
    // ── API-first: try uteke-serve HTTP ──
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

    // ── Fallback: uteke-core native graph (cosine auto-linked edges) ──
    let s = state.lock().await;
    if let Some(uteke) = s.uteke.as_ref() {
        let gd = uteke
            .graph_data(namespace.as_deref())
            .map_err(|e| CommandError::Uteke(e.to_string()))?;

        let nodes: Vec<MemoryEntry> = gd
            .nodes
            .iter()
            .filter_map(|n| {
                let mid = n.memory_id.as_deref()?;
                let m = uteke.get_by_id(mid).ok().flatten()?;
                Some(MemoryEntry {
                    id: m.id,
                    content: m.content,
                    tags: m.tags,
                    content_type: Some(m.content_type),
                    importance: Some(m.importance as f32),
                    namespace: Some(m.namespace),
                    created_at: Some(m.created_at.to_rfc3339()),
                    updated_at: Some(m.updated_at.to_rfc3339()),
                })
            })
            .collect();

        let edges: Vec<GraphEdge> = gd
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| GraphEdge {
                id: Some(i as i64),
                source: e.source_id.clone(),
                target: e.target_id.clone(),
                weight: Some(e.weight as f32),
            })
            .collect();

        return Ok(GraphData { nodes, edges });
    }

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

/// List rooms via HTTP.
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
    Ok(rooms
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "namespace": r.namespace,
                "memory_count": 0,
                "participant_count": 0,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
            })
        })
        .collect())
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
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    // Use native store — the HTTP /room/recall endpoint has a namespace
    // scoping bug and returns 0 results. Native recall_room works correctly.
    let results = uteke
        .store()
        .recall_room(&room_id, None, limit)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(results
        .into_iter()
        .map(|m| MemoryEntry {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance as f32),
            namespace: Some(m.namespace),
            created_at: Some(m.created_at.to_rfc3339()),
            updated_at: Some(m.updated_at.to_rfc3339()),
        })
        .collect())
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
    // ── API-first: try uteke-serve before touching local store ──
    {
        let client = {
            let s = state.lock().await;
            s.uteke_client.clone()
        };
        if let Some(client) = client
            && client.is_available().await
            && let Ok(server_stats) = client.stats().await
        {
            let total_namespaces = client.namespaces().await.unwrap_or_default().len();
            return Ok(StatsResponse {
                total_memories: server_stats.total_memories,
                total_namespaces,
                total_tags: server_stats.unique_tags,
                total_edges: 0,
                db_size_bytes: server_stats.db_size_bytes,
            });
        }
    }

    // ── Fallback: read from uteke-core native store ──
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let total_memories = uteke.count(None).unwrap_or(0);
    let namespaces = uteke.list_namespaces().unwrap_or_default();
    let total_namespaces = namespaces.len();
    let tags = uteke.store().unique_tags(None).unwrap_or_default();

    let graph = uteke.graph_data(None).map(|gd| gd.edges.len()).unwrap_or(0);

    let db_size_bytes = uteke_core::uteke_home()
        .ok()
        .and_then(|p| std::fs::metadata(p.join("uteke.db")).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(StatsResponse {
        total_memories,
        total_namespaces,
        total_tags: tags.len(),
        total_edges: graph,
        db_size_bytes,
    })
}

#[tauri::command]
pub async fn list_namespaces(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;
    let namespaces = uteke
        .store()
        .list_namespaces()
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    Ok(namespaces)
}

#[tauri::command]
pub async fn list_tags(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
) -> Result<HashMap<String, usize>, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let tags = uteke
        .store()
        .tags_with_counts(namespace.as_deref())
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let tag_counts: HashMap<String, usize> = tags.into_iter().map(|t| (t.name, t.count)).collect();

    // Sort by count descending, take top 50
    let mut sorted: Vec<(String, usize)> = tag_counts.into_iter().collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
    sorted.truncate(50);
    let top: HashMap<String, usize> = sorted.into_iter().collect();

    Ok(top)
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
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
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
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
) -> Result<String, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

    let memories = uteke
        .store()
        .list(None, None, 10000, 0)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    match format.as_str() {
        "json" => {
            let export = serde_json::json!({
                "version": "2.0",
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "memories": memories.iter().map(|m| serde_json::json!({
                    "id": m.id,
                    "content": m.content,
                    "tags": m.tags,
                    "namespace": m.namespace,
                    "created_at": m.created_at.to_rfc3339(),
                    "updated_at": m.updated_at.to_rfc3339(),
                })).collect::<Vec<_>>(),
            });
            Ok(serde_json::to_string_pretty(&export).unwrap_or_default())
        }
        "markdown" => {
            let mut doc = String::from("# CorIn Export\n\n");
            for m in &memories {
                doc.push_str(&format!(
                    "## {}\n\n{}\n\n**Tags:** {}\n**Namespace:** {}\n**Created:** {}\n\n---\n\n",
                    m.id,
                    m.content,
                    m.tags.join(", "),
                    m.namespace,
                    m.created_at.to_rfc3339(),
                ));
            }
            Ok(doc)
        }
        _ => Err(CommandError::Uteke(format!(
            "unsupported export format: {} (use 'json' or 'markdown')",
            format
        ))),
    }
}

#[tauri::command]
pub async fn import_data(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    data: String,
) -> Result<usize, CommandError> {
    let s = state.lock().await;
    let uteke = s.ensure_uteke()?;

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
            .map(|s| s.to_string());

        let tag_refs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
        uteke
            .remember(content, &tag_refs, None, namespace.as_deref())
            .map_err(|e| CommandError::Uteke(e.to_string()))?;
        count += 1;
    }

    Ok(count)
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

    // Open uteke-core native store (~/.uteke) with embedding + graph
    let uteke_home = uteke_core::uteke_home().map_err(|e| CommandError::Io(e.to_string()))?;
    let uteke_store =
        uteke_core::Uteke::open(&uteke_home).map_err(|e| CommandError::Uteke(e.to_string()))?;

    let corin_dir = crate::config::corin_dir().map_err(|e| CommandError::Io(e.to_string()))?;

    s.data_dir = Some(corin_dir.clone());
    s.db_path = Some(db_path);
    s.conn = Some(conn);
    s.uteke = Some(uteke_store);

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

    // Build nodes from memories.
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

    // Build edges from shared tags.
    // For each tag, collect all memory IDs that have it, then connect them.
    let mut tag_to_memories: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for m in &memories {
        for tag in &m.tags {
            tag_to_memories.entry(tag.as_str()).or_default().push(&m.id);
        }
    }

    let mut edges: Vec<serde_json::Value> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for ids in tag_to_memories.values() {
        // Connect all pairs within the same tag (cap at 5 per tag to avoid clutter).
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

    let node_count = nodes.len();
    let edge_count = edges.len();
    let relation_types: Vec<&str> = if edges.is_empty() {
        vec![]
    } else {
        vec!["shared_tag"]
    };

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
#[tauri::command]
pub async fn run_dream_cycle(
    _state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, CommandError> {
    // Find uteke binary
    let uteke_bin = find_in_path("uteke")
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/bin/uteke")))
        .filter(|p| p.is_file());

    let Some(bin) = uteke_bin else {
        return Ok(serde_json::json!({
            "success": false,
            "hint": "uteke binary not found. Install: curl -fsSL https://raw.githubusercontent.com/codecoradev/uteke/main/install.sh | sh"
        }));
    };

    let output = std::process::Command::new(&bin)
        .arg("dream")
        .arg("--json")
        .arg("--quiet")
        .output()
        .map_err(|e| CommandError::Io(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Try to parse JSON output
    let result = serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|_| {
        serde_json::json!({
            "raw_stdout": stdout,
            "raw_stderr": stderr,
        })
    });

    Ok(serde_json::json!({
        "success": output.status.success(),
        "result": result,
    }))
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
