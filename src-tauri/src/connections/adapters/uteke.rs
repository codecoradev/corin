//! Uteke adapter — wraps [`UtekeClient`] and implements [`MemoryBackend`].

use crate::connections::traits::MemoryBackend;
use crate::connections::{
    ConnectionConfig, GraphEdge, GraphNode, GraphResponse, HealthInfo, Memory, ProductType, Room,
    Stats,
};
use crate::uteke_client::UtekeClient;

/// Adapter that wraps the existing HTTP client and converts its types
/// into the generic connection-layer types.
pub struct UtekeAdapter {
    client: UtekeClient,
}

impl UtekeAdapter {
    /// Build from a connection config (URL + optional auth token).
    pub fn new(cfg: &ConnectionConfig) -> Self {
        let client = UtekeClient::with_auth(&cfg.url, cfg.auth_token.clone());
        Self { client }
    }

    /// Build from a URL without auth (legacy / local).
    pub fn from_url(url: &str) -> Self {
        Self {
            client: UtekeClient::new(url),
        }
    }

    /// Get a reference to the underlying client (for direct use in commands.rs).
    pub fn client(&self) -> &UtekeClient {
        &self.client
    }
}

impl crate::connections::traits::ProductAdapter for UtekeAdapter {
    fn product_type(&self) -> ProductType {
        ProductType::Uteke
    }

    fn display_name(&self) -> &'static str {
        "Uteke"
    }

    fn icon(&self) -> &'static str {
        "🔮"
    }

    fn capabilities(&self) -> crate::connections::Capabilities {
        crate::connections::ProductType::Uteke.capabilities()
    }

    fn health_check(
        &self,
        cfg: ConnectionConfig,
    ) -> impl std::future::Future<Output = Result<HealthInfo, String>> + Send {
        let client = UtekeClient::with_auth(&cfg.url, cfg.auth_token.clone());
        async move {
            let start = std::time::Instant::now();
            let available = client.is_available().await;
            let latency_ms = start.elapsed().as_millis() as u64;

            if available {
                Ok(HealthInfo {
                    success: true,
                    latency_ms,
                    version: None,
                    error: None,
                })
            } else {
                Ok(HealthInfo {
                    success: false,
                    latency_ms,
                    version: None,
                    error: Some("Server not reachable".to_string()),
                })
            }
        }
    }
}

