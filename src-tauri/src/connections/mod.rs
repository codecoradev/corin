//! Generic connection layer for CorIn.
//!
//! Manages remote (and local) product connections in a single `connections`
//! table.  Each connection row holds a URL, optional auth credentials,
//! product-type discriminator, and status metadata.
//!
//! # Architecture
//!
//! ```text
//! connections/
//!   mod.rs          — registry, types, ProductType, ConnectionConfig
//!   traits.rs       — ProductAdapter, MemoryBackend traits
//!   store.rs        — persistence (load / save / test connections in DB)
//!   adapters/
//!     mod.rs
//!     uteke.rs      — UtekeAdapter (wraps HTTP client)
//! ```
//!
//! Adding a new product = implement an adapter + register in `build_adapter()`.

pub mod adapters;
pub mod store;
pub mod traits;

use serde::{Deserialize, Serialize};

/// Supported product types.  Each variant maps to an adapter implementation
/// in [`adapters`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProductType {
    Uteke,
}

impl std::fmt::Display for ProductType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uteke => write!(f, "uteke"),
        }
    }
}

impl ProductType {
    /// Display name shown in the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Uteke => "Uteke",
        }
    }

    /// Emoji icon for the UI.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Uteke => "🔮",
        }
    }

    /// Capabilities flag — describes what operations this product supports.
    pub fn capabilities(&self) -> Capabilities {
        match self {
            Self::Uteke => Capabilities {
                read: true,
                write: true,
                search: true,
                realtime: false,
            },
        }
    }
}

/// Capability flags.  The UI uses these to render context-appropriate actions
/// per connection without hardcoding per-product logic.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub read: bool,
    pub write: bool,
    pub search: bool,
    pub realtime: bool,
}

/// Information returned by a health-check / test-connection call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthInfo {
    pub success: bool,
    pub latency_ms: u64,
    pub version: Option<String>,
    pub error: Option<String>,
}

/// A connection row from the DB, deserialized.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionRow {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub url: String,
    pub auth_type: Option<String>,
    pub auth_token: Option<String>,
    pub metadata: serde_json::Value,
    pub status: String,
    pub is_primary: bool,
    pub created_at: String,
    pub last_tested_at: Option<String>,
}

/// Lightweight info returned to the frontend (token redacted).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub url: String,
    pub has_token: bool,
    pub capabilities: Capabilities,
    pub status: String,
    pub is_primary: bool,
    pub created_at: String,
    pub last_tested_at: Option<String>,
}

impl From<&ConnectionRow> for ConnectionInfo {
    fn from(r: &ConnectionRow) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            product_type: r.product_type,
            url: r.url.clone(),
            has_token: r.auth_token.is_some(),
            capabilities: r.product_type.capabilities(),
            status: r.status.clone(),
            is_primary: r.is_primary,
            created_at: r.created_at.clone(),
            last_tested_at: r.last_tested_at.clone(),
        }
    }
}

/// Resolved connection config used to build an adapter.
#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub url: String,
    pub auth_token: Option<String>,
    pub metadata: serde_json::Value,
}

impl From<&ConnectionRow> for ConnectionConfig {
    fn from(r: &ConnectionRow) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            product_type: r.product_type,
            url: r.url.clone(),
            auth_token: r.auth_token.clone(),
            metadata: r.metadata.clone(),
        }
    }
}

// ─── Shared data types ────────────────────────────────────────────

/// Graph node (memory vertex).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: Option<String>,
    pub namespace: Option<String>,
}

/// Graph edge (link between memories).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: Option<String>,
    pub weight: Option<f64>,
}

/// A memory entry returned by recall / search / list.
/// Mirrors the existing `UtekeMemory` struct.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub content_type: Option<String>,
    pub importance: Option<f64>,
    pub namespace: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Server statistics.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub total_memories: usize,
    pub namespaces: usize,
    pub tags: usize,
    pub graph_edges: usize,
    pub rooms: Option<usize>,
    pub version: Option<String>,
}

/// Graph response.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// A room.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub namespace: Option<String>,
    pub memory_count: Option<usize>,
    pub created_at: Option<String>,
}
