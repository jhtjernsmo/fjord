use rusqlite::{OptionalExtension, Row, params};

use crate::error::{Error, Result};
use crate::models::{NewProject, Project, ProjectPatch, ProjectSummary, Status};
use crate::store::{Store, require_text};

/// (name, color, is_done) for every new project's board.
const DEFAULT_STATUSES: &[(&str, &str, bool)] = &[
    ("Å gjøre", "#8b8f98", false),
    ("Pågår", "#f5a524", false),
    ("Ferdig", "#3fb950", true),
];

const PROJECT_COLS: &str =
    "id, name, slug, description, color, icon, created_at, updated_at, archived_at";

fn project_from_row(row: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        description: row.get(3)?,
        color: row.get(4)?,
        icon: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        archived_at: row.get(8)?,
    })
}

fn status_from_row(row: &Row) -> rusqlite::Result<Status> {
    Ok(Status {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        color: row.get(3)?,
        position: row.get(4)?,
        is_done: row.get(5)?,
    })
}

/// "Min Første App!" -> "min-forste-app"
pub fn slugify(name: &str) -> String {
    let mapped: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'æ' => 'e',
            'ø' => 'o',
            'å' => 'a',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let slug = mapped.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() { "project".to_string() } else { slug }
}

impl Store {
    pub fn create_project(&mut self, new: NewProject) -> Result<Project> {
        let name = require_text(&new.name, "project name")?;
        let slug = self.unique_slug(&slugify(&name))?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO projects (name, slug, description, color, icon)
             VALUES (?1, ?2, ?3, coalesce(?4, '#7c9cff'), coalesce(?5, '📁'))",
            params![name, slug, new.description.trim(), new.color, new.icon],
        )?;
        let id = tx.last_insert_rowid();
        for (position, (status, color, is_done)) in DEFAULT_STATUSES.iter().enumerate() {
            tx.execute(
                "INSERT INTO statuses (project_id, name, color, position, is_done)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, status, color, position as i64, is_done],
            )?;
        }
        tx.commit()?;
        self.log(Some(id), None, "project.create", &format!("opprettet prosjektet «{name}»"))?;
        self.get_project(id)
    }

    fn unique_slug(&self, base: &str) -> Result<String> {
        let mut candidate = base.to_string();
        let mut n = 2;
        while self
            .conn
            .query_row("SELECT 1 FROM projects WHERE slug = ?1", [&candidate], |_| Ok(()))
            .optional()?
            .is_some()
        {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        Ok(candidate)
    }

    pub fn get_project(&self, id: i64) -> Result<Project> {
        self.conn
            .query_row(&format!("SELECT {PROJECT_COLS} FROM projects WHERE id = ?1"), [id], project_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("project {id}")))
    }

    /// Looks a project up by numeric id or slug (handy for CLI and agents).
    pub fn find_project(&self, id_or_slug: &str) -> Result<Project> {
        if let Ok(id) = id_or_slug.parse::<i64>() {
            return self.get_project(id);
        }
        self.conn
            .query_row(
                &format!("SELECT {PROJECT_COLS} FROM projects WHERE slug = ?1"),
                [id_or_slug],
                project_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("project «{id_or_slug}»")))
    }

    pub fn list_projects(&self, include_archived: bool) -> Result<Vec<ProjectSummary>> {
        let cols = PROJECT_COLS.split(", ").map(|c| format!("p.{c}")).collect::<Vec<_>>().join(", ");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {cols},
                (SELECT count(*) FROM tasks t WHERE t.project_id = p.id AND t.archived_at IS NULL),
                (SELECT count(*) FROM tasks t JOIN statuses s ON s.id = t.status_id
                  WHERE t.project_id = p.id AND t.archived_at IS NULL AND s.is_done),
                (SELECT count(*) FROM tasks t JOIN statuses s ON s.id = t.status_id
                  WHERE t.project_id = p.id AND t.archived_at IS NULL AND NOT s.is_done
                    AND t.due_at IS NOT NULL AND t.due_at < date('now'))
             FROM projects p
             WHERE ?1 OR p.archived_at IS NULL
             ORDER BY p.archived_at IS NOT NULL, p.updated_at DESC, p.id DESC"
        ))?;
        let rows = stmt.query_map([include_archived], |row| {
            Ok(ProjectSummary {
                project: project_from_row(row)?,
                task_count: row.get(9)?,
                done_count: row.get(10)?,
                overdue_count: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_project(&mut self, id: i64, patch: ProjectPatch) -> Result<Project> {
        let current = self.get_project(id)?;
        let name = match &patch.name {
            Some(n) => require_text(n, "project name")?,
            None => current.name.clone(),
        };
        self.conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, color = ?3, icon = ?4,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?5",
            params![
                name,
                patch.description.unwrap_or(current.description),
                patch.color.unwrap_or(current.color),
                patch.icon.unwrap_or(current.icon),
                id
            ],
        )?;
        self.log(Some(id), None, "project.update", &format!("oppdaterte prosjektet «{name}»"))?;
        self.get_project(id)
    }

    /// Archiving is reversible; Fjord never hard-deletes from the UI or agents.
    pub fn set_project_archived(&mut self, id: i64, archived: bool) -> Result<Project> {
        let project = self.get_project(id)?;
        self.conn.execute(
            "UPDATE projects SET archived_at = CASE WHEN ?1 THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') END,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?2",
            params![archived, id],
        )?;
        let verb = if archived { "arkiverte" } else { "gjenopprettet" };
        self.log(Some(id), None, "project.archive", &format!("{verb} prosjektet «{}»", project.name))?;
        self.get_project(id)
    }

    pub fn list_statuses(&self, project_id: i64) -> Result<Vec<Status>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, name, color, position, is_done
             FROM statuses WHERE project_id = ?1 ORDER BY position",
        )?;
        let rows = stmt.query_map([project_id], status_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Finds a status in a project by id or (case-insensitive) name.
    pub fn find_status(&self, project_id: i64, id_or_name: &str) -> Result<Status> {
        let wanted = id_or_name.trim().to_lowercase();
        self.list_statuses(project_id)?
            .into_iter()
            .find(|s| s.id.to_string() == wanted || s.name.to_lowercase() == wanted)
            .ok_or_else(|| Error::NotFound(format!("status «{id_or_name}»")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::store;

    #[test]
    fn slugify_handles_norwegian_letters_and_symbols() {
        assert_eq!(slugify("Min Første App!"), "min-forste-app");
        assert_eq!(slugify("Bokost   2.0"), "bokost-2-0");
        assert_eq!(slugify("!!!"), "project");
    }

    #[test]
    fn create_project_adds_default_statuses_and_logs_activity() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: " Reisly ".into(), ..Default::default() }).unwrap();
        assert_eq!(p.name, "Reisly");
        assert_eq!(p.slug, "reisly");
        let statuses = s.list_statuses(p.id).unwrap();
        assert_eq!(statuses.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Å gjøre", "Pågår", "Ferdig"]);
        assert!(statuses[2].is_done);
        assert_eq!(s.recent_activity(Some(p.id), 10).unwrap()[0].action, "project.create");
    }

    #[test]
    fn duplicate_names_get_unique_slugs() {
        let (mut s, _dir) = store();
        let a = s.create_project(NewProject { name: "App".into(), ..Default::default() }).unwrap();
        let b = s.create_project(NewProject { name: "App".into(), ..Default::default() }).unwrap();
        assert_eq!((a.slug.as_str(), b.slug.as_str()), ("app", "app-2"));
    }

    #[test]
    fn empty_name_is_rejected() {
        let (mut s, _dir) = store();
        let err = s.create_project(NewProject { name: "   ".into(), ..Default::default() }).unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn find_project_by_id_or_slug() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "Bokost".into(), ..Default::default() }).unwrap();
        assert_eq!(s.find_project("bokost").unwrap().id, p.id);
        assert_eq!(s.find_project(&p.id.to_string()).unwrap().id, p.id);
        assert!(matches!(s.find_project("nope"), Err(Error::NotFound(_))));
    }

    #[test]
    fn archived_projects_are_hidden_unless_requested() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "Old".into(), ..Default::default() }).unwrap();
        s.set_project_archived(p.id, true).unwrap();
        assert!(s.list_projects(false).unwrap().is_empty());
        assert_eq!(s.list_projects(true).unwrap().len(), 1);
        s.set_project_archived(p.id, false).unwrap();
        assert_eq!(s.list_projects(false).unwrap().len(), 1);
    }

    #[test]
    fn update_project_keeps_unpatched_fields() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "A".into(), description: "desc".into(), ..Default::default() }).unwrap();
        let u = s.update_project(p.id, ProjectPatch { color: Some("#ff0000".into()), ..Default::default() }).unwrap();
        assert_eq!((u.name.as_str(), u.description.as_str(), u.color.as_str()), ("A", "desc", "#ff0000"));
    }

    #[test]
    fn find_status_by_name_is_case_insensitive() {
        let (mut s, _dir) = store();
        let p = s.create_project(NewProject { name: "A".into(), ..Default::default() }).unwrap();
        assert!(s.find_status(p.id, "ferdig").unwrap().is_done);
        assert!(s.find_status(p.id, "PÅGÅR").is_ok());
    }
}
