//! Uteke HTTP client — talks to uteke-serve (default: localhost:8767).
//!
//! The actual server URL is resolved dynamically by
//! [`crate::config::detect_uteke_serve_url`] from `~/.uteke/config.toml`,
//! falling back to [`DEFAULT_URL`] when the config is missing.
//!
//! Corin is a pure HTTP client to uteke-serve — no native uteke-core
//! dependency. All operations (memory CRUD, graph, rooms) go through
//! the HTTP API.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default Uteke serve URL (used when `~/.uteke/config.toml` is unreadable).
pub const DEFAULT_URL: &str = "http://127.0.0.1:8767";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeMemory {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub namespace: String,
    pub importance: f32,
    pub memory_type: String,
    pub content_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub memory: UtekeMemory,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeStats {
    pub total_memories: usize,
    pub unique_tags: usize,
    pub db_size_bytes: u64,
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub relation_types: Vec<String>,
}

/// A namespace with its memory count.
///
/// Returned by `/namespaces?with_counts=true` (uteke >= #527).
/// On older servers, `count` is `0` (unknown) — callers should treat 0 as
/// "count unavailable" rather than "empty".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceCount {
    pub name: String,
    pub count: usize,
}

/// A tag with its usage count.
///
/// Returned by GET /tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    pub count: usize,
}

/// A single timeline event for a memory.
///
/// Returned by GET /timeline?id=...&limit=...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: i64,
    pub memory_id: String,
    pub event_type: String,
    pub event_data: Option<String>,
    pub created_at: String,
}

/// A single edge between two memories.
///
/// Returned as part of EdgeList from GET /edges?id=...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub created_at: String,
}

/// Edge list for a memory (outgoing + incoming).
///
/// Returned by GET /edges?id=...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeList {
    pub memory_id: String,
    pub outgoing: Vec<MemoryEdge>,
    pub incoming: Vec<MemoryEdge>,
}

// ── Dream Cycle (maintenance pipeline) ──────────────────────────────

/// Result of a single dream phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: String,
    pub status: String,
    pub summary: String,
    pub changes: usize,
    pub warnings: usize,
}

/// Full dream cycle report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamReport {
    pub phases: Vec<PhaseResult>,
    pub total_changes: usize,
    pub total_warnings: usize,
    pub total_errors: usize,
    pub dry_run: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeRoom {
    pub id: String,
    pub title: Option<String>,
    pub namespace: String,
    pub created_at: String,
    pub updated_at: String,
}

/// HTTP client for uteke-serve.
#[derive(Clone)]
pub struct UtekeClient {
    client: reqwest::Client,
    base_url: String,
    /// Optional bearer token for authenticated endpoints.
    auth_token: Option<String>,
}

impl Default for UtekeClient {
    fn default() -> Self {
        Self::new(DEFAULT_URL)
    }
}

impl UtekeClient {
    /// Create a client without auth.
    pub fn new(base_url: &str) -> Self {
        Self::with_auth(base_url, None)
    }

