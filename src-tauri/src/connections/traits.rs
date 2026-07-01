//! Product adapter traits.
//!
//! Every remote product implements [`ProductAdapter`].  Category-specific
//! traits (e.g. [`MemoryBackend`]) extend it with domain operations.

use super::{ConnectionConfig, HealthInfo};

/// Core trait: every registered product type implements this.
///
/// Provides identity, capabilities, and a health-check.  The UI uses
/// `capabilities()` to render product-agnostic connection cards.
pub trait ProductAdapter: Send + Sync {
    fn product_type(&self) -> super::ProductType;
    fn display_name(&self) -> &'static str;
    fn icon(&self) -> &'static str;

    fn capabilities(&self) -> super::Capabilities;

    /// Ping the remote endpoint and return latency + version.
    async fn health_check(&self, cfg: &ConnectionConfig) -> Result<HealthInfo, String>;
}

/// Category trait for memory / knowledge-base backends (uteke).
///
/// Every method mirrors an existing `UtekeClient` method so the
/// migration from direct `UtekeClient` usage is drop-in.
pub trait MemoryBackend: ProductAdapter {
    async fn recall(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<super::Memory>, String>;

    async fn search(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<super::Memory>, String>;

    async fn remember(
        &self,
        content: &str,
        namespace: Option<&str>,
        content_type: Option<&str>,
        importance: Option<f64>,
        tags: Option<&str>,
    ) -> Result<String, String>;

    async fn forget(&self, id: &str) -> Result<(), String>;

    async fn list(
        &self,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<super::Memory>, String>;

    async fn get(&self, id: &str) -> Result<super::Memory, String>;

    async fn stats(&self) -> Result<super::Stats, String>;

    async fn namespaces(&self) -> Result<Vec<String>, String>;

    async fn graph(
        &self,
        namespace: Option<&str>,
    ) -> Result<super::GraphResponse, String>;

    async fn rooms(&self, namespace: Option<&str>) -> Result<Vec<super::Room>, String>;

    async fn room_recall(
        &self,
        room_id: &str,
        query: &str,
    ) -> Result<Vec<super::Memory>, String>;

    async fn room_summary(&self, room_id: &str) -> Result<serde_json::Value, String>;

    async fn room_document(&self, room_id: &str) -> Result<serde_json::Value, String>;

    async fn room_delete(&self, room_id: &str) -> Result<(), String>;

    async fn room_create(
        &self,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, String>;
}
