use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::models::Attachment;
use crate::store::Store;

const ATTACHMENT_SELECT: &str = "SELECT a.id, a.project_id, a.task_id, a.original_name, f.sha256, f.size,
        a.added_by, a.created_at
     FROM attachments a JOIN files f ON f.id = a.file_id";

fn attachment_from_row(row: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        original_name: row.get(3)?,
        sha256: row.get(4)?,
        size: row.get(5)?,
        added_by: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn sha256_of(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn not_found_or_io(source: &Path) -> impl Fn(io::Error) -> Error + '_ {
    move |e| match e.kind() {
        io::ErrorKind::NotFound => Error::NotFound(source.display().to_string()),
        _ => Error::Io(e),
    }
}

impl Store {
    fn blob_path(&self, sha256: &str) -> PathBuf {
        self.files_dir.join(&sha256[..2]).join(sha256)
    }

    /// Copies `source` into the content-addressed store and attaches it to a
    /// project (and optionally a task). Identical files are stored once.
    pub fn attach_file(&mut self, project_id: i64, task_id: Option<i64>, source: &Path) -> Result<Attachment> {
        let project = self.get_project(project_id)?;
        if let Some(task_id) = task_id
            && self.get_task(task_id)?.project_id != project_id {
                return Err(Error::Invalid(format!("task {task_id} is not in project {project_id}")));
            }
        let meta = fs::metadata(source).map_err(not_found_or_io(source))?;
        if !meta.is_file() {
            return Err(Error::Invalid(format!("{} is not a file", source.display())));
        }
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| Error::Invalid("file has no name".into()))?;

        let sha = sha256_of(source)?;
        let blob = self.blob_path(&sha);
        if !blob.exists() {
            fs::create_dir_all(blob.parent().expect("blob path has a parent"))?;
            let tmp = blob.with_extension("part");
            fs::copy(source, &tmp)?;
            fs::rename(&tmp, &blob)?;
        }

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO files (sha256, size) VALUES (?1, ?2) ON CONFLICT(sha256) DO NOTHING",
            params![sha, meta.len() as i64],
        )?;
        let file_id: i64 = tx.query_row("SELECT id FROM files WHERE sha256 = ?1", [&sha], |r| r.get(0))?;
        tx.execute(
            "INSERT INTO attachments (file_id, project_id, task_id, original_name, added_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![file_id, project_id, task_id, name, self.actor],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        self.touch_project(project_id)?;
        self.log(Some(project.id), task_id, "file.attach", &name, None)?;
        self.get_attachment(id)
    }

    pub fn get_attachment(&self, id: i64) -> Result<Attachment> {
        self.conn
            .query_row(&format!("{ATTACHMENT_SELECT} WHERE a.id = ?1"), [id], attachment_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("attachment {id}")))
    }

    /// All attachments of a project, or only those of one task.
    pub fn list_attachments(&self, project_id: i64, task_id: Option<i64>) -> Result<Vec<Attachment>> {
        let mut stmt = self.conn.prepare(&format!(
            "{ATTACHMENT_SELECT} WHERE a.project_id = ?1 AND (?2 IS NULL OR a.task_id = ?2)
             ORDER BY a.created_at DESC, a.id DESC"
        ))?;
        let rows = stmt.query_map(params![project_id, task_id], attachment_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Where the bytes of an attachment live on disk (for opening/preview).
    pub fn attachment_path(&self, id: i64) -> Result<PathBuf> {
        Ok(self.blob_path(&self.get_attachment(id)?.sha256))
    }

    /// Removes the link only; the blob stays (other attachments may share it).
    pub fn detach_file(&mut self, id: i64) -> Result<()> {
        let a = self.get_attachment(id)?;
        self.conn.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
        self.log(Some(a.project_id), a.task_id, "file.detach", &a.original_name, None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    fn project(s: &mut Store, name: &str) -> i64 {
        s.create_project(NewProject { name: name.into(), ..Default::default() }).unwrap().id
    }

    #[test]
    fn attach_copies_file_and_dedupes_identical_content() {
        let (mut s, dir) = store();
        let p = project(&mut s, "P");
        let src = dir.path().join("skisse.png");
        fs::write(&src, b"fake png bytes").unwrap();

        let a = s.attach_file(p, None, &src).unwrap();
        let b = s.attach_file(p, None, &src).unwrap();
        assert_eq!(a.original_name, "skisse.png");
        assert_eq!(a.sha256, b.sha256);
        assert_eq!(a.size, 14);
        assert_eq!(fs::read(s.attachment_path(a.id).unwrap()).unwrap(), b"fake png bytes");
        assert_eq!(s.list_attachments(p, None).unwrap().len(), 2);
        let blobs: i64 = s.conn.query_row("SELECT count(*) FROM files", [], |r| r.get(0)).unwrap();
        assert_eq!(blobs, 1);
    }

    #[test]
    fn attachments_can_be_filtered_by_task() {
        let (mut s, dir) = store();
        let p = project(&mut s, "P");
        let t = s.create_task(NewTask { project_id: p, title: "T".into(), ..Default::default() }).unwrap();
        let src = dir.path().join("a.txt");
        fs::write(&src, b"a").unwrap();
        s.attach_file(p, None, &src).unwrap();
        s.attach_file(p, Some(t.id), &src).unwrap();
        assert_eq!(s.list_attachments(p, Some(t.id)).unwrap().len(), 1);
    }

    #[test]
    fn rejects_missing_files_directories_and_foreign_tasks() {
        let (mut s, dir) = store();
        let p1 = project(&mut s, "P1");
        let p2 = project(&mut s, "P2");
        let t2 = s.create_task(NewTask { project_id: p2, title: "T".into(), ..Default::default() }).unwrap();
        assert!(matches!(s.attach_file(p1, None, &dir.path().join("nope")), Err(Error::NotFound(_))));
        assert!(matches!(s.attach_file(p1, None, dir.path()), Err(Error::Invalid(_))));
        let src = dir.path().join("a.txt");
        fs::write(&src, b"a").unwrap();
        assert!(matches!(s.attach_file(p1, Some(t2.id), &src), Err(Error::Invalid(_))));
    }

    #[test]
    fn detach_keeps_blob_on_disk() {
        let (mut s, dir) = store();
        let p = project(&mut s, "P");
        let src = dir.path().join("a.txt");
        fs::write(&src, b"a").unwrap();
        let a = s.attach_file(p, None, &src).unwrap();
        let path = s.attachment_path(a.id).unwrap();
        s.detach_file(a.id).unwrap();
        assert!(path.exists());
        assert!(s.list_attachments(p, None).unwrap().is_empty());
    }
}
