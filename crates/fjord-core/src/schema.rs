//! Database schema and migrations, tracked with `PRAGMA user_version`.

use rusqlite::Connection;

use crate::error::Result;

const MIGRATIONS: &[&str] = &[
    // v1: initial schema
    r#"
    CREATE TABLE projects (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL,
        slug        TEXT NOT NULL UNIQUE,
        description TEXT NOT NULL DEFAULT '',
        color       TEXT NOT NULL DEFAULT '#7c9cff',
        icon        TEXT NOT NULL DEFAULT '>_',
        created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        archived_at TEXT
    );

    CREATE TABLE statuses (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        name       TEXT NOT NULL,
        color      TEXT NOT NULL DEFAULT '#8b8f98',
        position   INTEGER NOT NULL,
        is_done    INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE tasks (
        id          INTEGER PRIMARY KEY,
        project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        status_id   INTEGER NOT NULL REFERENCES statuses(id),
        title       TEXT NOT NULL,
        body_md     TEXT NOT NULL DEFAULT '',
        priority    INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 3),
        due_at      TEXT,
        position    REAL NOT NULL,
        created_by  TEXT NOT NULL,
        created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        archived_at TEXT
    );
    CREATE INDEX tasks_project ON tasks(project_id, status_id, position);

    CREATE TABLE files (
        id         INTEGER PRIMARY KEY,
        sha256     TEXT NOT NULL UNIQUE,
        size       INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    );

    CREATE TABLE attachments (
        id            INTEGER PRIMARY KEY,
        file_id       INTEGER NOT NULL REFERENCES files(id),
        project_id    INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        task_id       INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
        original_name TEXT NOT NULL,
        added_by      TEXT NOT NULL,
        created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    );

    CREATE TABLE notes (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        title      TEXT NOT NULL,
        body_md    TEXT NOT NULL DEFAULT '',
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    );

    CREATE TABLE activity (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
        task_id    INTEGER,
        actor      TEXT NOT NULL,
        action     TEXT NOT NULL,
        subject    TEXT NOT NULL,
        detail     TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    );
    CREATE INDEX activity_project ON activity(project_id, created_at);

    CREATE VIRTUAL TABLE search_fts USING fts5(
        kind UNINDEXED, ref_id UNINDEXED, project_id UNINDEXED, title, body,
        tokenize = 'unicode61 remove_diacritics 2'
    );

    CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
        INSERT INTO search_fts(kind, ref_id, project_id, title, body)
        VALUES ('task', new.id, new.project_id, new.title, new.body_md);
    END;
    CREATE TRIGGER tasks_au AFTER UPDATE OF title, body_md ON tasks BEGIN
        UPDATE search_fts SET title = new.title, body = new.body_md
        WHERE kind = 'task' AND ref_id = new.id;
    END;
    CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
        DELETE FROM search_fts WHERE kind = 'task' AND ref_id = old.id;
    END;

    CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
        INSERT INTO search_fts(kind, ref_id, project_id, title, body)
        VALUES ('note', new.id, new.project_id, new.title, new.body_md);
    END;
    CREATE TRIGGER notes_au AFTER UPDATE OF title, body_md ON notes BEGIN
        UPDATE search_fts SET title = new.title, body = new.body_md
        WHERE kind = 'note' AND ref_id = new.id;
    END;
    CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
        DELETE FROM search_fts WHERE kind = 'note' AND ref_id = old.id;
    END;

    CREATE TRIGGER attachments_ai AFTER INSERT ON attachments BEGIN
        INSERT INTO search_fts(kind, ref_id, project_id, title, body)
        VALUES ('file', new.id, new.project_id, new.original_name, '');
    END;
    CREATE TRIGGER attachments_ad AFTER DELETE ON attachments BEGIN
        DELETE FROM search_fts WHERE kind = 'file' AND ref_id = old.id;
    END;
    "#,
    // v2: link projects to a git repository; remember each task's branch
    r#"
    CREATE TABLE project_repos (
        project_id   INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
        path         TEXT NOT NULL,
        github_owner TEXT,
        github_repo  TEXT,
        auto_move    INTEGER NOT NULL DEFAULT 1
    );
    ALTER TABLE tasks ADD COLUMN branch TEXT;
    "#,
    // v3: app settings (user name, …)
    r#"
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    // v4: notespace — notes may live outside projects; [[wiki links]] are indexed
    r#"
    CREATE TABLE notes_v4 (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
        title      TEXT NOT NULL,
        body_md    TEXT NOT NULL DEFAULT '',
        folder     TEXT NOT NULL DEFAULT '',
        pinned     INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    );
    INSERT INTO notes_v4 (id, project_id, title, body_md, created_at, updated_at)
        SELECT id, project_id, title, body_md, updated_at, updated_at FROM notes;
    DROP TABLE notes;
    ALTER TABLE notes_v4 RENAME TO notes;

    CREATE TABLE note_links (
        note_id   INTEGER NOT NULL,
        kind      TEXT NOT NULL CHECK (kind IN ('project', 'task', 'note')),
        target_id INTEGER NOT NULL,
        PRIMARY KEY (note_id, kind, target_id)
    );
    CREATE INDEX note_links_target ON note_links(kind, target_id);

    CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
        INSERT INTO search_fts(kind, ref_id, project_id, title, body)
        VALUES ('note', new.id, new.project_id, new.title, new.body_md);
    END;
    CREATE TRIGGER notes_au AFTER UPDATE OF title, body_md, project_id ON notes BEGIN
        UPDATE search_fts SET title = new.title, body = new.body_md, project_id = new.project_id
        WHERE kind = 'note' AND ref_id = new.id;
    END;
    CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
        DELETE FROM search_fts WHERE kind = 'note' AND ref_id = old.id;
        DELETE FROM note_links WHERE note_id = old.id;
    END;
    "#,
    // v5: Azure DevOps remotes next to GitHub ones
    r#"
    ALTER TABLE project_repos ADD COLUMN azure_org TEXT;
    ALTER TABLE project_repos ADD COLUMN azure_project TEXT;
    ALTER TABLE project_repos ADD COLUMN azure_repo TEXT;
    "#,
];

pub fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
pub fn version() -> usize {
    MIGRATIONS.len()
}
