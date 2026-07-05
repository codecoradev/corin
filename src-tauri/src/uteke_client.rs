//! Uteke HTTP client — talks to uteke-serve (default: localhost:8767).
//!
//! The actual server URL is resolved dynamically by
//! [`crate::config::detect_uteke_serve_url`] from `~/.uteke/config.toml`,
//! falling back to [`DEFAULT_URL`] when the config is missing.
//!
//! Falls back to direct DB access (rusqlite) when server is not running.
//! This gives CorIn semantic search, cosine auto-linking, and graph API
//! without bundling the full uteke-core library (avoids dep conflicts).

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

/// Full document from uteke-serve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeDocument {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub namespace: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub version: i64,
    pub content_type: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub depth: i64,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub has_children: bool,
}

/// Document summary (for list views — no content/metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeDocumentSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub namespace: String,
    pub version: i64,
    pub updated_at: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub depth: i64,
    #[serde(default)]
    pub has_children: bool,
    #[serde(default)]
    pub sort_order: i64,
}

/// Document search result (semantic + FTS5 hybrid).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtekeDocSearchResult {
    pub document: UtekeDocumentSummary,
    #[serde(default)]
    pub chunk_heading: String,
    #[serde(default)]
    pub chunk_snippet: String,
    pub score: f32,
    #[serde(default)]
    pub mode: String,
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

    /// Add a graph edge between two memories (HTTP-only, uteke >= 0.6.5).
    ///
    /// Returns `Ok(true)` on success. The server validates both source/target
    /// exist and rejects self-loops (returns error string on failure).
    pub async fn graph_add_edge(
        &self,
        source: &str,
        target: &str,
        edge_type: Option<&str>,
        weight: Option<f64>,
    ) -> Result<(), String> {
        let mut body = serde_json::json!({
            "source": source,
            "target": target,
        });
        if let Some(et) = edge_type {
            body["edge_type"] = serde_json::Value::String(et.to_string());
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

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            Err(format!("graph add_edge failed ({status}): {body_text}"))
        }
    }

    /// Remove a graph edge by source + target (HTTP-only, uteke >= 0.6.5).
    ///
    /// Returns `Ok(true)` if edge was found and removed,
    /// `Err` if edge not found or server error.
    pub async fn graph_remove_edge(
        &self,
        source: &str,
        target: &str,
    ) -> Result<bool, String> {
        let resp = self
            .authed(
                self.client
                    .delete(format!("{}/graph/edge", self.base_url))
                    .query(&[("source", source), ("target", target)]),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            Ok(true)
        } else if resp.status().as_u16() == 404 {
            Ok(false) // edge not found
        } else {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            Err(format!("graph remove_edge failed ({status}): {body_text}"))
        }
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

    /// Get recent memories (sorted by updated_at DESC, uteke >= 0.6.4).
    ///
    /// `GET /recent?limit=N&namespace=X&offset=M`
    /// Returns the same `Vec<UtekeMemory>` shape as `/list`.
    pub async fn recent(
        &self,
        limit: Option<usize>,
        namespace: Option<&str>,
        offset: Option<usize>,
    ) -> Result<Vec<UtekeMemory>, String> {
        let mut req = self
            .authed(self.client.get(format!("{}/recent", self.base_url)));
        if let Some(lim) = limit {
            req = req.query(&[("limit", lim.to_string())]);
        }
        if let Some(ns) = namespace {
            req = req.query(&[("namespace", ns)]);
        }
        if let Some(off) = offset {
            req = req.query(&[("offset", off.to_string())]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            resp.json().await.map_err(|e| e.to_string())
        } else {
            // Fallback: older server may not have /recent → use /list.
            let _ = resp.text().await; // consume body
            self.list(namespace, None, limit.unwrap_or(20), offset.unwrap_or(0))
                .await
        }
    }

    /// Room recall — memories linked to a room.
    pub async fn room_recall(
        &self,
        room_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RecallResult>, String> {
        let body = serde_json::json!({
            "room_id": room_id,
            "query": query,
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

    // ── Document API ──────────────────────────────────────────────────

    /// List documents with optional namespace filter.
    pub async fn doc_list(
        &self,
        namespace: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<UtekeDocumentSummary>, String> {
        let mut body = serde_json::json!({ "limit": limit.unwrap_or(50) });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/list", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// List root documents (parent_id IS NULL).
    pub async fn doc_list_roots(
        &self,
        namespace: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<UtekeDocumentSummary>, String> {
        let mut body = serde_json::json!({
            "limit": limit.unwrap_or(50),
            "roots_only": true,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/list", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// List children of a document.
    pub async fn doc_children(
        &self,
        parent_id: &str,
        namespace: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<UtekeDocumentSummary>, String> {
        let mut body = serde_json::json!({
            "parent": parent_id,
            "limit": limit.unwrap_or(50),
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/list", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// Get a single document by slug (preferred) or ID.
    pub async fn doc_get(
        &self,
        id_or_slug: &str,
        namespace: Option<&str>,
    ) -> Result<UtekeDocument, String> {
        // Try slug first, then fall back to ID.
        let mut body = serde_json::json!({ "slug": id_or_slug });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        if let Ok(doc) = self
            .authed(
                self.client
                    .post(format!("{}/doc/get", self.base_url))
                    .json(&body),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<UtekeDocument>()
            .await
        {
            return Ok(doc);
        }
        // Fallback: try by ID.
        let mut body_id = serde_json::json!({ "id": id_or_slug });
        if let Some(ns) = namespace {
            body_id["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/get", self.base_url))
                .json(&body_id),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// Create or update a document.
    pub async fn doc_create(
        &self,
        slug: &str,
        title: &str,
        content: &str,
        namespace: Option<&str>,
        tags: Vec<String>,
        parent_id: Option<&str>,
    ) -> Result<UtekeDocument, String> {
        let mut body = serde_json::json!({
            "slug": slug,
            "title": title,
            "content": content,
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        if !tags.is_empty() {
            body["tags"] = serde_json::Value::Array(
                tags.iter()
                    .map(|t| serde_json::Value::String(t.clone()))
                    .collect(),
            );
        }
        if let Some(pid) = parent_id {
            body["parent"] = serde_json::Value::String(pid.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/create", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// Search documents (hybrid semantic + FTS5).
    pub async fn doc_search(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<u32>,
        mode: Option<&str>,
    ) -> Result<Vec<UtekeDocSearchResult>, String> {
        let mut body = serde_json::json!({
            "query": query,
            "limit": limit.unwrap_or(20),
            "mode": mode.unwrap_or("hybrid"),
        });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/search", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// Delete a document by slug or ID (cascades to children).
    pub async fn doc_delete(
        &self,
        id_or_slug: &str,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        // Use slug-based deletion.
        let mut body = serde_json::json!({ "slug": id_or_slug });
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .delete(format!("{}/doc/delete", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    /// Move a document to a new parent.
    pub async fn doc_move(
        &self,
        id_or_slug: &str,
        new_parent: Option<&str>,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({ "slug": id_or_slug });
        if let Some(np) = new_parent {
            body["new_parent"] = serde_json::Value::String(np.to_string());
        }
        if let Some(ns) = namespace {
            body["namespace"] = serde_json::Value::String(ns.to_string());
        }
        self.authed(
            self.client
                .post(format!("{}/doc/move", self.base_url))
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }
}
