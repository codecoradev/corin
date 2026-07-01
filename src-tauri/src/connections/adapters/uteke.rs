//! Uteke adapter — wraps [`UtekeClient`] and implements [`MemoryBackend`].

use crate::connections::traits::MemoryBackend;
use crate::connections::{
    ConnectionConfig, GraphEdge, GraphNode, GraphResponse, HealthInfo, Memory,
    ProductType, Room, Stats,
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

    async fn health_check(&self, cfg: &ConnectionConfig) -> Result<HealthInfo, String> {
        let start = std::time::Instant::now();
        let url = cfg.url.clone();
        let client = UtekeClient::with_auth(&url, cfg.auth_token.clone());

        // Use the /health endpoint for a simple ping.
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

#[async_trait::async_trait]
impl MemoryBackend for UtekeAdapter {
    async fn recall(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>, String> {
        let limit = limit.unwrap_or(50);
        let results = self.client.recall(query, namespace, limit).await?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn search(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>, String> {
        let limit = limit.unwrap_or(50);
        let results = self.client.search(query, namespace, limit).await?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn remember(
        &self,
        content: &str,
        namespace: Option<&str>,
        _content_type: Option<&str>,
        _importance: Option<f64>,
        tags: Option<&str>,
    ) -> Result<String, String> {
        let parsed_tags: Vec<String> = tags
            .map(|t| {
                if t.starts_with('[') {
                    serde_json::from_str(t).unwrap_or_else(|_| {
                        t.split(',').map(|s| s.trim().to_string()).collect()
                    })
                } else {
                    t.split(',').map(|s| s.trim().to_string()).collect()
                }
            })
            .unwrap_or_default();

        self.client.remember(content, &parsed_tags, namespace).await
    }

    async fn forget(&self, id: &str) -> Result<(), String> {
        self.client.forget(id).await
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>, String> {
        let results = self
            .client
            .list(namespace, None, limit.unwrap_or(100), 0)
            .await?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: &str) -> Result<Memory, String> {
        let mem = self.client.get(id).await?;
        Ok(mem.into())
    }

    async fn stats(&self) -> Result<Stats, String> {
        let s = self.client.stats().await?;
        Ok(Stats {
            total_memories: s.total_memories,
            namespaces: s.unique_tags, // approximate: use tag count as proxy
            tags: s.unique_tags,
            graph_edges: 0,            // not in UtekeStats
            rooms: None,
            version: None,
        })
    }

    async fn namespaces(&self) -> Result<Vec<String>, String> {
        self.client.namespaces().await
    }

    async fn graph(&self, namespace: Option<&str>) -> Result<GraphResponse, String> {
        let g = self.client.graph(namespace).await?;
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

    async fn rooms(&self, namespace: Option<&str>) -> Result<Vec<Room>, String> {
        let rs = self.client.rooms(namespace).await?;
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

    async fn room_recall(
        &self,
        room_id: &str,
        _query: &str,
    ) -> Result<Vec<Memory>, String> {
        // UtekeClient room_recall doesn't support query — return all room memories.
        let results = self.client.room_recall(room_id, 100).await?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn room_summary(&self, room_id: &str) -> Result<serde_json::Value, String> {
        self.client.room_summary(room_id).await
    }

    async fn room_document(&self, room_id: &str) -> Result<serde_json::Value, String> {
        self.client.room_document(room_id).await
    }

    async fn room_delete(&self, room_id: &str) -> Result<(), String> {
        self.client.room_delete(room_id).await
    }

    async fn room_create(
        &self,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        self.client.room_create(name, None, namespace).await
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
