//! Board columns: add, rename/recolor, reorder and remove.

use rusqlite::params;

use crate::error::{Error, Result};
use crate::models::{Status, StatusPatch};
use crate::store::{Store, require_text};

const DEFAULT_COLUMN_COLOR: &str = "#8b8f98";

fn validate_color(color: &str) -> Result<String> {
    let c = color.trim();
    let hex = c.strip_prefix('#').unwrap_or("");
    if (hex.len() == 6 || hex.len() == 3) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(c.to_lowercase())
    } else {
        Err(Error::Invalid(format!(
            "color «{c}» must be #rgb or #rrggbb"
        )))
    }
}

/// Column names that mean "finished", so a new column called e.g. "Resolved"
/// counts as done without having to tick the box.
const DONE_NAMES: &[&str] = &[
    "done",
    "resolved",
    "closed",
    "completed",
    "finished",
    "ferdig",
    "løst",
    "lukket",
    "fullført",
];

fn sounds_done(name: &str) -> bool {
    DONE_NAMES.contains(&name.trim().to_lowercase().as_str())
}

impl Store {
    pub fn get_status(&self, id: i64) -> Result<Status> {
        self.conn
            .query_row(
                "SELECT id, project_id, name, color, position, is_done FROM statuses WHERE id = ?1",
                [id],
                |row| {
                    Ok(Status {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        name: row.get(2)?,
                        color: row.get(3)?,
                        position: row.get(4)?,
                        is_done: row.get(5)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("status {id}")),
                other => other.into(),
            })
    }

    /// Adds a column at the end of the board.
    pub fn create_status(
        &mut self,
        project_id: i64,
        name: &str,
        color: Option<&str>,
        is_done: bool,
    ) -> Result<Status> {
        let name = require_text(name, "column name")?;
        let color = validate_color(color.unwrap_or(DEFAULT_COLUMN_COLOR))?;
        self.get_project(project_id)?;
        let is_done = is_done || sounds_done(&name);
        self.conn.execute(
            "INSERT INTO statuses (project_id, name, color, position, is_done)
             VALUES (?1, ?2, ?3, (SELECT coalesce(max(position), -1) + 1 FROM statuses WHERE project_id = ?1), ?4)",
            params![project_id, name, color, is_done],
        )?;
        let id = self.conn.last_insert_rowid();
        self.touch_project(project_id)?;
        self.log(Some(project_id), None, "status.create", &name, None)?;
        self.get_status(id)
    }

    pub fn update_status(&mut self, id: i64, patch: StatusPatch) -> Result<Status> {
        let current = self.get_status(id)?;
        let name = match &patch.name {
            Some(n) => require_text(n, "column name")?,
            None => current.name.clone(),
        };
        let color = match &patch.color {
            Some(c) => validate_color(c)?,
            None => current.color.clone(),
        };
        self.conn.execute(
            "UPDATE statuses SET name = ?1, color = ?2, is_done = ?3 WHERE id = ?4",
            params![name, color, patch.is_done.unwrap_or(current.is_done), id],
        )?;
        self.touch_project(current.project_id)?;
        self.log(Some(current.project_id), None, "status.update", &name, None)?;
        self.get_status(id)
    }

    /// Moves a column to `index` (0-based, clamped) and renumbers the others.
    pub fn move_status(&mut self, id: i64, index: usize) -> Result<Vec<Status>> {
        let status = self.get_status(id)?;
        let mut order: Vec<i64> = self
            .list_statuses(status.project_id)?
            .into_iter()
            .map(|s| s.id)
            .collect();
        order.retain(|&s| s != id);
        order.insert(index.min(order.len()), id);
        let tx = self.conn.transaction()?;
        for (position, sid) in order.iter().enumerate() {
            tx.execute(
                "UPDATE statuses SET position = ?1 WHERE id = ?2",
                params![position as i64, sid],
            )?;
        }
        tx.commit()?;
        self.touch_project(status.project_id)?;
        self.log(
            Some(status.project_id),
            None,
            "status.move",
            &status.name,
            None,
        )?;
        self.list_statuses(status.project_id)
    }

    /// Removes an empty column. Archived tasks in it move to the first other
    /// column so they can still be restored. The last column can't be removed.
    pub fn delete_status(&mut self, id: i64) -> Result<()> {
        let status = self.get_status(id)?;
        let active: i64 = self.conn.query_row(
            "SELECT count(*) FROM tasks WHERE status_id = ?1 AND archived_at IS NULL",
            [id],
            |r| r.get(0),
        )?;
        if active > 0 {
            return Err(Error::Invalid(format!(
                "column «{}» still has {active} task(s)",
                status.name
            )));
        }
        let fallback = self
            .list_statuses(status.project_id)?
            .into_iter()
            .find(|s| s.id != id)
            .ok_or_else(|| Error::Invalid("a project needs at least one column".into()))?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE tasks SET status_id = ?1 WHERE status_id = ?2",
            params![fallback.id, id],
        )?;
        tx.execute("DELETE FROM statuses WHERE id = ?1", [id])?;
        tx.commit()?;
        self.touch_project(status.project_id)?;
        self.log(
            Some(status.project_id),
            None,
            "status.delete",
            &status.name,
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    fn project(s: &mut Store) -> i64 {
        s.create_project(NewProject {
            name: "P".into(),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    fn names(s: &Store, p: i64) -> Vec<String> {
        s.list_statuses(p)
            .unwrap()
            .into_iter()
            .map(|x| x.name)
            .collect()
    }

    #[test]
    fn create_appends_column_and_validates_input() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let review = s
            .create_status(p, " Review ", Some("#FF00AA"), false)
            .unwrap();
        assert_eq!(
            (review.name.as_str(), review.color.as_str(), review.position),
            ("Review", "#ff00aa", 3)
        );
        assert!(matches!(
            s.create_status(p, "  ", None, false),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.create_status(p, "X", Some("red"), false),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.create_status(999, "X", None, false),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn update_renames_and_toggles_done() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let first = s.list_statuses(p).unwrap()[0].id;
        let u = s
            .update_status(
                first,
                StatusPatch {
                    name: Some("Backlog".into()),
                    is_done: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            (u.name.as_str(), u.is_done, u.color.as_str()),
            ("Backlog", true, "#8b8f98")
        );
    }

    #[test]
    fn move_reorders_columns() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let done = s.find_status(p, "Done").unwrap().id;
        s.move_status(done, 0).unwrap();
        assert_eq!(names(&s, p), ["Done", "To do", "In progress"]);
        s.move_status(done, 99).unwrap();
        assert_eq!(names(&s, p), ["To do", "In progress", "Done"]);
    }

    #[test]
    fn delete_refuses_non_empty_and_last_column() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let cols = s.list_statuses(p).unwrap();
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "T".into(),
                status_id: Some(cols[1].id),
                ..Default::default()
            })
            .unwrap();
        assert!(matches!(
            s.delete_status(cols[1].id),
            Err(Error::Invalid(_))
        ));

        // Archived tasks move along so they stay restorable.
        s.set_task_archived(t.id, true).unwrap();
        s.delete_status(cols[1].id).unwrap();
        assert_eq!(s.get_task(t.id).unwrap().status_id, cols[0].id);

        s.delete_status(cols[2].id).unwrap();
        assert!(matches!(
            s.delete_status(cols[0].id),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn columns_named_like_done_count_as_done() {
        let (mut s, _d) = store();
        let p = s
            .create_project(crate::models::NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        assert!(s.create_status(p, "Resolved", None, false).unwrap().is_done);
        assert!(s.create_status(p, " løst ", None, false).unwrap().is_done);
        assert!(!s.create_status(p, "Review", None, false).unwrap().is_done);
    }
}
