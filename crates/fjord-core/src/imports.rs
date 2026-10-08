//! Tasks imported from other systems (Azure Boards work items for now): each
//! imported task remembers where it came from, so re-imports update instead of
//! duplicating, and agents can find the ones nobody has analyzed yet.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{NewTask, Task, TaskPatch};
use crate::store::Store;

const IMPORT_SETTINGS_KEY: &str = "azure.import";

/// One item from an external tracker, already mapped to Fjord's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalItem {
    pub source: String,
    /// Unique within the source, e.g. `contoso/1234`.
    pub external_id: String,
    pub url: String,
    /// Bumps whenever the item changes upstream.
    pub rev: i64,
    pub title: String,
    pub body_md: String,
    pub priority: i64,
    pub due_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExternalLink {
    pub task_id: i64,
    pub source: String,
    pub external_id: String,
    pub url: String,
    pub imported_at: String,
    pub analyzed_at: Option<String>,
}

/// An imported task with its link, for agents and the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ImportedTask {
    pub task: Task,
    pub link: ExternalLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportOutcome {
    Created,
    Updated,
    Unchanged,
}

/// Which Azure DevOps project's work items go into which Fjord project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportMapping {
    pub org: String,
    /// Azure DevOps project name (`System.TeamProject`).
    pub project: String,
    pub fjord_project_id: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportSettings {
    pub enabled: bool,
    pub mappings: Vec<ImportMapping>,
}

