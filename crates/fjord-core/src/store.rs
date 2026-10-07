use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::error::{Error, Result};
use crate::schema;

const DB_FILE: &str = "fjord.db";
const FILES_DIR: &str = "files";

/// Entry point to all Fjord data. One `Store` per process; GUI, CLI and MCP
/// each open their own and share the database through SQLite WAL.
pub struct Store {
    pub(crate) conn: Connection,
    pub(crate) files_dir: PathBuf,
    pub(crate) actor: String,
}

impl Store {
    /// Opens (and creates/migrates) the store in `data_dir`.
    /// `actor` is recorded on every change, e.g. "jonas" or "claude".
    pub fn open(data_dir: &Path, actor: &str) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let conn = Connection::open(data_dir.join(DB_FILE))?;
        Self::init(conn, data_dir.join(FILES_DIR), actor)
    }

    /// In-memory database with files in `files_dir`; meant for tests.
    pub fn open_in_memory(files_dir: &Path, actor: &str) -> Result<Self> {
        Self::init(
            Connection::open_in_memory()?,
            files_dir.to_path_buf(),
            actor,
        )
    }

    fn init(mut conn: Connection, files_dir: PathBuf, actor: &str) -> Result<Self> {
        let actor = actor.trim();
        if actor.is_empty() {
            return Err(Error::Invalid("actor must not be empty".into()));
        }
        schema::configure(&conn)?;
        schema::migrate(&mut conn)?;
        std::fs::create_dir_all(&files_dir)?;
        Ok(Self {
            conn,
            files_dir,
            actor: actor.to_string(),
        })
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// Data directory: $FJORD_DATA_DIR, else the platform data dir:
    /// ~/.local/share/fjord (Linux, honours $XDG_DATA_HOME), %APPDATA%\fjord (Windows),
    /// ~/Library/Application Support/fjord (macOS).
    pub fn default_dir() -> PathBuf {
        env_or_platform("FJORD_DATA_DIR", dirs::data_dir())
    }

    /// Bumps whenever anything changes; lets the GUI cheaply poll for
    /// changes made by other processes (CLI/agent).
    pub fn change_counter(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT coalesce(max(id), 0) FROM activity", [], |r| {
                r.get(0)
            })?)
    }

    pub(crate) fn log(
        &self,
        project_id: Option<i64>,
        task_id: Option<i64>,
        action: &str,
        subject: &str,
        detail: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO activity (project_id, task_id, actor, action, subject, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project_id, task_id, self.actor, action, subject, detail],
        )?;
        Ok(())
    }
}

/// Config directory (keymap.json): $FJORD_CONFIG_DIR, else ~/.config/fjord on Linux,
/// %APPDATA%\fjord on Windows, ~/Library/Application Support/fjord on macOS.
pub fn config_dir() -> PathBuf {
    env_or_platform("FJORD_CONFIG_DIR", dirs::config_dir())
}

/// Who is acting: $FJORD_ACTOR, else the OS user name ($USER / %USERNAME%).
pub fn default_actor() -> String {
    ["FJORD_ACTOR", "USER", "USERNAME"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "user".into())
}

pub(crate) fn env_or_platform(var: &str, platform: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = std::env::var_os(var).filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    platform.unwrap_or_else(std::env::temp_dir).join("fjord")
}

pub(crate) fn require_text(value: &str, field: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::Invalid(format!("{field} must not be empty")));
    }
    Ok(trimmed.to_string())
}
