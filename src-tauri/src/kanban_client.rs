//! Kanban HTTP client — talks to Hermes dashboard kanban plugin API.
//!
//! Mounted at `/api/plugins/kanban/` by the dashboard plugin system.
//! Every request must carry the session bearer token (or session cookie).
//!
//! Default URL: `http://127.0.0.1:9119`.
//! Auth: `Bearer <session_token>` (from `HERMES_DASHBOARD_SESSION_TOKEN` env,
//! or user-configured in Corin settings).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default Hermes dashboard URL.
pub const DEFAULT_URL: &str = "http://127.0.0.1:9119";

/// Kanban API base path.
const API_PATH: &str = "api/plugins/kanban";

// ---------------------------------------------------------------------------
// Serializable types (mirrors dashboard plugin_api.py responses)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    pub columns: Vec<KanbanColumn>,
    pub tenants: Vec<String>,
    pub assignees: Vec<String>,
    pub latest_event_id: i64,
    pub now: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanColumn {
    pub name: String,
    pub tasks: Vec<KanbanTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub assignee: Option<String>,
    pub status: String,
    pub priority: i32,
    pub tenant: Option<String>,
    pub workspace_kind: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub result: Option<String>,
    pub latest_summary: Option<String>,
    pub age: Option<TaskAge>,
    pub link_counts: Option<LinkCounts>,
    pub comment_count: Option<i64>,
    pub progress: Option<TaskProgress>,
    pub diagnostics: Option<Vec<serde_json::Value>>,
    pub warnings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAge {
    pub created_age_seconds: Option<f64>,
    pub started_age_seconds: Option<f64>,
    pub time_to_complete_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkCounts {
    pub parents: i64,
    pub children: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub done: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTaskDetail {
    pub task: KanbanTask,
    pub comments: Vec<KanbanComment>,
    pub events: Vec<KanbanEvent>,
    pub links: KanbanLinks,
    pub runs: Vec<KanbanRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanComment {
    pub id: i64,
    pub task_id: String,
    pub author: String,
    pub body: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanEvent {
    pub id: i64,
    pub task_id: String,
    pub kind: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: i64,
    pub run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanLinks {
    pub parents: Vec<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanRun {
    pub id: i64,
    pub profile: String,
    pub status: String,
    pub outcome: Option<String>,
    pub summary: Option<String>,
    pub error: Option<String>,
    pub worker_pid: Option<u32>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanStats {
    #[serde(flatten)]
    pub inner: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub body: Option<String>,
    pub assignee: Option<String>,
    pub tenant: Option<String>,
    pub priority: Option<i32>,
    pub triage: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<i32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub result: Option<String>,
    pub summary: Option<String>,
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCommentRequest {
    pub body: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskResponse {
    pub task: Option<KanbanTask>,
    pub warning: Option<String>,
}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

/// HTTP client for the Hermes dashboard kanban plugin.
#[derive(Clone)]
pub struct KanbanClient {
    client: reqwest::Client,
    base_url: String,
    session_token: Option<String>,
}

impl Default for KanbanClient {
    fn default() -> Self {
        Self::new(DEFAULT_URL, None)
    }
}

impl KanbanClient {
    /// Create a kanban client with dashboard URL and optional session token.
    pub fn new(base_url: &str, session_token: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: base_url.to_string(),
            session_token,
        }
    }

    /// Resolve the session token: env var → user-configured.
    pub fn resolve_session_token(user_token: Option<String>) -> Option<String> {
        // User-configured token takes priority (stored in Corin settings).
        if let Some(t) = user_token {
            if !t.is_empty() {
                return Some(t);
            }
        }
        // Env var fallback.
        std::env::var("HERMES_DASHBOARD_SESSION_TOKEN").ok()
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}/{}", self.base_url, API_PATH, path)
    }

    /// Inject session token as Bearer auth.
    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.session_token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    /// Check if the Hermes dashboard is reachable.
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Check if the kanban plugin API is available (auth check).
    pub async fn is_authenticated(&self) -> bool {
        self.authed(self.client.get(self.api_url("board")))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    // ---- Board ----

    /// Fetch the full kanban board grouped by columns.
    pub async fn get_board(
        &self,
        tenant: Option<&str>,
        include_archived: bool,
    ) -> Result<KanbanBoard, String> {
        let mut req = self.authed(self.client.get(self.api_url("board")));
        if let Some(t) = tenant {
            req = req.query(&[("tenant", t)]);
        }
        if include_archived {
            req = req.query(&[("include_archived", "true")]);
        }
        req.send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ---- Task detail ----

    /// Fetch a single task with comments, events, links, and runs.
    pub async fn get_task(&self, task_id: &str) -> Result<KanbanTaskDetail, String> {
        self.authed(self.client.get(self.api_url(&format!("tasks/{task_id}"))))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ---- Create task ----

    /// Create a new task on the kanban board.
    pub async fn create_task(
        &self,
        payload: &CreateTaskRequest,
    ) -> Result<CreateTaskResponse, String> {
        self.authed(self.client.post(self.api_url("tasks")))
            .json(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ---- Update task ----

    /// Update a task (status, assignee, priority, title, body).
    pub async fn update_task(
        &self,
        task_id: &str,
        payload: &UpdateTaskRequest,
    ) -> Result<KanbanTask, String> {
        let resp = self
            .authed(
                self.client
                    .patch(self.api_url(&format!("tasks/{task_id}")))
                    .json(payload),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            resp.json().await.map_err(|e| e.to_string())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Try to extract detail from JSON error response.
            if let Ok(detail) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(msg) = detail.get("detail").and_then(|v| v.as_str()) {
                    return Err(format!("{status}: {msg}"));
                }
            }
            Err(format!("{status}: {body}"))
        }
    }

    // ---- Comments ----

    /// Add a comment to a task.
    pub async fn add_comment(
        &self,
        task_id: &str,
        payload: &AddCommentRequest,
    ) -> Result<KanbanComment, String> {
        self.authed(
            self.client
                .post(self.api_url(&format!("tasks/{task_id}/comments")))
                .json(payload),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
    }

    // ---- Stats ----

    /// Fetch board statistics (per-status counts, per-assignee counts).
    pub async fn get_stats(&self) -> Result<KanbanStats, String> {
        self.authed(self.client.get(self.api_url("stats")))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }
}
