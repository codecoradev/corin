//! Kanban client — talks to Hermes dashboard plugin API.
//!
//! Architecture:
//!   Corin → reqwest → http://127.0.0.1:9119/api/plugins/kanban/*
//!                      → hermes dashboard plugin_api.py
//!                          → kanban.db (SQLite, WAL)
//!
//! Auth: session bearer token from `hermes dashboard` startup.
//! Zero new Rust deps (reqwest already in Cargo.toml).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Default Hermes dashboard URL.
pub const DEFAULT_URL: &str = "http://127.0.0.1:9119";

/// API prefix for kanban plugin routes.
const API_PREFIX: &str = "/api/plugins/kanban";

// ---------------------------------------------------------------------------
// Response types (mirror plugin_api.py serialization)
// ---------------------------------------------------------------------------

/// A board column (status name + list of task cards).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColumn {
    pub name: String,
    pub tasks: Vec<KanbanTask>,
}

/// Full board response from GET /board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardResponse {
    pub columns: Vec<BoardColumn>,
    pub tenants: Vec<String>,
    pub assignees: Vec<String>,
    pub latest_event_id: i64,
    pub now: i64,
}

/// Task card on the board.
///
/// Mirrors `_task_dict()` from plugin_api.py. Fields are `Option` where the
/// server may omit them (e.g. diagnostics, warnings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    pub status: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub tenant: Option<String>,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub latest_summary: Option<String>,
    /// Link counts { parents, children }.
    #[serde(default)]
    pub link_counts: LinkCounts,
    /// Number of comments.
    #[serde(default)]
    pub comment_count: usize,
    /// Parent→child progress { done, total } if this task has children.
    #[serde(default)]
    pub progress: Option<ProgressCounts>,
    /// Age metrics.
    #[serde(default)]
    pub age: TaskAge,
    /// Warnings summary (optional, may be null).
    #[serde(default)]
    pub warnings: Option<WarningsSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkCounts {
    #[serde(default)]
    pub parents: usize,
    #[serde(default)]
    pub children: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgressCounts {
    #[serde(default)]
    pub done: usize,
    #[serde(default)]
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskAge {
    #[serde(default)]
    pub created_age_seconds: Option<f64>,
    #[serde(default)]
    pub started_age_seconds: Option<f64>,
    #[serde(default)]
    pub time_to_complete_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WarningsSummary {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub highest_severity: Option<String>,
    #[serde(default)]
    pub latest_at: Option<i64>,
}

/// Task detail response from GET /tasks/:id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    pub task: KanbanTask,
    pub comments: Vec<Comment>,
    pub events: Vec<TaskEvent>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub links: TaskLinks,
    #[serde(default)]
    pub runs: Vec<TaskRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub task_id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: i64,
    pub task_id: String,
    pub kind: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: String,
    pub run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub task_id: String,
    pub filename: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub size: Option<usize>,
    pub uploaded_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskLinks {
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: i64,
    pub task_id: String,
    pub profile: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Board list item from GET /boards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInfo {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub counts: HashMap<String, usize>,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub is_current: bool,
}

/// Board list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardsResponse {
    pub boards: Vec<BoardInfo>,
    pub current: Option<String>,
}

/// Stats response from GET /stats.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KanbanStats {
    #[serde(default)]
    pub by_status: HashMap<String, usize>,
    #[serde(default)]
    pub by_assignee: HashMap<String, HashMap<String, usize>>,
    #[serde(default)]
    pub oldest_ready_age_seconds: Option<i64>,
    #[serde(default)]
    pub now: Option<i64>,
}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

/// HTTP client for the Hermes kanban dashboard plugin.
#[derive(Clone)]
pub struct KanbanClient {
    client: reqwest::Client,
    base_url: String,
    auth_token: Option<String>,
}

impl Default for KanbanClient {
    fn default() -> Self {
        Self::new(DEFAULT_URL)
    }
}

impl KanbanClient {
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

    /// Full API base (base_url + prefix).
    fn api(&self) -> String {
        format!("{}{}", self.base_url, API_PREFIX)
    }

    /// Inject bearer auth token.
    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    /// Check if Hermes dashboard kanban API is reachable.
    pub async fn is_available(&self) -> bool {
        self.authed(self.client.get(format!("{}/board", self.api())))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Health check — tries the dashboard root first, falls back to /board.
    pub async fn health_check(&self) -> Result<bool, String> {
        // Try dashboard root (200 = dashboard running).
        match self.client.get(format!("{}/", self.base_url)).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(true),
            _ => {}
        }
        // Fallback: try kanban board endpoint.
        Ok(self.is_available().await)
    }

    /// Get the full board grouped by status columns.
    pub async fn get_board(
        &self,
        board: Option<&str>,
        tenant: Option<&str>,
        include_archived: bool,
    ) -> Result<BoardResponse, String> {
        let mut req = self.authed(self.client.get(format!("{}/board", self.api())));
        if let Some(b) = board {
            req = req.query(&[("board", b)]);
        }
        if let Some(t) = tenant {
            req = req.query(&[("tenant", t)]);
        }
        if include_archived {
            req = req.query(&[("include_archived", "true")]);
        }

        req.send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Get task detail with comments, events, runs, links.
    pub async fn get_task(&self, task_id: &str, board: Option<&str>) -> Result<TaskDetail, String> {
        let mut req = self.authed(
            self.client
                .get(format!("{}/tasks/{}", self.api(), task_id)),
        );
        if let Some(b) = board {
            req = req.query(&[("board", b)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    /// Get board stats (per-status + per-assignee counts).
    pub async fn get_stats(&self, board: Option<&str>) -> Result<KanbanStats, String> {
        let mut req = self.authed(self.client.get(format!("{}/stats", self.api())));
        if let Some(b) = board {
            req = req.query(&[("board", b)]);
        }

        req.send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// List all boards.
    pub async fn list_boards(&self, include_archived: bool) -> Result<BoardsResponse, String> {
        let mut req = self.authed(self.client.get(format!("{}/boards", self.api())));
        if include_archived {
            req = req.query(&[("include_archived", "true")]);
        }

        req.send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    /// Add a comment to a task.
    pub async fn add_comment(
        &self,
        task_id: &str,
        author: &str,
        body: &str,
        board: Option<&str>,
    ) -> Result<(), String> {
        let mut req = self.authed(
            self.client
                .post(format!("{}/tasks/{}/comments", self.api(), task_id))
                .json(&serde_json::json!({ "author": author, "body": body })),
        );
        if let Some(b) = board {
            req = req.query(&[("board", b)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }
        let _ = resp.text().await;
        Ok(())
    }

    /// Update a task (status, title, body, priority, assignee).
    pub async fn update_task(
        &self,
        task_id: &str,
        updates: serde_json::Value,
        board: Option<&str>,
    ) -> Result<(), String> {
        let mut req = self.authed(
            self.client
                .patch(format!("{}/tasks/{}", self.api(), task_id))
                .json(&updates),
        );
        if let Some(b) = board {
            req = req.query(&[("board", b)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("server returned {}", resp.status()));
        }
        let _ = resp.text().await;
        Ok(())
    }
}
