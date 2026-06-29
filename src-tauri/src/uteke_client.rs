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
}

impl Default for UtekeClient {
    fn default() -> Self {
        Self::new(DEFAULT_URL)
    }
}

impl UtekeClient {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    /// Check if uteke-serve is reachable.
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
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

        self.client
            .post(format!("{}/recall", self.base_url))
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

        self.client
            .post(format!("{}/search", self.base_url))
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

        self.client
            .post(format!("{}/list", self.base_url))
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
            .client
            .get(format!("{}/memory", self.base_url))
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
        self.client
            .get(format!("{}/stats", self.base_url))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// List namespaces.
    pub async fn namespaces(&self) -> Result<Vec<String>, String> {
        self.client
            .get(format!("{}/namespaces", self.base_url))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Get graph data (nodes + edges from memory_edges + graph_edges).
    pub async fn graph(&self, namespace: Option<&str>) -> Result<GraphResponse, String> {
        let mut req = self.client.get(format!("{}/graph", self.base_url));
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
        let mut req = self.client.get(format!("{}/room/list", self.base_url));
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
            .client
            .post(format!("{}/remember", self.base_url))
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
            .client
            .delete(format!("{}/forget", self.base_url))
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

        self.client
            .post(format!("{}/room/recall", self.base_url))
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

        self.client
            .post(format!("{}/room/summary", self.base_url))
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

        self.client
            .post(format!("{}/room/document", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }
}
