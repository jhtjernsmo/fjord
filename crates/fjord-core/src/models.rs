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
    /// "no"/"nb"/"nn" gives Norwegian column names; anything else English.
    pub locale: Option<String>,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StatusPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_done: Option<bool>,
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
    /// `Some(None)` clears the due date. In JSON: field missing = keep, `null` = clear.
    #[serde(default, deserialize_with = "present_or_null")]
    pub due_at: Option<Option<String>>,
}

/// Distinguishes a JSON field that is present (even as `null`) from one that is missing.
fn present_or_null<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_patch_due_missing_keeps_and_null_clears() {
        let keep: TaskPatch = serde_json::from_str(r#"{"title":"x"}"#).unwrap();
        let clear: TaskPatch = serde_json::from_str(r#"{"due_at":null}"#).unwrap();
        let set: TaskPatch = serde_json::from_str(r#"{"due_at":"2026-01-01"}"#).unwrap();
        assert_eq!(keep.due_at, None);
        assert_eq!(clear.due_at, Some(None));
        assert_eq!(set.due_at, Some(Some("2026-01-01".into())));
    }

    #[test]
    fn ui_payloads_deserialize_with_defaults() {
        let task: NewTask =
            serde_json::from_str(r#"{"project_id":1,"title":"x","status_id":11}"#).unwrap();
        assert_eq!(
            (task.priority, task.body_md.as_str(), task.status_id),
            (0, "", Some(11))
        );
        let project: NewProject =
            serde_json::from_str(r##"{"name":"Bokost","color":"#00d4b0","icon":"💸"}"##).unwrap();
        assert_eq!(
            (project.description.as_str(), project.icon.as_deref()),
            ("", Some("💸"))
        );
    }
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
    /// e.g. "task.move"; UIs turn action + subject + detail into a sentence.
    pub action: String,
    /// Name of the thing acted on (task title, file name, …).
    pub subject: String,
    /// Extra context, e.g. the target status of a move.
    pub detail: Option<String>,
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
