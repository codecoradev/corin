//! IPC command layer — thin Tauri command wrappers over uteke-core.
//!
//! Every command is an `async fn` decorated with `#[tauri::command]`.
//! State is held in [`AppState`] behind `tokio::sync::Mutex`.

use std::collections::HashMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Phase 1: SQLite-only backend (no embedding, pure FTS5).
/// Phase 2: uteke-core library dep for full embedding + graph.
#[derive(Default)]
pub struct AppState {
    /// Path to the Hub SQLite database (~/.codecora/hub/hub.db).
    pub db_path: Option<PathBuf>,
    /// Hub SQLite connection for Hub-native operations.
    pub conn: Option<rusqlite::Connection>,
    pub data_dir: Option<PathBuf>,
    /// Path to Uteke database (~/.uteke/uteke.db via symlink).
    pub uteke_db_path: Option<PathBuf>,
    /// Read-only connection to Uteke database (None if not installed).
    pub uteke_conn: Option<rusqlite::Connection>,
}

impl AppState {
    /// Check if store is initialized.
    fn ensure_initialized(&self) -> Result<(), CommandError> {
        if self.conn.is_none() {
            return Err(CommandError::NotInitialized);
        }
        Ok(())
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
    content_type: Option<String>,
    importance: Option<f32>,
) -> Result<String, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let id = nanoid::nanoid!(12);

    let tags_json = serde_json::to_string(&tags).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO memories (id, content, tags, content_type, importance, namespace, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, content, tags_json, content_type, importance, namespace, now, now],
    ).map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(id)
}

// NOTE: Auto-edge generation deferred to Phase 2 when uteke-core library
// is integrated. Phase 1 edges are managed via uteke CLI import or
// the graph_edges table directly.

