//! Moving a task to another project. Its subtasks, files and story analysis go
//! with it; each lands in the column with the same name, or the first fitting one.

use rusqlite::params;

use crate::error::{Error, Result};
use crate::models::{Status, Task};
use crate::store::Store;

/// The column in `targets` that best matches `from`: same name, else the first
/// done column for done tasks, else the first column.
fn matching_status(targets: &[Status], from: Option<&Status>) -> Option<i64> {
    let same_name = from.and_then(|f| {
        targets
            .iter()
            .find(|t| t.name.trim().eq_ignore_ascii_case(f.name.trim()))
    });
    let done = from
        .filter(|f| f.is_done)
        .and_then(|_| targets.iter().find(|t| t.is_done));
    same_name.or(done).or(targets.first()).map(|s| s.id)
}

impl Store {
    /// Moves a task to `project_id`, into `status_id` if given (a column there),
    /// else the matching column. A subtask becomes a top-level task.
    pub fn move_task_to_project(
        &mut self,
        id: i64,
        project_id: i64,
        status_id: Option<i64>,
    ) -> Result<Task> {
        let task = self.get_task(id)?;
        if task.project_id == project_id {
            return Err(Error::Invalid("the task is already in that project".into()));
        }
        let from = self.get_project(task.project_id)?;
        let to = self.get_project(project_id)?;
        if to.archived_at.is_some() {
            return Err(Error::Invalid(format!("«{}» is archived", to.name)));
        }
        let old_statuses = self.list_statuses(task.project_id)?;
        let targets = self.list_statuses(project_id)?;
        let no_columns = || Error::Invalid(format!("«{}» has no columns", to.name));
        let status_of = |t: &Task| old_statuses.iter().find(|s| s.id == t.status_id);
        let main_status = match status_id {
            Some(sid) if targets.iter().any(|s| s.id == sid) => sid,
            Some(sid) => {
                return Err(Error::NotFound(format!(
                    "status {sid} in project {project_id}"
                )));
            }
            None => matching_status(&targets, status_of(&task)).ok_or_else(no_columns)?,
        };
        let mut moves = vec![(task.id, main_status)];
        for sub in self.list_all_subtasks(task.id)? {
            let sid = matching_status(&targets, status_of(&sub)).ok_or_else(no_columns)?;
            moves.push((sub.id, sid));
        }
        let analysis = self.story_analysis(task.id)?;

        let tx = self.conn.transaction()?;
        for (tid, sid) in &moves {
            tx.execute(
                "UPDATE tasks SET project_id = ?1, status_id = ?2,
                    position = (SELECT coalesce(max(position), 0) + 1 FROM tasks
                                WHERE status_id = ?2 AND archived_at IS NULL),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?3",
                params![project_id, sid, tid],
            )?;
            tx.execute(
                "UPDATE search_fts SET project_id = ?1 WHERE kind = 'task' AND ref_id = ?2",
                params![project_id, tid],
            )?;
            tx.execute(
                "UPDATE search_fts SET project_id = ?1 WHERE kind = 'file'
                    AND ref_id IN (SELECT id FROM attachments WHERE task_id = ?2)",
                params![project_id, tid],
            )?;
            tx.execute(
                "UPDATE attachments SET project_id = ?1 WHERE task_id = ?2",
                params![project_id, tid],
            )?;
        }
        tx.execute(
            "UPDATE tasks SET parent_id = NULL, child_order = 0 WHERE id = ?1",
            [task.id],
        )?;
        if let Some(note) = &analysis {
            tx.execute(
                "UPDATE notes SET project_id = ?1 WHERE id = ?2",
                params![project_id, note.id],
            )?;
        }
        tx.commit()?;

        self.touch_project(from.id)?;
        self.touch_project(project_id)?;
        self.log(
            Some(project_id),
            Some(id),
            "task.transfer",
            &task.title,
            Some(&from.name),
        )?;
        self.get_task(id)
    }

