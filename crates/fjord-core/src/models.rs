use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    #[serde(flatten)]
    pub project: Project,
    pub task_count: i64,
    pub done_count: i64,
    pub overdue_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub is_done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub status_id: i64,
    pub title: String,
    pub body_md: String,
    pub priority: i64,
    pub due_at: Option<String>,
    pub position: f64,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewTask {
    pub project_id: i64,
    pub title: String,
    #[serde(default)]
    pub body_md: String,
    #[serde(default)]
    pub priority: i64,
    pub due_at: Option<String>,
    /// Defaults to the project's first status.
    pub status_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub body_md: Option<String>,
    pub priority: Option<i64>,
    /// `Some(None)` clears the due date.
    pub due_at: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub project_id: i64,
    pub task_id: Option<i64>,
    pub original_name: String,
    pub sha256: String,
    pub size: i64,
    pub added_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub body_md: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    pub id: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub actor: String,
    pub action: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: String,
    pub ref_id: i64,
    pub project_id: i64,
    pub title: String,
    pub snippet: String,
}