    /// Create a client with an optional bearer token.
    pub fn with_auth(base_url: &str, auth_token: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: base_url.to_string(),
            auth_token,
        }
    }

    /// Inject bearer auth token into a request builder if configured.
    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    /// Check if uteke-serve is reachable.
    pub async fn is_available(&self) -> bool {
        self.authed(self.client.get(format!("{}/health", self.base_url)))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Semantic recall (vector + FTS5 hybrid search via RRF).
    pub async fn recall(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecallResult>, String> {
        let mut body = serde_json::json!({
            "query": query,
            "limit": limit,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        self.authed(self.client.post(format!("{}/recall", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Keyword search (FTS5 only, faster than recall).
    pub async fn search(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecallResult>, String> {
        let mut body = serde_json::json!({
            "query": query,
            "limit": limit,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        self.authed(self.client.post(format!("{}/search", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// List memories with optional filters.
    pub async fn list(
        &self,
        namespace: Option<&str>,
        tag: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<UtekeMemory>, String> {
        let mut body = serde_json::json!({
            "limit": limit,
            "offset": offset,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        if let Some(t) = tag {
            body["tag"] = serde_json::Value::String(t.to_string());
        }

        self.authed(self.client.post(format!("{}/list", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Get a single memory by ID.
    pub async fn get(&self, id: &str) -> Result<UtekeMemory, String> {
        let resp = self
            .authed(self.client.get(format!("{}/memory", self.base_url)))
            .query(&[("id", id)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Get stats.
    pub async fn stats(&self) -> Result<UtekeStats, String> {
        self.authed(self.client.get(format!("{}/stats", self.base_url)))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// List namespaces.
    pub async fn namespaces(&self) -> Result<Vec<String>, String> {
        self.authed(self.client.get(format!("{}/namespaces", self.base_url)))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// List namespaces with memory counts.
    ///
    /// Backward-compatible: tries `?with_counts=true` (uteke >= #527).
    /// Falls back to plain `/namespaces` with count `0` (unknown) on older
    /// servers that return 404.
    pub async fn namespaces_with_counts(&self) -> Result<Vec<NamespaceCount>, String> {
        let resp = self
            .authed(
                self.client
                    .get(format!("{}/namespaces", self.base_url))
                    .query(&[("with_counts", "true")]),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?;

        // New server returns 200 with [{name, count}].
        if resp.status().is_success() {
            return resp.json().await.map_err(|e| e.to_string());
        }

        // Older server → 404. Fall back to plain list with unknown counts.
        // Consume the error body so the connection can be reused.
        let _ = resp.text().await;
        let names = self.namespaces().await?;
        Ok(names
            .into_iter()
            .map(|name| NamespaceCount { name, count: 0 })
            .collect())
    }

    /// Get graph data (nodes + edges from memory_edges + graph_edges).
    pub async fn graph(&self, namespace: Option<&str>) -> Result<GraphResponse, String> {
        let mut req = self.authed(self.client.get(format!("{}/graph", self.base_url)));
        if let Some(ns) = namespace {
            req = req.query(&[("namespace", ns)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Add an edge between two memories (POST /graph/edge).
    pub async fn add_edge(
        &self,
        source: &str,
        target: &str,
        relation: Option<&str>,
        weight: Option<f32>,
    ) -> Result<(), String> {
        let mut body = serde_json::json!({
            "source": source,
            "target": target,
        });
        if let Some(rel) = relation {
            body["relation"] = serde_json::Value::String(rel.to_string());
        }
        if let Some(w) = weight {
            body["weight"] = serde_json::json!(w);
        }

        let resp = self
            .authed(self.client.post(format!("{}/graph/edge", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        // Consume response body.
        let _ = resp.text().await;
        Ok(())
    }

    /// Remove an edge (DELETE /graph/edge).
    pub async fn remove_edge(
        &self,
        source: &str,
        target: &str,
        relation: Option<&str>,
    ) -> Result<(), String> {
        let mut req = self
            .authed(self.client.delete(format!("{}/graph/edge", self.base_url)))
            .query(&[("source", source), ("target", target)]);
        if let Some(rel) = relation {
            req = req.query(&[("relation", rel)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        let _ = resp.text().await;
        Ok(())
    }

    /// Get graph data filtered to neighbors of a specific node.
    ///
    /// Uses GET /graph with `node_id` filter (uteke-serve returns only
    /// edges connected to that node).
    pub async fn graph_neighbors(
        &self,
        node_id: &str,
        namespace: Option<&str>,
    ) -> Result<GraphResponse, String> {
        let mut req = self.authed(self.client.get(format!("{}/graph", self.base_url)));
        req = req.query(&[("node_id", node_id)]);
        if let Some(ns) = namespace {
            req = req.query(&[("namespace", ns)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Get graph statistics only (no nodes/edges).
    pub async fn graph_stats(&self) -> Result<GraphStats, String> {
        let resp = self
            .authed(self.client.get(format!("{}/graph/stats", self.base_url)))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// List rooms.
    pub async fn rooms(&self, namespace: Option<&str>) -> Result<Vec<UtekeRoom>, String> {
        let mut req = self.authed(self.client.get(format!("{}/room/list", self.base_url)));
        if let Some(ns) = namespace {
            req = req.query(&[("namespace", ns)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Create a new memory (also triggers cosine auto-linking).
    pub async fn remember(
        &self,
        content: &str,
        tags: &[String],
        namespace: Option<&str>,
    ) -> Result<String, String> {
        let mut body = serde_json::json!({
            "content": content,
            "tags": tags,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        #[derive(Deserialize)]
        struct RememberResp {
            id: String,
        }

        let resp: RememberResp = self
            .authed(self.client.post(format!("{}/remember", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        Ok(resp.id)
    }

    /// Delete a memory by ID.
    pub async fn forget(&self, id: &str) -> Result<(), String> {
        let resp = self
            .authed(self.client.delete(format!("{}/forget", self.base_url)))
            .query(&[("id", id)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        Ok(())
    }

    /// Room recall — memories linked to a room.
    pub async fn room_recall(
        &self,
        room_id: &str,
        limit: usize,
    ) -> Result<Vec<RecallResult>, String> {
        let body = serde_json::json!({
            "room_id": room_id,
            "query": "",
            "limit": limit,
        });

        self.authed(self.client.post(format!("{}/room/recall", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Room summary — topic clustering + structured overview.
    pub async fn room_summary(&self, room_id: &str) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "room_id": room_id,
        });

        self.authed(self.client.post(format!("{}/room/summary", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Room document — structured document from room memories.
    pub async fn room_document(&self, room_id: &str) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "room_id": room_id,
        });

        self.authed(self.client.post(format!("{}/room/document", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Delete a room by ID.
    pub async fn room_delete(&self, room_id: &str) -> Result<(), String> {
        let resp = self
            .authed(self.client.delete(format!("{}/room/delete", self.base_url)))
            .query(&[("room_id", room_id)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        Ok(())
    }

    /// Create a new room.
    pub async fn room_create(
        &self,
        room_id: &str,
        title: Option<&str>,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({
            "room_id": room_id,
        });
        if let Some(t) = title {
            body["title"] = serde_json::Value::String(t.to_string());
        }
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        self.authed(self.client.post(format!("{}/room/create", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ── Tags ─────────────────────────────────────────────────────────────

    /// List tags with usage counts, optionally filtered by namespace.
    pub async fn list_tags(&self, namespace: Option<&str>) -> Result<Vec<TagInfo>, String> {
        let mut req = self.authed(self.client.get(format!("{}/tags", self.base_url)));
        if let Some(ns) = namespace {
            req = req.query(&[("namespace", ns)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Rename a tag across a namespace.
    pub async fn rename_tag(
        &self,
        old: &str,
        new: &str,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({
            "old": old,
            "new": new,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        self.authed(self.client.post(format!("{}/tags/rename", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Delete a tag from a namespace.
    pub async fn delete_tag(
        &self,
        tag: &str,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({
            "tag": tag,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        self.authed(self.client.post(format!("{}/tags/delete", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ── Pin ──────────────────────────────────────────────────────────────

    /// Pin a memory by ID.
    pub async fn pin(&self, id: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "id": id,
        });

        let resp = self
            .authed(self.client.post(format!("{}/pin", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        let _ = resp.text().await;
        Ok(())
    }

    /// Unpin a memory by ID.
    pub async fn unpin(&self, id: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "id": id,
        });

        let resp = self
            .authed(self.client.post(format!("{}/unpin", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        let _ = resp.text().await;
        Ok(())
    }

    // ── Timeline ─────────────────────────────────────────────────────────

    /// Get timeline events for a memory.
    pub async fn timeline(&self, id: &str, limit: usize) -> Result<Vec<TimelineEvent>, String> {
        let resp = self
            .authed(self.client.get(format!("{}/timeline", self.base_url)))
            .query(&[("id", id), ("limit", &limit.to_string())])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    // ── Edges ────────────────────────────────────────────────────────────

    /// Get edges for a memory (outgoing + incoming).
    pub async fn edges(&self, id: &str) -> Result<EdgeList, String> {
        let resp = self
            .authed(self.client.get(format!("{}/edges", self.base_url)))
            .query(&[("id", id)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    // ── Dream Cycle (maintenance pipeline) ───────────────────────────

    /// Run the dream cycle via HTTP (POST /dream).
    ///
    /// Triggers the full maintenance pipeline on the uteke server:
    /// lint → backlinks → dedup → orphans → compact → verify.
    /// The server handles the heavy lifting (embedding model, SQLite ops).
    pub async fn dream(
        &self,
        namespace: Option<&str>,
        dry_run: bool,
    ) -> Result<DreamReport, String> {
        let mut body = serde_json::json!({
            "dry_run": dry_run,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }

        let resp = self
            .authed(self.client.post(format!("{}/dream", self.base_url)))
            .json(&body)
            .timeout(Duration::from_secs(300)) // 5 min timeout for large stores
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("server returned {status}: {body_text}"));
        }

        resp.json().await.map_err(|e| e.to_string())
    }
}
