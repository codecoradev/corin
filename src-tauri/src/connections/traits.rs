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
    fn health_check(
        &self,
        cfg: ConnectionConfig,
    ) -> impl std::future::Future<Output = Result<HealthInfo, String>> + Send;
}

/// Category trait for memory / knowledge-base backends (uteke).
///
/// Every method mirrors an existing `UtekeClient` method so the
/// migration from direct `UtekeClient` usage is drop-in.
pub trait MemoryBackend: ProductAdapter {
    fn recall(
        &self,
        query: String,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<super::Memory>, String>> + Send;

    fn search(
        &self,
        query: String,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<super::Memory>, String>> + Send;

    fn remember(
        &self,
        content: String,
        namespace: Option<String>,
        content_type: Option<String>,
        importance: Option<f64>,
        tags: Option<String>,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;

    fn forget(&self, id: String) -> impl std::future::Future<Output = Result<(), String>> + Send;

    fn list(
        &self,
        namespace: Option<String>,
        limit: Option<usize>,
    ) -> impl std::future::Future<Output = Result<Vec<super::Memory>, String>> + Send;

    fn get(
        &self,
        id: String,
    ) -> impl std::future::Future<Output = Result<super::Memory, String>> + Send;

    fn stats(&self) -> impl std::future::Future<Output = Result<super::Stats, String>> + Send;

    fn namespaces(&self) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;

    fn graph(
        &self,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<super::GraphResponse, String>> + Send;

    fn rooms(
        &self,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<super::Room>, String>> + Send;

    fn room_recall(
        &self,
        room_id: String,
        query: String,
    ) -> impl std::future::Future<Output = Result<Vec<super::Memory>, String>> + Send;

    fn room_summary(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;

    fn room_document(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;

    fn room_delete(
        &self,
        room_id: String,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;

    fn room_create(
        &self,
        name: String,
        namespace: Option<String>,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}