#[tauri::command]
pub async fn recall(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    query: String,
    _namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let limit = limit.unwrap_or(10);
    let query_lower = format!("%{}%", query.to_lowercase());

    let mut stmt = conn.prepare(
        "SELECT id, content, tags FROM memories WHERE content LIKE ?1 ORDER BY updated_at DESC LIMIT ?2"
    ).map_err(|e| CommandError::Uteke(e.to_string()))?;

    let results = stmt
        .query_map(rusqlite::params![query_lower, limit as i64], |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(SearchResult {
                id: row.get(0)?,
                content: row.get(1)?,
                score: 0.0, // Phase 1: no embedding, score = 0. Phase 2: uteke-core recall
                tags,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
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
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    let mut sql = String::from(
        "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at FROM memories WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref ns) = namespace {
        sql.push_str(" AND namespace = ?");
        params.push(Box::new(ns.clone()));
    }
    if let Some(ref t) = tag {
        sql.push_str(" AND tags LIKE ?");
        params.push(Box::new(format!("%\"{}\"%", t)));
    }

    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
    let limit_i: i64 = limit as i64;
    let offset_i: i64 = offset as i64;
    params.push(Box::new(limit_i));
    params.push(Box::new(offset_i));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let results = stmt
        .query_map(param_refs.as_slice(), |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn forget(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    conn.execute(
        "DELETE FROM graph_edges WHERE source = ?1 OR target = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub async fn get_memory(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<MemoryEntry, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    conn.query_row(
        "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at FROM memories WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|_| CommandError::NotFound(id))
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
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let limit = limit.unwrap_or(100);

    // Get nodes
    let mut sql_nodes = String::from(
        "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at FROM memories",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(ref ns) = namespace {
        sql_nodes.push_str(" WHERE namespace = ?");
        params.push(Box::new(ns.clone()));
    }
    sql_nodes.push_str(&format!(" LIMIT {}", limit));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql_nodes)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let nodes: Vec<MemoryEntry> = stmt
        .query_map(param_refs.as_slice(), |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

    // Get edges between nodes
    let mut edges = Vec::new();
    if node_ids.len() > 1 {
        let placeholders = node_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql_edges = format!(
            "SELECT id, source, target, weight FROM graph_edges WHERE source IN ({0}) AND target IN ({0})",
            placeholders
        );
        let edge_params: Vec<Box<dyn rusqlite::types::ToSql>> = node_ids
            .iter()
            .map(|id| Box::new(id.clone()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let edge_param_refs: Vec<&dyn rusqlite::types::ToSql> =
            edge_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt_e = conn
            .prepare(&sql_edges)
            .map_err(|e| CommandError::Uteke(e.to_string()))?;
        edges = stmt_e
            .query_map(edge_param_refs.as_slice(), |row| {
                Ok(GraphEdge {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    target: row.get(2)?,
                    weight: row.get(3)?,
                })
            })
            .map_err(|e| CommandError::Uteke(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
    }

    Ok(GraphData { nodes, edges })
}

#[tauri::command]
pub async fn get_neighbors(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
    depth: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let depth = depth.unwrap_or(1);
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    // BFS from the given ID
    let mut visited = std::collections::HashSet::new();
    visited.insert(id.clone());
    let mut frontier = vec![id.clone()];
    let mut result_ids = Vec::new();

    for _ in 0..depth {
        let mut next_frontier = Vec::new();
        for node_id in &frontier {
            let mut stmt = conn
                .prepare("SELECT source, target FROM graph_edges WHERE source = ?1 OR target = ?1")
                .map_err(|e| CommandError::Uteke(e.to_string()))?;

            let neighbors: Vec<String> = stmt
                .query_map(rusqlite::params![node_id], |row| {
                    let source: String = row.get(0)?;
                    let target: String = row.get(1)?;
                    Ok(if source == *node_id { target } else { source })
                })
                .map_err(|e| CommandError::Uteke(e.to_string()))?
                .filter_map(|r| r.ok())
                .filter(|n| !visited.contains(n))
                .collect();

            for n in &neighbors {
                visited.insert(n.clone());
                next_frontier.push(n.clone());
                result_ids.push(n.clone());
            }
        }
        frontier = next_frontier;
    }

    // Fetch the memory entries
    let mut entries = Vec::new();
    for id in result_ids {
        if let Ok(entry) = conn.query_row(
            "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at FROM memories WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let tags_str: String = row.get(2)?;
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    tags,
                    content_type: row.get(3)?,
                    importance: row.get(4)?,
                    namespace: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        ) {
            entries.push(entry);
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

    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    // Verify both nodes exist
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM memories WHERE id = ?1)",
            rusqlite::params![source],
            |row| row.get(0),
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    if !exists {
        return Err(CommandError::NotFound(source));
    }
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM memories WHERE id = ?1)",
            rusqlite::params![target],
            |row| row.get(0),
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    if !exists {
        return Err(CommandError::NotFound(target));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let edge_type = edge_type.unwrap_or_else(|| "related".to_string());
    let weight = weight.unwrap_or(1.0);

    conn.execute(
        "INSERT INTO graph_edges (source, target, edge_type, weight, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![source, target, edge_type, weight, now],
    )
    .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub async fn remove_edge(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: i64,
) -> Result<(), CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let affected = conn
        .execute(
            "DELETE FROM graph_edges WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    if affected == 0 {
        return Err(CommandError::NotFound(id.to_string()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands: Room
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_rooms(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<RoomEntry>, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    // Phase 1: Rooms are memories with room: tag prefix
    let mut stmt = conn.prepare(
        "SELECT id, content, tags, namespace, created_at FROM memories WHERE tags LIKE '%\"room:%' ORDER BY created_at DESC"
    ).map_err(|e| CommandError::Uteke(e.to_string()))?;

    let rooms: Vec<RoomEntry> = stmt
        .query_map([], |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let room_name = tags
                .iter()
                .find(|t| t.starts_with("room:"))
                .map(|t| t.replace("room:", ""))
                .unwrap_or("unnamed".to_string());
            Ok(RoomEntry {
                id: row.get(0)?,
                name: room_name,
                participant_count: 1,
                memory_count: 1,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rooms)
}

#[tauri::command]
pub async fn get_room_summary(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let content: String = conn
        .query_row(
            "SELECT content FROM memories WHERE id = ?1",
            rusqlite::params![room_id],
            |row| row.get(0),
        )
        .map_err(|_| CommandError::NotFound(room_id))?;

    Ok(content)
}

#[tauri::command]
pub async fn create_room(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    name: String,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<String, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    // Rooms are memories with room:<name> tag
    let mut room_tags = tags.unwrap_or_default();
    room_tags.push(format!("room:{}", name));

    let id = nanoid::nanoid!(12);
    let tags_json = serde_json::to_string(&room_tags).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let content = format!("# Room: {}\n\n", name);

    conn.execute(
        "INSERT INTO memories (id, content, tags, content_type, importance, namespace, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, content, tags_json, "room", 0.5, namespace, now, now],
    )
    .map_err(|e| CommandError::Uteke(e.to_string()))?;

    Ok(id)
}

#[tauri::command]
pub async fn get_room_document(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    room_id: String,
) -> Result<String, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    // Fetch all memories that share the room:<name> tag
    let room_content: String = conn
        .query_row(
            "SELECT content FROM memories WHERE id = ?1",
            rusqlite::params![room_id],
            |row| row.get(0),
        )
        .map_err(|_| CommandError::NotFound(room_id.clone()))?;

    // Extract room name from content
    let room_name = room_content
        .strip_prefix("# Room: ")
        .and_then(|s| s.trim().strip_suffix('\n').or_else(|| Some(s.trim())))
        .unwrap_or("unnamed");
    let room_tag = format!("\"room:{}\"", room_name);

    // Get all memories with this room tag
    let mut stmt = conn
        .prepare(
            "SELECT content FROM memories WHERE tags LIKE ?1 AND id != ?2 ORDER BY created_at ASC",
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let mut doc = format!("# Room: {}\n\n", room_name);
    let entries: Vec<String> = stmt
        .query_map(
            rusqlite::params![format!("%{}%", room_tag), room_id],
            |row| row.get(0),
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if entries.is_empty() {
        doc.push_str("_No memories in this room yet._\n");
    } else {
        for entry in &entries {
            doc.push_str(entry);
            doc.push_str("\n\n---\n\n");
        }
    }

    Ok(doc)
}

// ---------------------------------------------------------------------------
// Commands: Uteke Integration (read-only)
// ---------------------------------------------------------------------------

/// Check if Uteke database is available.
#[tauri::command]
pub async fn uteke_available(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<bool, CommandError> {
    let s = state.lock().await;
    Ok(s.uteke_conn.is_some())
}

/// List memories from Uteke database (read-only).
///
/// Queries the Uteke DB through the symlink at ~/.codecora/uteke/.
/// Falls back to Hub DB if Uteke is not installed.
#[tauri::command]
pub async fn uteke_list(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    tag: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let mut s = state.lock().await;

    let conn = s.uteke_conn.as_mut().ok_or(CommandError::Uteke(
        "Uteke not installed. Install from https://github.com/codecoradev/uteke".to_string(),
    ))?;

    let limit = limit.unwrap_or(50) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let mut sql = String::from(
        "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at
         FROM memories WHERE deprecated = 0",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref ns) = namespace {
        sql.push_str(" AND namespace = ?");
        params.push(Box::new(ns.clone()));
    }
    if let Some(ref t) = tag {
        sql.push_str(" AND tags LIKE ?");
        params.push(Box::new(format!("%\"{}\"%", t)));
    }

    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let results = stmt
        .query_map(param_refs.as_slice(), |row| {
            let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Get a single memory from Uteke database by ID (read-only).
#[tauri::command]
pub async fn uteke_get(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
) -> Result<MemoryEntry, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    conn.query_row(
        "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at
         FROM memories WHERE id = ?1 AND deprecated = 0",
        rusqlite::params![id],
        |row| {
            let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|_| CommandError::NotFound(id))
}

/// Search memories in Uteke database using FTS5.
#[tauri::command]
pub async fn uteke_search(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    query: String,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, CommandError> {
    let mut s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_mut()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let limit = limit.unwrap_or(10) as i64;

    // Try FTS5 first (Uteke has memories_fts table)
    let mut sql = String::from(
        "SELECT m.id, m.content, m.tags, bm25(memories_fts) as score
         FROM memories_fts
         JOIN memories m ON m.rowid = memories_fts.rowid
         WHERE memories_fts MATCH ? AND m.deprecated = 0",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(query.clone())];

    if let Some(ref ns) = namespace {
        sql.push_str(" AND m.namespace = ?");
        params.push(Box::new(ns.clone()));
    }

    sql.push_str(" ORDER BY score LIMIT ?");
    params.push(Box::new(limit));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let fts_result = conn.prepare(&sql);

    match fts_result {
        Ok(mut stmt) => {
            let results: Vec<SearchResult> = stmt
                .query_map(param_refs.as_slice(), |row| {
                    let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                    let score: f64 = row.get(3).unwrap_or(0.0);
                    Ok(SearchResult {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        score: score as f32,
                        tags,
                    })
                })
                .map_err(|e| CommandError::Uteke(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            if !results.is_empty() {
                return Ok(results);
            }
        }
        Err(e) => {
            eprintln!("FTS5 query failed, falling back to LIKE: {e}");
        }
    }

    // Fallback to LIKE search (with namespace filter if provided)
    let pattern = format!("%{}%", query);
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(ref ns) = namespace {
            (
                "SELECT id, content, tags FROM memories
             WHERE content LIKE ? AND deprecated = 0 AND namespace = ?
             ORDER BY updated_at DESC LIMIT ?"
                    .to_string(),
                vec![Box::new(pattern), Box::new(ns.clone()), Box::new(limit)],
            )
        } else {
            (
                "SELECT id, content, tags FROM memories
             WHERE content LIKE ? AND deprecated = 0
             ORDER BY updated_at DESC LIMIT ?"
                    .to_string(),
                vec![Box::new(pattern), Box::new(limit)],
            )
        };
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    let results = stmt
        .query_map(param_refs.as_slice(), |row| {
            let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(SearchResult {
                id: row.get(0)?,
                content: row.get(1)?,
                score: 0.0,
                tags,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

/// Generate graph data from Uteke memories based on shared tags.
///
/// Two memories are connected if they share at least one tag.
/// This creates an Obsidian-like knowledge graph.
/// Edge weight = number of shared tags.
#[tauri::command]
pub async fn uteke_graph(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
    limit: Option<usize>,
) -> Result<GraphData, CommandError> {
    let mut s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_mut()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let limit = limit.unwrap_or(100) as i64;

    // Get memories with their tags
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(ref ns) = namespace {
            (
            "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at
             FROM memories WHERE deprecated = 0 AND namespace = ?
             ORDER BY updated_at DESC LIMIT ?"
                .to_string(),
            vec![Box::new(ns.clone()), Box::new(limit)],
        )
        } else {
            (
            "SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at
             FROM memories WHERE deprecated = 0
             ORDER BY updated_at DESC LIMIT ?"
                .to_string(),
            vec![Box::new(limit)],
        )
        };
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let nodes: Vec<MemoryEntry> = stmt
        .query_map(param_refs.as_slice(), |row| {
            let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    // Build edge list from shared tags
    use std::collections::{HashMap, HashSet};

    // Map: tag -> list of memory indices
    let mut tag_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        for tag in &node.tags {
            tag_map.entry(tag.clone()).or_default().push(i);
        }
    }

    // For each tag with 2+ memories, create edges between all pairs
    let mut edge_set: HashSet<(usize, usize)> = HashSet::new();
    for indices in tag_map.values() {
        if indices.len() < 2 {
            continue;
        }
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let a = indices[i];
                let b = indices[j];
                let pair = if a < b { (a, b) } else { (b, a) };
                edge_set.insert(pair);
            }
        }
    }

    let edges: Vec<GraphEdge> = edge_set
        .iter()
        .map(|(a, b)| GraphEdge {
            id: None,
            source: nodes[*a].id.clone(),
            target: nodes[*b].id.clone(),
            weight: Some(1.0),
        })
        .collect();

    Ok(GraphData { nodes, edges })
}

/// List distinct namespaces from Uteke database.
#[tauri::command]
pub async fn uteke_namespaces(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let mut stmt = conn
        .prepare("SELECT DISTINCT namespace FROM memories WHERE deprecated = 0 ORDER BY namespace")
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let namespaces = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(namespaces)
}

/// Uteke room entry (maps to Uteke rooms table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeRoom {
    pub id: String,
    pub title: Option<String>,
    pub namespace: String,
    pub memory_count: usize,
    pub participant_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// List rooms from Uteke database.
/// Returns actual Uteke rooms (rooms table + room_memories).
#[tauri::command]
pub async fn uteke_rooms(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    _namespace: Option<String>,
) -> Result<Vec<UtekeRoom>, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.title, r.namespace, r.created_at, r.updated_at,
             (SELECT COUNT(*) FROM room_memories rm WHERE rm.room_id = r.id),
             (SELECT COUNT(DISTINCT rm.author) FROM room_memories rm WHERE rm.room_id = r.id)
             FROM rooms r ORDER BY r.updated_at DESC",
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let rooms = stmt
        .query_map([], |row| {
            let mem_count: i64 = row.get(5).unwrap_or(0);
            let part_count: i64 = row.get(6).unwrap_or(0);
            Ok(UtekeRoom {
                id: row.get(0)?,
                title: row.get(1)?,
                namespace: row.get(2)?,
                memory_count: mem_count as usize,
                participant_count: part_count as usize,
                created_at: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                updated_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rooms)
}

/// Neighbor entry — a memory connected to another with relationship info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborEntry {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub namespace: Option<String>,
    pub importance: Option<f32>,
    pub content_type: Option<String>,
    pub created_at: Option<String>,
    pub relationship: String,
    pub score: Option<f32>,
    pub shared_tags: Vec<String>,
}

/// Find connected memories for a given memory ID.
///
/// Combines:
/// 1. Explicit edges from memory_edges table (if any)
/// 2. Shared-tag neighbors (computed, always works)
#[tauri::command]
pub async fn uteke_neighbors(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    id: String,
    limit: Option<usize>,
) -> Result<Vec<NeighborEntry>, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let limit_i = limit.unwrap_or(20) as i64;

    // Get the source memory's tags
    let source_tags: Vec<String> = conn
        .query_row(
            "SELECT tags FROM memories WHERE id = ?1 AND deprecated = 0",
            rusqlite::params![id],
            |row| {
                let tags_str: String = row.get::<_, Option<String>>(0)?.unwrap_or_default();
                Ok(serde_json::from_str(&tags_str).unwrap_or_default())
            },
        )
        .map_err(|_| CommandError::NotFound(id.clone()))?;

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(id.clone());

    // 1. Explicit edges from memory_edges
    let edge_sql =
        "SELECT m.id, m.content, m.tags, m.namespace, m.importance, m.content_type, m.created_at,
                    e.edge_type, e.weight
                    FROM memory_edges e
                    JOIN memories m ON (m.id = e.target_id OR m.id = e.source_id)
                    WHERE (e.source_id = ?1 OR e.target_id = ?1) AND m.id != ?1 AND m.deprecated = 0
                    LIMIT ?2";
    if let Ok(mut stmt) = conn.prepare(edge_sql) {
        let edges = stmt
            .query_map(rusqlite::params![id, limit_i], |row| {
                let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(NeighborEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    tags,
                    namespace: row.get(3)?,
                    importance: row.get(4)?,
                    content_type: row.get(5)?,
                    created_at: row.get(6)?,
                    relationship: row
                        .get::<_, String>(7)
                        .unwrap_or_else(|_| "related".to_string()),
                    score: row.get(8).ok(),
                    shared_tags: vec![],
                })
            })
            .ok();
        if let Some(edges) = edges {
            for e in edges.flatten() {
                if seen.insert(e.id.clone()) {
                    results.push(e);
                }
            }
        }
    }

    // 2. Shared-tag neighbors (computed)
    if !source_tags.is_empty() && results.len() < limit_i as usize {
        // Build tag filter
        let tag_conditions: Vec<String> = source_tags
            .iter()
            .map(|t| format!("tags LIKE '%\"{}\"%'", t.replace('"', "")))
            .collect();
        let tag_sql = format!(
            "SELECT id, content, tags, namespace, importance, content_type, created_at
             FROM memories
             WHERE id != ?1 AND deprecated = 0 AND ({})
             ORDER BY updated_at DESC LIMIT ?2",
            tag_conditions.join(" OR ")
        );

        if let Ok(mut stmt) = conn.prepare(&tag_sql) {
            let tag_neighbors = stmt
                .query_map(rusqlite::params![id, limit_i], |row| {
                    let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                    let shared: Vec<String> = tags
                        .iter()
                        .filter(|t| source_tags.contains(t))
                        .cloned()
                        .collect();
                    let score = if source_tags.is_empty() {
                        0.0
                    } else {
                        shared.len() as f32 / source_tags.len() as f32
                    };
                    Ok(NeighborEntry {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        tags,
                        namespace: row.get(3)?,
                        importance: row.get(4)?,
                        content_type: row.get(5)?,
                        created_at: row.get(6)?,
                        relationship: format!("shared_tag ({})", shared.len()),
                        score: Some(score),
                        shared_tags: shared,
                    })
                })
                .ok();
            if let Some(tag_neighbors) = tag_neighbors {
                for n in tag_neighbors.flatten() {
                    if seen.insert(n.id.clone()) {
                        results.push(n);
                        if results.len() >= limit_i as usize {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Recall memories linked to a Uteke room.
#[tauri::command]
pub async fn uteke_room_recall(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    room_id: String,
    limit: Option<usize>,
) -> Result<Vec<MemoryEntry>, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let limit_i = limit.unwrap_or(50) as i64;

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.content, m.tags, m.content_type, m.importance, m.namespace, m.created_at, m.updated_at
             FROM memories m
             INNER JOIN room_memories rm ON m.id = rm.memory_id
             WHERE rm.room_id = ?1 AND m.deprecated = 0
             ORDER BY rm.joined_at ASC
             LIMIT ?2",
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let results = stmt
        .query_map(rusqlite::params![room_id, limit_i], |row| {
            let tags_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Get Uteke stats.
#[tauri::command]
pub async fn uteke_stats(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<StatsResponse, CommandError> {
    let s = state.lock().await;

    let conn = s
        .uteke_conn
        .as_ref()
        .ok_or(CommandError::Uteke("Uteke not installed".to_string()))?;

    let total_memories: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE deprecated = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_namespaces: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT namespace) FROM memories WHERE deprecated = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_edges: i64 = conn
        .query_row("SELECT COUNT(*) FROM memory_edges", [], |row| row.get(0))
        .unwrap_or(0);

    // Count unique tags
    let tags_str: String = conn
        .query_row(
            "SELECT GROUP_CONCAT(tags) FROM memories WHERE tags IS NOT NULL AND deprecated = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    let all_tags: Vec<String> = tags_str
        .split(']')
        .filter_map(|s| {
            let cleaned = s.trim().trim_start_matches('[').trim();
            if cleaned.is_empty() {
                return None;
            }
            let tags: Vec<String> =
                serde_json::from_str(&format!("[{}]", cleaned)).unwrap_or_default();
            Some(tags)
        })
        .flatten()
        .collect();
    let unique_tags: std::collections::HashSet<String> = all_tags.into_iter().collect();

    let db_size_bytes = s
        .uteke_db_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(StatsResponse {
        total_memories: total_memories as usize,
        total_namespaces: total_namespaces as usize,
        total_tags: unique_tags.len(),
        total_edges: total_edges as usize,
        db_size_bytes,
    })
}

// ---------------------------------------------------------------------------
// Commands: System
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn stats(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<StatsResponse, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    let total_memories: usize = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |row| {
            let count: i64 = row.get(0)?;
            Ok(count)
        })
        .unwrap_or(0) as usize;

    let total_edges: usize = conn
        .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| {
            let count: i64 = row.get(0)?;
            Ok(count)
        })
        .unwrap_or(0) as usize;

    let total_namespaces: usize = conn
        .query_row(
            "SELECT COUNT(DISTINCT namespace) FROM memories WHERE namespace IS NOT NULL",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                Ok(count)
            },
        )
        .unwrap_or(0) as usize;

    let tags_str: String = conn
        .query_row(
            "SELECT GROUP_CONCAT(tags) FROM memories WHERE tags IS NOT NULL AND tags != '[]'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    let all_tags: Vec<String> = tags_str
        .split(']')
        .filter_map(|s| {
            let cleaned = s.trim().trim_start_matches('[').trim();
            if cleaned.is_empty() {
                return None;
            }
            let tags: Vec<String> =
                serde_json::from_str(&format!("[{}]", cleaned)).unwrap_or_default();
            Some(tags)
        })
        .flatten()
        .collect();
    let unique_tags: std::collections::HashSet<String> = all_tags.into_iter().collect();

    let db_path = s.data_dir.as_ref().unwrap().join("uteke.db");
    let db_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(StatsResponse {
        total_memories,
        total_namespaces,
        total_tags: unique_tags.len(),
        total_edges,
        db_size_bytes,
    })
}

#[tauri::command]
pub async fn list_namespaces(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT namespace FROM memories WHERE namespace IS NOT NULL ORDER BY namespace"
    ).map_err(|e| CommandError::Uteke(e.to_string()))?;

    let namespaces = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(namespaces)
}

#[tauri::command]
pub async fn list_tags(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppState>>>,
    namespace: Option<String>,
) -> Result<HashMap<String, usize>, CommandError> {
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    let all_memories: Vec<String> = if let Some(ns) = namespace {
        let mut stmt = conn
            .prepare("SELECT tags FROM memories WHERE namespace = ?1")
            .unwrap();
        stmt.query_map(rusqlite::params![ns], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    } else {
        let mut stmt = conn
            .prepare("SELECT tags FROM memories WHERE tags IS NOT NULL AND tags != '[]'")
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for tags_str in all_memories {
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_str) {
            for tag in tags {
                *tag_counts.entry(tag).or_insert(0) += 1;
            }
        }
    }

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
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
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
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
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
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();

    match format.as_str() {
        "json" => {
            let memories = export_all_memories(conn)?;
            let edges = export_all_edges(conn)?;
            let export = serde_json::json!({
                "version": "1.0",
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "memories": memories,
                "edges": edges,
            });
            Ok(serde_json::to_string_pretty(&export).unwrap_or_default())
        }
        "markdown" => {
            let memories = export_all_memories(conn)?;
            let mut doc = String::from("# Codecora Hub Export\n\n");
            for m in memories {
                doc.push_str(&format!(
                    "## {}\n\n{}\n\n**Tags:** {}\n**Namespace:** {}\n**Created:** {}\n\n---\n\n",
                    m.id,
                    m.content,
                    m.tags.join(", "),
                    m.namespace.unwrap_or_else(|| "-".to_string()),
                    m.created_at.unwrap_or_else(|| "-".to_string()),
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
    let mut s = state.lock().await;
    s.ensure_initialized()?;

    let conn = s.conn.as_mut().unwrap();
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
        let id = m
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| nanoid::nanoid!(12));
        let content = m
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = m
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let tags_json = serde_json::to_string(&tags).unwrap_or_default();
        let content_type = m
            .get("content_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let importance = m
            .get("importance")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);
        let namespace = m
            .get("namespace")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let now = chrono::Utc::now().to_rfc3339();
        // Preserve original timestamps if present, otherwise use now
        let created_at = m
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| now.clone());
        let updated_at = m
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| now.clone());

        conn.execute(
            "INSERT OR REPLACE INTO memories (id, content, tags, content_type, importance, namespace, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, content, tags_json, content_type, importance, namespace, created_at, updated_at],
        )
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
        count += 1;
    }

    // Import edges if present
    if let Some(edges) = parsed.get("edges").and_then(|v| v.as_array()) {
        for e in edges {
            let source = e
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let target = e
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if source.is_empty() || target.is_empty() {
                continue;
            }
            let weight = e.get("weight").and_then(|v| v.as_f64()).map(|f| f as f32);
            let created_at = e
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            // Skip if edge with same source+target already exists (dedup)
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM graph_edges WHERE source = ?1 AND target = ?2)",
                    rusqlite::params![source, target],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if exists {
                continue;
            }

            conn.execute(
                "INSERT INTO graph_edges (source, target, weight, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![source, target, weight, created_at],
            )
            .map_err(|err| CommandError::Uteke(err.to_string()))?;
        }
    }

    Ok(count)
}

/// Helper: fetch all memories for export.
fn export_all_memories(conn: &rusqlite::Connection) -> Result<Vec<MemoryEntry>, CommandError> {
    let mut stmt = conn
        .prepare("SELECT id, content, tags, content_type, importance, namespace, created_at, updated_at FROM memories")
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    let entries = stmt
        .query_map([], |row| {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                tags,
                content_type: row.get(3)?,
                importance: row.get(4)?,
                namespace: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

/// Helper: fetch all edges for export.
fn export_all_edges(conn: &rusqlite::Connection) -> Result<Vec<GraphEdge>, CommandError> {
    let mut stmt = conn
        .prepare("SELECT id, source, target, weight FROM graph_edges")
        .map_err(|e| CommandError::Uteke(e.to_string()))?;
    let edges = stmt
        .query_map([], |row| {
            Ok(GraphEdge {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                weight: row.get(3)?,
            })
        })
        .map_err(|e| CommandError::Uteke(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(edges)
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

    // Auto-initialize ~/.codecora/ environment
    let (db_path, _config) =
        crate::config::init_environment().map_err(|e| CommandError::Io(e.to_string()))?;

    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| CommandError::Uteke(e.to_string()))?;

    // Ensure schema exists (same as setup hook in lib.rs)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '[]',
            content_type TEXT,
            importance REAL,
            namespace TEXT,
            created_at TEXT,
            updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS graph_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            edge_type TEXT DEFAULT 'related',
            weight REAL DEFAULT 1.0,
            created_at TEXT
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
        CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories(updated_at);
        CREATE INDEX IF NOT EXISTS idx_edges_source ON graph_edges(source);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON graph_edges(target);
        ",
    )
    .map_err(|e| CommandError::Uteke(e.to_string()))?;

    let hub_dir = crate::config::hub_dir().map_err(|e| CommandError::Io(e.to_string()))?;

    s.data_dir = Some(hub_dir.clone());
    s.db_path = Some(db_path);
    s.conn = Some(conn);

    Ok(hub_dir.to_string_lossy().to_string())
}
