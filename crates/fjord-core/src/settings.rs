//! App settings, the user's name, and permanent deletion.
//!
//! Deletion is for people only: the CLI asks for confirmation and the MCP
//! server deliberately has no delete tools, so agents can only archive.

use std::collections::HashSet;
use std::fs;

use rusqlite::{OptionalExtension, params};

use crate::error::Result;
use crate::store::{Store, require_text};

const USER_NAME_KEY: &str = "user_name";

impl Store {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    /// The name chosen in Settings, if any.
    pub fn user_name(&self) -> Result<Option<String>> {
        self.get_setting(USER_NAME_KEY)
    }

    /// Changes who this store acts as for future changes.
    pub fn set_actor(&mut self, actor: &str) -> Result<()> {
        self.actor = require_text(actor, "user name")?;
        Ok(())
    }

    /// Saves a new user name and makes it the actor. With `rewrite_history`,
    /// past activity, tasks and files recorded under the old name follow along.
    pub fn rename_user(&mut self, new_name: &str, rewrite_history: bool) -> Result<String> {
        let new_name = require_text(new_name, "user name")?;
        let old = self.actor.clone();
        if rewrite_history && old != new_name {
            let tx = self.conn.transaction()?;
            tx.execute(
                "UPDATE activity SET actor = ?1 WHERE actor = ?2",
                params![new_name, old],
            )?;
            tx.execute(
                "UPDATE tasks SET created_by = ?1 WHERE created_by = ?2",
                params![new_name, old],
            )?;
            tx.execute(
                "UPDATE attachments SET added_by = ?1 WHERE added_by = ?2",
                params![new_name, old],
            )?;
            tx.commit()?;
        }
        self.set_setting(USER_NAME_KEY, &new_name)?;
        self.actor = new_name.clone();
        Ok(new_name)
    }

    /// Permanently deletes a task, its attachments and its links.
    pub fn delete_task(&mut self, id: i64) -> Result<()> {
        let task = self.get_task(id)?;
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        self.touch_project(task.project_id)?;
        self.log(
            Some(task.project_id),
            None,
            "task.delete",
            &task.title,
            None,
        )?;
        self.remove_orphan_files()?;
        Ok(())
    }

    /// Permanently deletes a project with all its tasks, columns, notes and files.
    pub fn delete_project(&mut self, id: i64) -> Result<()> {
        let project = self.get_project(id)?;
        let tx = self.conn.transaction()?;
        // Tasks reference statuses without ON DELETE CASCADE, so remove them first.
        tx.execute("DELETE FROM tasks WHERE project_id = ?1", [id])?;
        tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
        tx.commit()?;
        self.log(None, None, "project.delete", &project.name, None)?;
        self.remove_orphan_files()?;
        Ok(())
    }

    pub fn delete_note(&mut self, id: i64) -> Result<()> {
        let note = self.get_note(id)?;
        self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        if let Some(p) = note.project_id {
            self.touch_project(p)?;
        }
        self.log(note.project_id, None, "note.delete", &note.title, None)?;
        Ok(())
    }

    /// Deletes stored file contents that no attachment points to any more.
    /// Returns how many blobs were removed.
    pub fn remove_orphan_files(&mut self) -> Result<usize> {
        let orphans: Vec<String> = {
            let mut stmt = self.conn.prepare(
                "SELECT sha256 FROM files WHERE id NOT IN (SELECT file_id FROM attachments)",
            )?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let orphan_set: HashSet<&str> = orphans.iter().map(String::as_str).collect();
        for sha in &orphan_set {
            let path = self.files_dir.join(&sha[..2]).join(sha);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.conn.execute(
            "DELETE FROM files WHERE id NOT IN (SELECT file_id FROM attachments)",
            [],
        )?;
        Ok(orphan_set.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::Error;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    #[test]
    fn settings_round_trip_and_user_rename() {
        let (mut s, _dir) = store();
        assert_eq!(s.user_name().unwrap(), None);
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        s.create_task(NewTask {
            project_id: p,
            title: "T".into(),
            ..Default::default()
        })
        .unwrap();

        // Without history rewrite, old entries keep the old name.
        s.rename_user("Jonas", false).unwrap();
        assert_eq!(s.user_name().unwrap().as_deref(), Some("Jonas"));
        assert_eq!(s.actor(), "Jonas");
        assert_eq!(s.recent_activity(Some(p), 10).unwrap()[0].actor, "tester");

        // With rewrite, they follow along.
        s.rename_user("Jonas H", true).unwrap();
        s.set_actor("tester").unwrap();
        s.rename_user("Jonas H", true).unwrap();
        assert!(
            s.recent_activity(Some(p), 10)
                .unwrap()
                .iter()
                .all(|a| a.actor == "Jonas H")
        );
        assert_eq!(s.list_tasks(p).unwrap()[0].created_by, "Jonas H");
        assert!(matches!(s.rename_user("  ", false), Err(Error::Invalid(_))));
    }

    #[test]
    fn delete_task_removes_it_and_unreferenced_files() {
        let (mut s, dir) = store();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let a = s
            .create_task(NewTask {
                project_id: p,
                title: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = s
            .create_task(NewTask {
                project_id: p,
                title: "B".into(),
                ..Default::default()
            })
            .unwrap();
        let file = dir.path().join("x.txt");
        std::fs::write(&file, "shared").unwrap();
        let on_a = s.attach_file(p, Some(a.id), &file).unwrap();
        s.attach_file(p, Some(b.id), &file).unwrap();
        let blob = s.attachment_path(on_a.id).unwrap();

        s.delete_task(a.id).unwrap();
        assert!(matches!(s.get_task(a.id), Err(Error::NotFound(_))));
        assert!(blob.exists(), "still used by task B");

        s.delete_task(b.id).unwrap();
        assert!(!blob.exists(), "no longer referenced");
        assert_eq!(
            s.recent_activity(Some(p), 1).unwrap()[0].action,
            "task.delete"
        );
    }

    #[test]
    fn delete_project_removes_everything_inside() {
        let (mut s, dir) = store();
        let p = s
            .create_project(NewProject {
                name: "Gone".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let keep = s
            .create_project(NewProject {
                name: "Keep".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "T".into(),
                ..Default::default()
            })
            .unwrap();
        s.add_note(p, "N", "").unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "f").unwrap();
        let att = s.attach_file(p, Some(t.id), &file).unwrap();
        let blob = s.attachment_path(att.id).unwrap();

        s.delete_project(p).unwrap();
        assert!(matches!(s.get_project(p), Err(Error::NotFound(_))));
        assert!(s.list_tasks(p).unwrap().is_empty());
        assert!(s.list_notes(p).unwrap().is_empty());
        assert!(!blob.exists());
        assert_eq!(s.list_projects(true).unwrap().len(), 1);
        assert!(s.get_project(keep).is_ok());
        assert_eq!(
            s.recent_activity(None, 1).unwrap()[0].action,
            "project.delete"
        );
    }

    #[test]
    fn delete_note() {
        let (mut s, _dir) = store();
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let n = s.add_note(p, "N", "body").unwrap();
        s.delete_note(n.id).unwrap();
        assert!(s.list_notes(p).unwrap().is_empty());
        assert!(matches!(s.delete_note(n.id), Err(Error::NotFound(_))));
    }
}