impl MemoryBackend for UtekeAdapter {
    fn recall(
        &self,
        query: String,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<Memory>, String>> + Send {
        let limit = limit.unwrap_or(50);
        let client = self.client.clone();
        async move {
            let results = client.recall(&query, namespace.as_deref(), limit).await?;
            Ok(results.into_iter().map(Into::into).collect())
        }
    }

    fn search(
        &self,
        query: String,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<Memory>, String>> + Send {
        let limit = limit.unwrap_or(50);
        let client = self.client.clone();
        async move {
            let results = client.search(&query, namespace.as_deref(), limit).await?;
            Ok(results.into_iter().map(Into::into).collect())
        }
    }

    fn remember(
        &self,
        content: String,
        namespace: Option<String>,
        _content_type: Option<String>,
        _importance: Option<f64>,
        tags: Option<String>,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send {
        let parsed_tags: Vec<String> = tags
            .map(|t| {
                if t.starts_with('[') {
                    serde_json::from_str(&t)
                        .unwrap_or_else(|_| t.split(',').map(|s| s.trim().to_string()).collect())
                } else {
                    t.split(',').map(|s| s.trim().to_string()).collect()
                }
            })
            .unwrap_or_default();
        let client = self.client.clone();
        async move {
            client
                .remember(&content, &parsed_tags, namespace.as_deref())
                .await
        }
    }

    fn forget(&self, id: String) -> impl std::future::Future<Output = Result<(), String>> + Send {
        let client = self.client.clone();
        async move { client.forget(&id).await }
    }

    fn list(
        &self,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<Memory>, String>> + Send {
        let client = self.client.clone();
        async move {
            let results = client
                .list(namespace.as_deref(), None, limit.unwrap_or(100), 0)
                .await?;
            Ok(results.into_iter().map(Into::into).collect())
        }
    }

    fn get(&self, id: String) -> impl std::future::Future<Output = Result<Memory, String>> + Send {
        let client = self.client.clone();
        async move {
            let mem = client.get(&id).await?;
            Ok(mem.into())
        }
    }

    fn stats(&self) -> impl std::future::Future<Output = Result<Stats, String>> + Send {
        let client = self.client.clone();
        async move {
            let s = client.stats().await?;
            Ok(Stats {
                total_memories: s.total_memories,
                namespaces: s.unique_tags,
                tags: s.unique_tags,
                graph_edges: 0,
                rooms: None,
                version: None,
            })
        }
    }

    fn namespaces(&self) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send {
        let client = self.client.clone();
        async move { client.namespaces().await }
    }

    fn graph(
        &self,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<GraphResponse, String>> + Send {
        let client = self.client.clone();
        async move {
            let g = client.graph(namespace.as_deref()).await?;
            Ok(GraphResponse {
                nodes: g
                    .nodes
                    .into_iter()
                    .map(|n| GraphNode {
                        id: n.id,
                        label: Some(n.label),
                        namespace: n.entity_type,
                    })
                    .collect(),
                edges: g
                    .edges
                    .into_iter()
                    .map(|e| GraphEdge {
                        source: e.source,
                        target: e.target,
                        edge_type: Some(e.relation),
                        weight: Some(e.weight as f64),
                    })
                    .collect(),
            })
        }
    }

    fn rooms(
        &self,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<Room>, String>> + Send {
        let client = self.client.clone();
        async move {
            let rs = client.rooms(namespace.as_deref()).await?;
            Ok(rs
                .into_iter()
                .map(|r| Room {
                    id: r.id.clone(),
                    name: r.title.unwrap_or(r.id.clone()),
                    namespace: Some(r.namespace),
                    memory_count: None,
                    created_at: Some(r.created_at),
                })
                .collect())
        }
    }

    fn room_recall(
        &self,
        room_id: String,
        _query: String,
    ) -> impl std::future::Future<Output = Result<Vec<Memory>, String>> + Send {
        let client = self.client.clone();
        async move {
            let results = client.room_recall(&room_id, 100).await?;
            Ok(results.into_iter().map(Into::into).collect())
        }
    }

    fn room_summary(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send {
        let client = self.client.clone();
        async move { client.room_summary(&room_id).await }
    }

    fn room_document(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send {
        let client = self.client.clone();
        async move { client.room_document(&room_id).await }
    }

    fn room_delete(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send {
        let client = self.client.clone();
        async move { client.room_delete(&room_id).await }
    }

    fn room_create(
        &self,
        name: String,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send {
        let client = self.client.clone();
        async move { client.room_create(&name, None, namespace.as_deref()).await }
    }
}

// ─── Type conversions (UtekeClient types → generic connection types) ─

impl From<crate::uteke_client::UtekeMemory> for Memory {
    fn from(m: crate::uteke_client::UtekeMemory) -> Self {
        Self {
            id: m.id,
            content: m.content,
            tags: m.tags,
            content_type: Some(m.content_type),
            importance: Some(m.importance as f64),
            namespace: Some(m.namespace),
            created_at: Some(m.created_at),
            updated_at: Some(m.updated_at),
        }
    }
}

impl From<crate::uteke_client::RecallResult> for Memory {
    fn from(r: crate::uteke_client::RecallResult) -> Self {
        Self {
            id: r.memory.id,
            content: r.memory.content,
            tags: r.memory.tags,
            content_type: Some(r.memory.content_type),
            importance: Some(r.score as f64),
            namespace: Some(r.memory.namespace),
            created_at: Some(r.memory.created_at),
            updated_at: Some(r.memory.updated_at),
        }
    }
}
