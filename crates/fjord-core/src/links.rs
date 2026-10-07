//! The notespace: free-floating notes, `[[wiki links]]` to projects, tasks
//! and notes, and backlinks ("mentioned in").
//!
//! Link syntax inside markdown:
//! - `[[#12]]`            → task 12
//! - `[[Bokost]]`         → project by name or slug
//! - `[[Fix push]]`       → task by title
//! - `[[Launch plan]]`    → note by title
//! - `[[target|label]]`   → same, shown as "label"
//!
//! Resolution order for plain text: project, then note, then task.

use rusqlite::{OptionalExtension, params};

use crate::error::{Error, Result};
use crate::models::{LinkTarget, Note};
use crate::notes::{NOTE_COLS, note_from_row};
use crate::store::Store;

const MAX_SUGGESTIONS: usize = 12;

/// The raw targets of all `[[…]]` links in `markdown`, in order, without duplicates.
/// A `|label` suffix is dropped; empty links are ignored.
pub fn parse_links(markdown: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let inner = &after[..end];
        // A newline means this wasn't a link after all.
        if !inner.contains('\n') {
            let target = inner.split('|').next().unwrap_or("").trim();
            if !target.is_empty() && !out.iter().any(|t| t == target) {
                out.push(target.to_string());
            }
        }
        rest = &after[end + 2..];
    }
    out
}

impl Store {
    /// What `[[text]]` points at right now, if anything.
    pub fn resolve_link(&self, text: &str) -> Result<Option<LinkTarget>> {
        let text = text.split('|').next().unwrap_or("").trim();
        if text.is_empty() {
            return Ok(None);
        }
        if let Some(id) = text.strip_prefix('#').and_then(|n| n.parse::<i64>().ok()) {
            return self.task_target(id);
        }
        let project: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT id, name FROM projects WHERE lower(name) = lower(?1) OR slug = lower(?1) LIMIT 1",
                [text],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((id, name)) = project {
            return Ok(Some(LinkTarget {
                kind: "project".into(),
                id,
                label: name,
                project_id: Some(id),
                done: false,
            }));
        }
        let note: Option<(i64, String, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT id, title, project_id FROM notes WHERE lower(title) = lower(?1) ORDER BY updated_at DESC LIMIT 1",
                [text],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((id, title, project_id)) = note {
            return Ok(Some(LinkTarget {
                kind: "note".into(),
                id,
                label: title,
                project_id,
                done: false,
            }));
        }
        let task: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM tasks WHERE lower(title) = lower(?1) AND archived_at IS NULL ORDER BY updated_at DESC LIMIT 1",
                [text],
                |r| r.get(0),
            )
            .optional()?;
        match task {
            Some(id) => self.task_target(id),
            None => Ok(None),
        }
    }

