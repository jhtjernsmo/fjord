use rusqlite::{OptionalExtension, Row, params};

use crate::error::{Error, Result};
use crate::models::{Activity, Note, SearchHit};
use crate::store::{Store, require_text};

const MAX_SEARCH_HITS: i64 = 50;

fn note_from_row(row: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        body_md: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

/// Turns user text into a safe FTS5 prefix query: each word quoted, `*` suffix.
fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\"*"))
        .collect();
    if terms.is_empty() { None } else { Some(terms.join(" ")) }
}

impl Store {
    pub fn add_note(&mut self, project_id: i64, title: &str, body_md: &str) -> Result<Note> {
        let title = require_text(title, "note title")?;
        self.get_project(project_id)?;
        self.conn.execute(
            "INSERT INTO notes (project_id, title, body_md) VALUES (?1, ?2, ?3)",
            params![project_id, title, body_md],
        )?;
        let id = self.conn.last_insert_rowid();
        self.touch_project(project_id)?;
        self.log(Some(project_id), None, "note.create", &format!("skrev notatet «{title}»"))?;
        self.get_note(id)
    }

    pub fn update_note(&mut self, id: i64, title: &str, body_md: &str) -> Result<Note> {
        let title = require_text(title, "note title")?;
        let note = self.get_note(id)?;
        self.conn.execute(
            "UPDATE notes SET title = ?1, body_md = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?3",
            params![title, body_md, id],
        )?;
        self.touch_project(note.project_id)?;
        self.log(Some(note.project_id), None, "note.update", &format!("oppdaterte notatet «{title}»"))?;
        self.get_note(id)
    }

    pub fn get_note(&self, id: i64) -> Result<Note> {
        self.conn
            .query_row(
                "SELECT id, project_id, title, body_md, updated_at FROM notes WHERE id = ?1",
                [id],
                note_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("note {id}")))
    }

    pub fn list_notes(&self, project_id: i64) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, body_md, updated_at FROM notes
             WHERE project_id = ?1 ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([project_id], note_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Full-text search across tasks, notes and file names (prefix matching,
    /// diacritics-insensitive). Archived tasks/projects are excluded.
    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        let Some(fts) = fts_query(query) else { return Ok(Vec::new()) };
        let mut stmt = self.conn.prepare(
            "SELECT f.kind, f.ref_id, f.project_id, f.title,
                    snippet(search_fts, 4, '[', ']', '…', 12)
             FROM search_fts f
             JOIN projects p ON p.id = f.project_id AND p.archived_at IS NULL
             LEFT JOIN tasks t ON f.kind = 'task' AND t.id = f.ref_id
             WHERE search_fts MATCH ?1 AND (f.kind != 'task' OR t.archived_at IS NULL)
             ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts, MAX_SEARCH_HITS], |row| {
            Ok(SearchHit {
                kind: row.get(0)?,
                ref_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                snippet: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Newest first; `project_id = None` gives activity across all projects.
    pub fn recent_activity(&self, project_id: Option<i64>, limit: i64) -> Result<Vec<Activity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, task_id, actor, action, summary, created_at FROM activity
             WHERE ?1 IS NULL OR project_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(Activity {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_id: row.get(2)?,
                actor: row.get(3)?,
                action: row.get(4)?,
                summary: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask, TaskPatch};
    use crate::test_util::store;

    #[test]
    fn fts_query_quotes_terms_and_drops_quotes() {
        assert_eq!(fts_query("push varsel").as_deref(), Some("\"push\"* \"varsel\"*"));
        assert_eq!(fts_query(" \"\" "), None);
        assert_eq!(fts_query("a\"b OR").as_deref(), Some("\"ab\"* \"OR\"*"));
    }

    #[test]
    fn search_finds_tasks_notes_and_files_by_prefix_without_diacritics() {
        let (mut s, dir) = store();
        let p = s.create_project(NewProject { name: "Bokost".into(), ..Default::default() }).unwrap();
        s.create_task(NewTask {
            project_id: p.id,
            title: "Fiks push-varsler".into(),
            body_md: "OneSignal sender dobbelt".into(),
            ..Default::default()
        })
        .unwrap();
        s.add_note(p.id, "Møtenotater", "Lansering før påske").unwrap();
        let file = dir.path().join("varsel-logg.txt");
        std::fs::write(&file, b"x").unwrap();
        s.attach_file(p.id, None, &file).unwrap();

        let kinds = |q: &str| {
            let mut k: Vec<String> = s.search(q).unwrap().into_iter().map(|h| h.kind).collect();
            k.sort();
            k
        };
        assert_eq!(kinds("onesig"), ["task"]);
        assert_eq!(kinds("møtenot"), ["note"]);
        assert_eq!(kinds("paske"), ["note"]); // å folds to a; ø/æ are letters, not diacritics
        assert_eq!(kinds("vars"), ["file", "task"]);
        assert!(s.search("   ").unwrap().is_empty());
    }

    #[test]
    fn search_follows_edits_and_skips_archived() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "P".into(), ..Default::default() }).unwrap();
        let t = s.create_task(NewTask { project_id: p.id, title: "gammel".into(), ..Default::default() }).unwrap();
        s.update_task(t.id, TaskPatch { title: Some("ny tittel".into()), ..Default::default() }).unwrap();
        assert!(s.search("gammel").unwrap().is_empty());
        assert_eq!(s.search("tittel").unwrap().len(), 1);
        s.set_task_archived(t.id, true).unwrap();
        assert!(s.search("tittel").unwrap().is_empty());
    }

    #[test]
    fn activity_records_actor_newest_first() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "P".into(), ..Default::default() }).unwrap();
        s.add_note(p.id, "N", "").unwrap();
        let log = s.recent_activity(Some(p.id), 10).unwrap();
        assert_eq!(log.iter().map(|a| a.action.as_str()).collect::<Vec<_>>(), ["note.create", "project.create"]);
        assert!(log.iter().all(|a| a.actor == "tester"));
        assert!(s.change_counter().unwrap() >= 2);
    }
}