impl Store {
    pub fn import_settings(&self) -> Result<ImportSettings> {
        Ok(self
            .get_setting(IMPORT_SETTINGS_KEY)?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    pub fn set_import_settings(&mut self, settings: &ImportSettings) -> Result<()> {
        for m in &settings.mappings {
            self.get_project(m.fjord_project_id)?;
        }
        let json = serde_json::to_string(settings).expect("settings serialize");
        self.set_setting(IMPORT_SETTINGS_KEY, &json)
    }

    pub fn external_link(&self, source: &str, external_id: &str) -> Result<Option<ExternalLink>> {
        Ok(self
            .conn
            .query_row(
                "SELECT task_id, source, external_id, url, imported_at, analyzed_at
                 FROM external_links WHERE source = ?1 AND external_id = ?2",
                params![source, external_id],
                link_from_row,
            )
            .optional()?)
    }

    /// Creates the task the first time an item is seen. Later imports only refresh
    /// title, priority and due date (when the item changed upstream), never the
    /// description, which people and agents may have added to.
    pub fn upsert_imported_task(
        &mut self,
        project_id: i64,
        item: &ExternalItem,
    ) -> Result<(Task, ImportOutcome)> {
        let existing: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT task_id, rev FROM external_links WHERE source = ?1 AND external_id = ?2",
                params![item.source, item.external_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match existing {
            Some((task_id, rev)) if rev >= item.rev => {
                Ok((self.get_task(task_id)?, ImportOutcome::Unchanged))
            }
            Some((task_id, _)) => {
                let task = self.update_task(
                    task_id,
                    TaskPatch {
                        title: Some(item.title.clone()),
                        body_md: None,
                        priority: Some(item.priority),
                        due_at: Some(item.due_at.clone()),
                    },
                )?;
                self.conn.execute(
                    "UPDATE external_links SET rev = ?1, url = ?2 WHERE task_id = ?3",
                    params![item.rev, item.url, task_id],
                )?;
                Ok((task, ImportOutcome::Updated))
            }
            None => {
                let task = self.create_task(NewTask {
                    project_id,
                    title: item.title.clone(),
                    body_md: item.body_md.clone(),
                    priority: item.priority,
                    due_at: item.due_at.clone(),
                    status_id: None,
                    parent_id: None,
                })?;
                self.conn.execute(
                    "INSERT INTO external_links (task_id, source, external_id, url, rev)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![task.id, item.source, item.external_id, item.url, item.rev],
                )?;
                Ok((task, ImportOutcome::Created))
            }
        }
    }

    /// External ids already imported from `source`, with their task ids.
    pub fn external_ids(&self, source: &str) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT external_id, task_id FROM external_links WHERE source = ?1")?;
        let rows = stmt.query_map([source], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Imported tasks an agent hasn't analyzed yet (oldest first), skipping archived ones.
    pub fn unanalyzed_imports(&self) -> Result<Vec<ImportedTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.task_id, l.source, l.external_id, l.url, l.imported_at, l.analyzed_at
             FROM external_links l JOIN tasks t ON t.id = l.task_id
             WHERE l.analyzed_at IS NULL AND t.archived_at IS NULL
             ORDER BY l.imported_at, l.task_id",
        )?;
        let links = stmt
            .query_map([], link_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        links
            .into_iter()
            .map(|link| {
                Ok(ImportedTask {
                    task: self.get_task(link.task_id)?,
                    link,
                })
            })
            .collect()
    }

    pub fn mark_import_analyzed(&mut self, task_id: i64) -> Result<ExternalLink> {
        let changed = self.conn.execute(
            "UPDATE external_links SET analyzed_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE task_id = ?1",
            [task_id],
        )?;
        if changed == 0 {
            return Err(crate::Error::NotFound(format!("imported task {task_id}")));
        }
        Ok(self.conn.query_row(
            "SELECT task_id, source, external_id, url, imported_at, analyzed_at
             FROM external_links WHERE task_id = ?1",
            [task_id],
            link_from_row,
        )?)
    }
}

fn link_from_row(r: &rusqlite::Row) -> rusqlite::Result<ExternalLink> {
    Ok(ExternalLink {
        task_id: r.get(0)?,
        source: r.get(1)?,
        external_id: r.get(2)?,
        url: r.get(3)?,
        imported_at: r.get(4)?,
        analyzed_at: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewProject;

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (Store::open(dir.path(), "tester").unwrap(), dir)
    }

    fn item(rev: i64, title: &str) -> ExternalItem {
        ExternalItem {
            source: "azure".into(),
            external_id: "contoso/42".into(),
            url: "https://dev.azure.com/contoso/App/_workitems/edit/42".into(),
            rev,
            title: title.into(),
            body_md: "From Azure".into(),
            priority: 3,
            due_at: Some("2026-10-20".into()),
        }
    }

    #[test]
    fn imports_once_then_updates_without_touching_the_description() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;

        let (task, outcome) = s.upsert_imported_task(p, &item(1, "Fix login")).unwrap();
        assert_eq!(outcome, ImportOutcome::Created);
        assert_eq!(
            (task.priority, task.due_at.as_deref()),
            (3, Some("2026-10-20"))
        );

        // An agent adds its analysis; the same revision is a no-op.
        s.update_task(
            task.id,
            TaskPatch {
                body_md: Some("From Azure\n\n## Analysis".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            s.upsert_imported_task(p, &item(1, "Fix login")).unwrap().1,
            ImportOutcome::Unchanged
        );

        // A new revision refreshes the title but keeps the analysis.
        let (updated, outcome) = s
            .upsert_imported_task(p, &item(2, "Fix login on iOS"))
            .unwrap();
        assert_eq!(outcome, ImportOutcome::Updated);
        assert_eq!(updated.id, task.id);
        assert_eq!(updated.title, "Fix login on iOS");
        assert!(updated.body_md.contains("## Analysis"));
        assert_eq!(s.list_tasks(p).unwrap().len(), 1, "no duplicates");
    }

    #[test]
    fn agents_see_unanalyzed_imports_until_marked() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let (task, _) = s.upsert_imported_task(p, &item(1, "A")).unwrap();
        let pending = s.unanalyzed_imports().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].link.external_id, "contoso/42");

        let link = s.mark_import_analyzed(task.id).unwrap();
        assert!(link.analyzed_at.is_some());
        assert!(s.unanalyzed_imports().unwrap().is_empty());
        assert!(s.mark_import_analyzed(9999).is_err());

        s.delete_task(task.id).unwrap();
        assert!(
            s.external_link("azure", "contoso/42").unwrap().is_none(),
            "link goes with the task"
        );
    }

    #[test]
    fn import_settings_round_trip_and_validate_projects() {
        let (mut s, _d) = store();
        assert_eq!(s.import_settings().unwrap(), ImportSettings::default());
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let settings = ImportSettings {
            enabled: true,
            mappings: vec![ImportMapping {
                org: "contoso".into(),
                project: "Mobile App".into(),
                fjord_project_id: p,
            }],
        };
        s.set_import_settings(&settings).unwrap();
        assert_eq!(s.import_settings().unwrap(), settings);
        let bad = ImportSettings {
            mappings: vec![ImportMapping {
                fjord_project_id: 999,
                ..settings.mappings[0].clone()
            }],
            ..settings
        };
        assert!(s.set_import_settings(&bad).is_err());
    }
}