    fn task_target(&self, id: i64) -> Result<Option<LinkTarget>> {
        Ok(self
            .conn
            .query_row(
                "SELECT t.id, t.title, t.project_id, s.is_done FROM tasks t JOIN statuses s ON s.id = t.status_id WHERE t.id = ?1",
                [id],
                |r| {
                    Ok(LinkTarget {
                        kind: "task".into(),
                        id: r.get(0)?,
                        label: r.get(1)?,
                        project_id: Some(r.get(2)?),
                        done: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Re-parses a note's links and stores what they resolve to (for backlinks).
    pub(crate) fn reindex_links(&mut self, note_id: i64, body_md: &str) -> Result<()> {
        let targets: Vec<LinkTarget> = parse_links(body_md)
            .iter()
            .filter_map(|t| self.resolve_link(t).transpose())
            .collect::<Result<_>>()?;
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM note_links WHERE note_id = ?1", [note_id])?;
        for t in targets
            .iter()
            .filter(|t| !(t.kind == "note" && t.id == note_id))
        {
            tx.execute(
                "INSERT OR IGNORE INTO note_links (note_id, kind, target_id) VALUES (?1, ?2, ?3)",
                params![note_id, t.kind, t.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Notes that link to a project, task or note ("mentioned in").
    pub fn backlinks(&self, kind: &str, target_id: i64) -> Result<Vec<Note>> {
        if !matches!(kind, "project" | "task" | "note") {
            return Err(Error::Invalid(format!("unknown link kind «{kind}»")));
        }
        let cols = NOTE_COLS
            .split(", ")
            .map(|c| format!("n.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {cols} FROM notes n JOIN note_links l ON l.note_id = n.id
             WHERE l.kind = ?1 AND l.target_id = ?2 ORDER BY n.updated_at DESC"
        ))?;
        let rows = stmt.query_map(params![kind, target_id], note_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Autocomplete for `[[`: projects, notes and open tasks matching `query`.
    pub fn link_suggestions(&self, query: &str) -> Result<Vec<LinkTarget>> {
        let like = format!("%{}%", query.trim().replace(['%', '_'], ""));
        let mut out = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT 'project', id, name, id, 0 FROM projects WHERE archived_at IS NULL AND name LIKE ?1
             UNION ALL
             SELECT 'note', id, title, project_id, 0 FROM notes WHERE title LIKE ?1
             UNION ALL
             SELECT 'task', t.id, t.title, t.project_id, s.is_done FROM tasks t JOIN statuses s ON s.id = t.status_id
               WHERE t.archived_at IS NULL AND (t.title LIKE ?1 OR ('#' || t.id) LIKE ?1)
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![like, MAX_SUGGESTIONS as i64], |r| {
            Ok(LinkTarget {
                kind: r.get(0)?,
                id: r.get(1)?,
                label: r.get(2)?,
                project_id: r.get(3)?,
                done: r.get(4)?,
            })
        })?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Every note, pinned first: the global notespace plus project notes.
    /// `scope`: "all", "free" (no project) or a project id.
    pub fn list_all_notes(&self, free_only: bool) -> Result<Vec<Note>> {
        let filter = if free_only {
            "WHERE project_id IS NULL"
        } else {
            ""
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {NOTE_COLS} FROM notes {filter} ORDER BY pinned DESC, folder, updated_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map([], note_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Moves a note into a project, or out to the notespace with `None`.
    pub fn move_note(&mut self, id: i64, project_id: Option<i64>) -> Result<Note> {
        self.get_note(id)?;
        if let Some(p) = project_id {
            self.get_project(p)?;
        }
        self.conn.execute(
            "UPDATE notes SET project_id = ?1 WHERE id = ?2",
            params![project_id, id],
        )?;
        self.get_note(id)
    }

    pub fn set_note_folder(&mut self, id: i64, folder: &str) -> Result<Note> {
        self.get_note(id)?;
        let folder = folder.trim().trim_matches('/');
        self.conn.execute(
            "UPDATE notes SET folder = ?1 WHERE id = ?2",
            params![folder, id],
        )?;
        self.get_note(id)
    }

    pub fn set_note_pinned(&mut self, id: i64, pinned: bool) -> Result<Note> {
        self.get_note(id)?;
        self.conn.execute(
            "UPDATE notes SET pinned = ?1 WHERE id = ?2",
            params![pinned, id],
        )?;
        self.get_note(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    #[test]
    fn parses_wiki_links() {
        let md = "See [[Bokost]] and [[#12|the push bug]].\n[[ Launch plan ]] again [[Bokost]] [[]] [[broken\n]]";
        assert_eq!(parse_links(md), ["Bokost", "#12", "Launch plan"]);
        assert!(parse_links("no links [ [x] ]").is_empty());
        assert_eq!(parse_links("unterminated [[x"), Vec::<String>::new());
    }

    #[test]
    fn free_notes_live_outside_projects_and_can_move() {
        let (mut s, _dir) = store();
        let p = s
            .create_project(NewProject {
                name: "Bokost".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let free = s.create_note(None, "Ideas", "- widget").unwrap();
        assert_eq!(free.project_id, None);
        s.add_note(p, "Launch plan", "").unwrap();
        assert_eq!(s.list_all_notes(true).unwrap().len(), 1);
        assert_eq!(s.list_all_notes(false).unwrap().len(), 2);

        let moved = s.move_note(free.id, Some(p)).unwrap();
        assert_eq!(moved.project_id, Some(p));
        assert_eq!(s.move_note(free.id, None).unwrap().project_id, None);
        assert_eq!(
            s.set_note_folder(free.id, " /work/ideas/ ").unwrap().folder,
            "work/ideas"
        );
        assert!(s.set_note_pinned(free.id, true).unwrap().pinned);
        assert_eq!(
            s.list_all_notes(false).unwrap()[0].id,
            free.id,
            "pinned first"
        );
        assert!(matches!(
            s.move_note(free.id, Some(999)),
            Err(Error::NotFound(_))
        ));
        // Free notes are searchable too.
        assert_eq!(s.search("widget").unwrap()[0].project_id, None);
    }

    #[test]
    fn links_resolve_and_backlinks_follow_edits() {
        let (mut s, _dir) = store();
        let p = s
            .create_project(NewProject {
                name: "Bokost".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "Fix push".into(),
                ..Default::default()
            })
            .unwrap();
        let plan = s.create_note(None, "Launch plan", "").unwrap();

        assert_eq!(s.resolve_link("bokost").unwrap().unwrap().kind, "project");
        assert_eq!(
            s.resolve_link(&format!("#{}", t.id))
                .unwrap()
                .unwrap()
                .label,
            "Fix push"
        );
        assert_eq!(s.resolve_link("fix push").unwrap().unwrap().id, t.id);
        assert_eq!(
            s.resolve_link("Launch plan|the plan").unwrap().unwrap().id,
            plan.id
        );
        assert!(s.resolve_link("nothing here").unwrap().is_none());

        let n = s
            .create_note(
                None,
                "Weekly",
                "Work on [[Bokost]]: [[Fix push]], see [[Launch plan]] and [[Missing]]",
            )
            .unwrap();
        assert_eq!(s.backlinks("project", p).unwrap()[0].id, n.id);
        assert_eq!(s.backlinks("task", t.id).unwrap()[0].id, n.id);
        assert_eq!(s.backlinks("note", plan.id).unwrap()[0].id, n.id);

        s.update_note(n.id, "Weekly", "only [[Launch plan]] now")
            .unwrap();
        assert!(s.backlinks("task", t.id).unwrap().is_empty());
        assert_eq!(s.backlinks("note", plan.id).unwrap().len(), 1);

        s.delete_note(n.id).unwrap();
        assert!(s.backlinks("note", plan.id).unwrap().is_empty());
        assert!(matches!(s.backlinks("bogus", 1), Err(Error::Invalid(_))));
    }

    #[test]
    fn done_tasks_report_done_and_suggestions_cover_all_kinds() {
        let (mut s, _dir) = store();
        let p = s
            .create_project(NewProject {
                name: "Push app".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "Push fix".into(),
                ..Default::default()
            })
            .unwrap();
        s.create_note(None, "Push notes", "").unwrap();
        let done = s.find_status(p, "Done").unwrap().id;
        s.move_task(t.id, done, None).unwrap();
        assert!(s.resolve_link(&format!("#{}", t.id)).unwrap().unwrap().done);

        let kinds: Vec<String> = s
            .link_suggestions("push")
            .unwrap()
            .into_iter()
            .map(|l| l.kind)
            .collect();
        assert_eq!(kinds, ["project", "note", "task"]);
        assert_eq!(
            s.link_suggestions(&format!("#{}", t.id)).unwrap()[0].id,
            t.id
        );
    }
}
