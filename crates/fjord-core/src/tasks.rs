use rusqlite::{OptionalExtension, Row, params};

use crate::error::{Error, Result};
use crate::models::{NewTask, Task, TaskPatch};
use crate::store::{Store, require_text};

const TASK_COLS: &str = "id, project_id, status_id, title, body_md, priority, due_at, position,
    created_by, created_at, updated_at, archived_at, branch, parent_id, child_order";

fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        status_id: row.get(2)?,
        title: row.get(3)?,
        body_md: row.get(4)?,
        priority: row.get(5)?,
        due_at: row.get(6)?,
        position: row.get(7)?,
        created_by: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        archived_at: row.get(11)?,
        branch: row.get(12)?,
        parent_id: row.get(13)?,
        child_order: row.get(14)?,
    })
}

fn validate_priority(priority: i64) -> Result<i64> {
    if (0..=3).contains(&priority) {
        Ok(priority)
    } else {
        Err(Error::Invalid("priority must be 0–3".into()))
    }
}

/// Accepts YYYY-MM-DD only, so date comparisons in SQL stay correct.
fn validate_due(due: Option<String>) -> Result<Option<String>> {
    let Some(d) = due.map(|d| d.trim().to_string()).filter(|d| !d.is_empty()) else {
        return Ok(None);
    };
    let parts: Vec<&str> = d.split('-').collect();
    let number = |i: usize| {
        parts
            .get(i)
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let valid = parts.len() == 3
        && [4, 2, 2]
            .iter()
            .zip(&parts)
            .all(|(len, p)| p.len() == *len && p.chars().all(|c| c.is_ascii_digit()))
        && (1..=12).contains(&number(1))
        && (1..=31).contains(&number(2));
    if valid {
        Ok(Some(d))
    } else {
        Err(Error::Invalid(format!("due date «{d}» must be YYYY-MM-DD")))
    }
}

impl Store {
    pub fn create_task(&mut self, new: NewTask) -> Result<Task> {
        let title = require_text(&new.title, "task title")?;
        let priority = validate_priority(new.priority)?;
        let due = validate_due(new.due_at)?;
        let project = self.get_project(new.project_id)?;
        let status_id = match new.status_id {
            Some(id) => self.status_in_project(project.id, id)?,
            None => self
                .list_statuses(project.id)?
                .first()
                .map(|s| s.id)
                .ok_or_else(|| Error::Invalid("project has no statuses".into()))?,
        };
        let child_order = match new.parent_id {
            Some(parent_id) => {
                let parent = self.get_task(parent_id)?;
                if parent.project_id != project.id {
                    return Err(Error::Invalid(
                        "a subtask must be in its parent's project".into(),
                    ));
                }
                if parent.parent_id.is_some() {
                    return Err(Error::Invalid(
                        "subtasks can't have subtasks of their own".into(),
                    ));
                }
                self.conn.query_row(
                    "SELECT coalesce(max(child_order), 0) + 1 FROM tasks WHERE parent_id = ?1",
                    [parent_id],
                    |r| r.get::<_, f64>(0),
                )?
            }
            None => 0.0,
        };
        let position = self.next_position(status_id)?;
        self.conn.execute(
            "INSERT INTO tasks (project_id, status_id, title, body_md, priority, due_at, position, created_by,
                                parent_id, child_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project.id,
                status_id,
                title,
                new.body_md,
                priority,
                due,
                position,
                self.actor,
                new.parent_id,
                child_order
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.touch_project(project.id)?;
        self.log(Some(project.id), Some(id), "task.create", &title, None)?;
        self.get_task(id)
    }

    pub fn get_task(&self, id: i64) -> Result<Task> {
        self.conn
            .query_row(
                &format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"),
                [id],
                task_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("task {id}")))
    }

    /// Active tasks of a project, ordered by status column then position.
    pub fn list_tasks(&self, project_id: i64) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE project_id = ?1 AND archived_at IS NULL
             ORDER BY status_id, position"
        ))?;
        let rows = stmt.query_map([project_id], task_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// A task's subtasks (archived ones excluded), in their own order.
    pub fn list_subtasks(&self, parent_id: i64) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE parent_id = ?1 AND archived_at IS NULL
             ORDER BY child_order, id"
        ))?;
        let rows = stmt.query_map([parent_id], task_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Makes a task a subtask of `parent` (or a top-level task with `None`).
    /// Same rules as creating one: same project, one level deep.
    pub fn set_task_parent(&mut self, id: i64, parent: Option<i64>) -> Result<Task> {
        let task = self.get_task(id)?;
        let child_order = match parent {
            Some(parent_id) => {
                let p = self.get_task(parent_id)?;
                if p.id == task.id || p.project_id != task.project_id {
                    return Err(Error::Invalid(
                        "a subtask must be in its parent's project".into(),
                    ));
                }
                if p.parent_id.is_some() {
                    return Err(Error::Invalid(
                        "subtasks can't have subtasks of their own".into(),
                    ));
                }
                if !self.list_subtasks(task.id)?.is_empty() {
                    return Err(Error::Invalid(
                        "a task with subtasks can't become a subtask".into(),
                    ));
                }
                self.conn.query_row(
                    "SELECT coalesce(max(child_order), 0) + 1 FROM tasks WHERE parent_id = ?1",
                    [parent_id],
                    |r| r.get::<_, f64>(0),
                )?
            }
            None => 0.0,
        };
        self.conn.execute(
            "UPDATE tasks SET parent_id = ?1, child_order = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?3",
            params![parent, child_order, id],
        )?;
        self.touch_project(task.project_id)?;
        self.get_task(id)
    }

    /// Moves a subtask to `index` among its siblings.
    pub fn move_subtask(&mut self, id: i64, index: usize) -> Result<Vec<Task>> {
        let task = self.get_task(id)?;
        let parent_id = task
            .parent_id
            .ok_or_else(|| Error::Invalid(format!("task {id} is not a subtask")))?;
        let mut siblings: Vec<i64> = self
            .list_subtasks(parent_id)?
            .into_iter()
            .map(|t| t.id)
            .filter(|&s| s != id)
            .collect();
        siblings.insert(index.min(siblings.len()), id);
        for (i, sid) in siblings.iter().enumerate() {
            self.conn.execute(
                "UPDATE tasks SET child_order = ?1 WHERE id = ?2",
                params![(i + 1) as f64, sid],
            )?;
        }
        self.touch_project(task.project_id)?;
        self.list_subtasks(parent_id)
    }

    /// Archived tasks of a project, most recently archived first.
    pub fn list_archived_tasks(&self, project_id: i64) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE project_id = ?1 AND archived_at IS NOT NULL
             ORDER BY archived_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map([project_id], task_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Open tasks due on or before `until` (YYYY-MM-DD) in active projects,
    /// overdue ones included. Earliest due date first, then highest priority.
    pub fn list_due_tasks(&self, until: &str) -> Result<Vec<Task>> {
        let until = validate_due(Some(until.to_string()))?
            .ok_or_else(|| Error::Invalid("a date is required".into()))?;
        let cols = TASK_COLS
            .split(',')
            .map(|c| format!("t.{}", c.trim()))
            .collect::<Vec<_>>()
            .join(", ");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {cols} FROM tasks t
             JOIN statuses s ON s.id = t.status_id
             JOIN projects p ON p.id = t.project_id
             WHERE t.archived_at IS NULL AND p.archived_at IS NULL AND NOT s.is_done
               AND t.due_at IS NOT NULL AND t.due_at <= ?1
             ORDER BY t.due_at, t.priority DESC, t.id"
        ))?;
        let rows = stmt.query_map([until], task_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_task(&mut self, id: i64, patch: TaskPatch) -> Result<Task> {
        let current = self.get_task(id)?;
        let title = match &patch.title {
            Some(t) => require_text(t, "task title")?,
            None => current.title.clone(),
        };
        let priority = validate_priority(patch.priority.unwrap_or(current.priority))?;
        let due = match patch.due_at {
            Some(d) => validate_due(d)?,
            None => current.due_at.clone(),
        };
        self.conn.execute(
            "UPDATE tasks SET title = ?1, body_md = ?2, priority = ?3, due_at = ?4,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?5",
            params![
                title,
                patch.body_md.unwrap_or(current.body_md),
                priority,
                due,
                id
            ],
        )?;
        self.touch_project(current.project_id)?;
        self.log(
            Some(current.project_id),
            Some(id),
            "task.update",
            &title,
            None,
        )?;
        self.get_task(id)
    }

    /// Moves a task to `status_id`, placed before `before_task_id` or last.
    pub fn move_task(
        &mut self,
        id: i64,
        status_id: i64,
        before_task_id: Option<i64>,
    ) -> Result<Task> {
        let task = self.get_task(id)?;
        let status_id = self.status_in_project(task.project_id, status_id)?;
        let position = match before_task_id {
            Some(before) => self.position_before(status_id, before)?,
            None => self.next_position(status_id)?,
        };
        self.conn.execute(
            "UPDATE tasks SET status_id = ?1, position = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?3",
            params![status_id, position, id],
        )?;
        let status: String = self.conn.query_row(
            "SELECT name FROM statuses WHERE id = ?1",
            [status_id],
            |r| r.get(0),
        )?;
        self.touch_project(task.project_id)?;
        self.log(
            Some(task.project_id),
            Some(id),
            "task.move",
            &task.title,
            Some(&status),
        )?;
        self.get_task(id)
    }

    pub fn set_task_archived(&mut self, id: i64, archived: bool) -> Result<Task> {
        let task = self.get_task(id)?;
        self.conn.execute(
            "UPDATE tasks SET archived_at = CASE WHEN ?1 THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') END,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?2 OR parent_id = ?2",
            params![archived, id],
        )?;
        let action = if archived {
            "task.archive"
        } else {
            "task.restore"
        };
        self.touch_project(task.project_id)?;
        self.log(Some(task.project_id), Some(id), action, &task.title, None)?;
        self.get_task(id)
    }

    fn status_in_project(&self, project_id: i64, status_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT id FROM statuses WHERE id = ?1 AND project_id = ?2",
                [status_id, project_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("status {status_id} in project {project_id}")))
    }

    fn next_position(&self, status_id: i64) -> Result<f64> {
        Ok(self.conn.query_row(
            "SELECT coalesce(max(position), 0) + 1 FROM tasks WHERE status_id = ?1 AND archived_at IS NULL",
            [status_id],
            |r| r.get(0),
        )?)
    }

    /// Midpoint between `before` and the task above it — no renumbering needed.
    fn position_before(&self, status_id: i64, before: i64) -> Result<f64> {
        let anchor = self.get_task(before)?;
        if anchor.status_id != status_id {
            return Err(Error::Invalid("anchor task is in another column".into()));
        }
        let above: f64 = self.conn.query_row(
            "SELECT coalesce(max(position), ?2 - 1) FROM tasks
             WHERE status_id = ?1 AND position < ?2 AND archived_at IS NULL",
            params![status_id, anchor.position],
            |r| r.get(0),
        )?;
        Ok((above + anchor.position) / 2.0)
    }

    pub(crate) fn touch_project(&self, project_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            [project_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewProject;
    use crate::test_util::store;

    fn project(s: &mut Store) -> i64 {
        s.create_project(NewProject {
            name: "P".into(),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    fn task(s: &mut Store, project_id: i64, title: &str) -> Task {
        s.create_task(NewTask {
            project_id,
            title: title.into(),
            ..Default::default()
        })
        .unwrap()
    }

    fn titles(s: &Store, project_id: i64) -> Vec<String> {
        s.list_tasks(project_id)
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect()
    }

    #[test]
    fn new_task_lands_last_in_first_column_with_actor() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let a = task(&mut s, p, "A");
        let b = task(&mut s, p, "B");
        let first = s.list_statuses(p).unwrap()[0].id;
        assert_eq!((a.status_id, b.status_id), (first, first));
        assert!(b.position > a.position);
        assert_eq!(a.created_by, "tester");
    }

    #[test]
    fn invalid_input_is_rejected() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let new = |title: &str, priority: i64, due: Option<&str>| NewTask {
            project_id: p,
            title: title.into(),
            priority,
            due_at: due.map(Into::into),
            ..Default::default()
        };
        assert!(matches!(
            s.create_task(new("", 0, None)),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.create_task(new("x", 7, None)),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.create_task(new("x", 0, Some("31.12.2026"))),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.create_task(new("x", 0, Some("2026-13-01"))),
            Err(Error::Invalid(_))
        ));
        assert!(s.create_task(new("x", 3, Some("2026-12-31"))).is_ok());
    }

    #[test]
    fn task_in_unknown_project_is_not_found() {
        let (mut s, _dir) = store();
        let err = s.create_task(NewTask {
            project_id: 999,
            title: "x".into(),
            ..Default::default()
        });
        assert!(matches!(err, Err(Error::NotFound(_))));
    }

    #[test]
    fn move_task_to_done_counts_as_done() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let t = task(&mut s, p, "Ship it");
        let done = s.find_status(p, "Done").unwrap();
        let moved = s.move_task(t.id, done.id, None).unwrap();
        assert_eq!(moved.status_id, done.id);
        let summary = &s.list_projects(false).unwrap()[0];
        assert_eq!((summary.task_count, summary.done_count), (1, 1));
    }

    #[test]
    fn move_before_places_task_between_neighbours() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let a = task(&mut s, p, "A");
        let b = task(&mut s, p, "B");
        let c = task(&mut s, p, "C");
        s.move_task(c.id, c.status_id, Some(b.id)).unwrap();
        assert_eq!(titles(&s, p), ["A", "C", "B"]);
        s.move_task(b.id, b.status_id, Some(a.id)).unwrap();
        assert_eq!(titles(&s, p), ["B", "A", "C"]);
    }

    #[test]
    fn cannot_move_to_status_of_other_project() {
        let (mut s, _dir) = store();
        let p1 = project(&mut s);
        let p2 = project(&mut s);
        let t = task(&mut s, p1, "A");
        let foreign = s.list_statuses(p2).unwrap()[0].id;
        assert!(matches!(
            s.move_task(t.id, foreign, None),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn update_can_keep_or_clear_due_date() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "A".into(),
                due_at: Some("2026-01-01".into()),
                ..Default::default()
            })
            .unwrap();
        let kept = s
            .update_task(
                t.id,
                TaskPatch {
                    priority: Some(2),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(kept.due_at.as_deref(), Some("2026-01-01"));
        let cleared = s
            .update_task(
                t.id,
                TaskPatch {
                    due_at: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(cleared.due_at, None);
    }

    #[test]
    fn overdue_open_tasks_are_counted() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        s.create_task(NewTask {
            project_id: p,
            title: "late".into(),
            due_at: Some("2000-01-01".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(s.list_projects(false).unwrap()[0].overdue_count, 1);
    }

    #[test]
    fn due_tasks_skip_done_archived_and_later_ones() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let due = |s: &mut Store, title: &str, date: &str, priority: i64| {
            s.create_task(NewTask {
                project_id: p,
                title: title.into(),
                due_at: Some(date.into()),
                priority,
                ..Default::default()
            })
            .unwrap()
        };
        due(&mut s, "late", "2026-01-01", 0);
        due(&mut s, "low", "2026-01-05", 1);
        due(&mut s, "high", "2026-01-05", 3);
        due(&mut s, "next week", "2026-01-12", 0);
        let finished = due(&mut s, "finished", "2026-01-03", 0);
        let done = s
            .list_statuses(p)
            .unwrap()
            .into_iter()
            .find(|st| st.is_done)
            .unwrap();
        s.move_task(finished.id, done.id, None).unwrap();
        let shelved = due(&mut s, "shelved", "2026-01-03", 0);
        s.set_task_archived(shelved.id, true).unwrap();
        let titles: Vec<String> = s
            .list_due_tasks("2026-01-11")
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles, ["late", "high", "low"]);
        assert!(s.list_due_tasks("soon").is_err());
    }

    #[test]
    fn archived_tasks_leave_the_board() {
        let (mut s, _dir) = store();
        let p = project(&mut s);
        let t = task(&mut s, p, "A");
        s.set_task_archived(t.id, true).unwrap();
        assert!(s.list_tasks(p).unwrap().is_empty());
        assert_eq!(s.list_archived_tasks(p).unwrap()[0].id, t.id);
        s.set_task_archived(t.id, false).unwrap();
        assert_eq!(s.list_tasks(p).unwrap().len(), 1);
        assert!(s.list_archived_tasks(p).unwrap().is_empty());
    }

    #[test]
    fn subtasks_are_one_level_ordered_and_follow_their_parent() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path(), "tester").unwrap();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let other = s
            .create_project(NewProject {
                name: "Q".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let parent = s
            .create_task(NewTask {
                project_id: p,
                title: "Parent".into(),
                ..Default::default()
            })
            .unwrap();
        let sub = |s: &mut Store, title: &str| {
            s.create_task(NewTask {
                project_id: p,
                title: title.into(),
                parent_id: Some(parent.id),
                ..Default::default()
            })
            .unwrap()
        };
        let a = sub(&mut s, "A");
        let b = sub(&mut s, "B");
        let c = sub(&mut s, "C");
        assert_eq!(a.parent_id, Some(parent.id));
        let titles = |s: &Store| {
            s.list_subtasks(parent.id)
                .unwrap()
                .into_iter()
                .map(|t| t.title)
                .collect::<Vec<_>>()
        };
        assert_eq!(titles(&s), ["A", "B", "C"]);

        s.move_subtask(c.id, 0).unwrap();
        assert_eq!(titles(&s), ["C", "A", "B"]);
        assert!(
            s.move_subtask(parent.id, 0).is_err(),
            "only subtasks can be reordered"
        );

        // One level only, and only within the same project.
        assert!(
            s.create_task(NewTask {
                project_id: p,
                title: "X".into(),
                parent_id: Some(a.id),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            s.create_task(NewTask {
                project_id: other,
                title: "Y".into(),
                parent_id: Some(parent.id),
                ..Default::default()
            })
            .is_err()
        );

        // Archiving the parent archives its subtasks; restoring brings them back.
        s.set_task_archived(parent.id, true).unwrap();
        assert!(s.list_subtasks(parent.id).unwrap().is_empty());
        assert!(s.get_task(b.id).unwrap().archived_at.is_some());
        s.set_task_archived(parent.id, false).unwrap();
        assert_eq!(titles(&s).len(), 3);

        // Deleting the parent deletes its subtasks.
        s.delete_task(parent.id).unwrap();
        assert!(s.get_task(a.id).is_err());
    }

    #[test]
    fn tasks_can_be_moved_under_a_parent_and_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path(), "tester").unwrap();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let q = s
            .create_project(NewProject {
                name: "Q".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let story = s
            .create_task(NewTask {
                project_id: p,
                title: "Story".into(),
                ..Default::default()
            })
            .unwrap();
        let task = s
            .create_task(NewTask {
                project_id: p,
                title: "Task".into(),
                ..Default::default()
            })
            .unwrap();
        let elsewhere = s
            .create_task(NewTask {
                project_id: q,
                title: "Other".into(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(
            s.set_task_parent(task.id, Some(story.id))
                .unwrap()
                .parent_id,
            Some(story.id)
        );
        assert_eq!(s.list_subtasks(story.id).unwrap().len(), 1);
        assert!(
            s.set_task_parent(story.id, Some(task.id)).is_err(),
            "a task with subtasks can't become one"
        );
        assert!(
            s.set_task_parent(elsewhere.id, Some(story.id)).is_err(),
            "other project"
        );
        assert!(
            s.set_task_parent(story.id, Some(story.id)).is_err(),
            "itself"
        );
        assert_eq!(s.set_task_parent(task.id, None).unwrap().parent_id, None);
    }
}
