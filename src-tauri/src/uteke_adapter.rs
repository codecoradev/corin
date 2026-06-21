//! Uteke adapter — thin abstraction over uteke-core library.
//!
//! Phase 1: Direct SQLite operations (commands.rs uses rusqlite directly).
//! Phase 2: Swap to `use uteke_core as uteke;` for full embedding + graph + FTS5.
//!
//! This module provides the migration path. When uteke-core is published to
//! crates.io, we add it as a Cargo dependency and replace the raw SQLite
//! calls in commands.rs with uteke_core API calls:
//!
//!   commands.rs  →  uteke_adapter::remember(content, tags, ns)
//!   commands.rs  →  uteke_adapter::recall(query, ns, limit)
//!   commands.rs  →  uteke_adapter::graph(ns, limit)
//!
//! Until then, this module is a placeholder documenting the integration plan.