    /// Subtasks of `parent_id`, archived ones included.
    fn list_all_subtasks(&self, parent_id: i64) -> Result<Vec<Task>> {
        let ids: Vec<i64> = self
            .conn
            .prepare("SELECT id FROM tasks WHERE parent_id = ?1")?
            .query_map([parent_id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        ids.into_iter().map(|i| self.get_task(i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{NewProject, NewTask, StatusPatch};
    use crate::test_util::store;
    use crate::{Error, Store};

    fn project(s: &mut Store, name: &str) -> i64 {
        s.create_project(NewProject {
            name: name.into(),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    fn task(s: &mut Store, project_id: i64, title: &str, parent_id: Option<i64>) -> i64 {
        s.create_task(NewTask {
            project_id,
            title: title.into(),
            parent_id,
            ..Default::default()
        })
        .unwrap()
        .id
    }

    #[test]
    fn task_moves_with_subtasks_into_matching_columns() {
        let (mut s, _dir) = store();
        let a = project(&mut s, "A");
        let b = project(&mut s, "B");
        let parent = task(&mut s, a, "Parent", None);
        let sub = task(&mut s, a, "Sub", Some(parent));
        let a_cols = s.list_statuses(a).unwrap();
        s.move_task(parent, a_cols[1].id, None).unwrap();
        s.move_task(sub, a_cols.last().unwrap().id, None).unwrap();

        let moved = s.move_task_to_project(parent, b, None).unwrap();

        let b_cols = s.list_statuses(b).unwrap();
        assert_eq!(moved.project_id, b);
        assert_eq!(moved.status_id, b_cols[1].id);
        let sub = s.get_task(sub).unwrap();
        assert_eq!((sub.project_id, sub.parent_id), (b, Some(parent)));
        assert_eq!(sub.status_id, b_cols.last().unwrap().id);
        assert!(s.list_tasks(a).unwrap().is_empty());
        let hit = s
            .search("Parent")
            .unwrap()
            .into_iter()
            .find(|h| h.kind == "task")
            .unwrap();
        assert_eq!(hit.project_id, Some(b));
    }

    #[test]
    fn unknown_column_names_fall_back_to_done_or_first() {
        let (mut s, _dir) = store();
        let a = project(&mut s, "A");
        let b = project(&mut s, "B");
        let open = task(&mut s, a, "Open", None);
        let done = task(&mut s, a, "Done", None);
        let a_cols = s.list_statuses(a).unwrap();
        let a_done = a_cols.iter().find(|c| c.is_done).unwrap().id;
        s.move_task(done, a_done, None).unwrap();
        s.move_task(open, a_cols[1].id, None).unwrap();
        for c in &a_cols {
            let patch = StatusPatch {
                name: Some(format!("Old {}", c.name)),
                ..Default::default()
            };
            s.update_status(c.id, patch).unwrap();
        }

        let b_cols = s.list_statuses(b).unwrap();
        let b_done = b_cols.iter().find(|c| c.is_done).unwrap().id;
        let open = s.move_task_to_project(open, b, None).unwrap();
        let done = s.move_task_to_project(done, b, None).unwrap();
        assert_eq!((open.status_id, done.status_id), (b_cols[0].id, b_done));
    }

    #[test]
    fn subtask_becomes_top_level_in_the_new_project() {
        let (mut s, _dir) = store();
        let a = project(&mut s, "A");
        let b = project(&mut s, "B");
        let parent = task(&mut s, a, "Parent", None);
        let sub = task(&mut s, a, "Sub", Some(parent));
        let column = s.list_statuses(b).unwrap()[2].id;

        let moved = s.move_task_to_project(sub, b, Some(column)).unwrap();

        assert_eq!((moved.project_id, moved.parent_id), (b, None));
        assert_eq!(moved.status_id, column);
        assert!(s.list_subtasks(parent).unwrap().is_empty());
    }

    #[test]
    fn invalid_targets_are_rejected() {
        let (mut s, _dir) = store();
        let a = project(&mut s, "A");
        let b = project(&mut s, "B");
        let t = task(&mut s, a, "T", None);
        let a_col = s.list_statuses(a).unwrap()[0].id;
        let invalid = |r| matches!(r, Err(Error::Invalid(_)));
        assert!(invalid(s.move_task_to_project(t, a, None)));
        assert!(matches!(
            s.move_task_to_project(t, b, Some(a_col)),
            Err(Error::NotFound(_))
        ));
        s.set_project_archived(b, true).unwrap();
        assert!(invalid(s.move_task_to_project(t, b, None)));
        assert_eq!(s.get_task(t).unwrap().project_id, a);
    }

    #[test]
    fn story_analysis_follows_the_task() {
        let (mut s, _dir) = store();
        let a = project(&mut s, "A");
        let b = project(&mut s, "B");
        let t = task(&mut s, a, "Story", None);
        let (note, _) = s.create_story_analysis(t).unwrap();

        s.move_task_to_project(t, b, None).unwrap();

        let found = s.story_analysis(t).unwrap().unwrap();
        assert_eq!((found.id, found.project_id), (note.id, Some(b)));
    }
}
