//! Fjord core: projects, tasks, files, notes, search and activity on SQLite.
//! Shared by the GUI, the `fjord` CLI and the MCP server.

mod error;
mod files;
mod links;
mod models;
mod notes;
mod projects;
mod repos;
mod schema;
mod settings;
mod statuses;
mod store;
mod tasks;

pub use error::{Error, Result};
pub use links::parse_links;
pub use models::*;
pub use projects::{slugify, status_names};
pub use store::{Store, config_dir, default_actor};

#[cfg(test)]
pub(crate) mod test_util {
    use crate::Store;

    /// Fresh in-memory store plus a temp dir that lives as long as the test.
    pub fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in_memory(&dir.path().join("files"), "tester").unwrap();
        (store, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_on_disk_migrates_once_and_reopens() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut s = Store::open(dir.path(), "jonas").unwrap();
            s.create_project(NewProject {
                name: "Persist".into(),
                ..Default::default()
            })
            .unwrap();
        }
        let s = Store::open(dir.path(), "claude").unwrap();
        assert_eq!(s.list_projects(false).unwrap()[0].project.name, "Persist");
        let version: i64 = s
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version as usize, schema::version());
        assert_eq!(s.actor(), "claude");
    }

    #[test]
    fn platform_dirs_end_in_fjord_and_env_overrides_win() {
        let platform = Some(std::path::PathBuf::from("/platform/data"));
        let fallback = store::env_or_platform("FJORD_TEST_VAR_THAT_IS_NEVER_SET", platform);
        assert_eq!(
            fallback,
            std::path::PathBuf::from("/platform/data").join("fjord")
        );
        // PATH is always set, so it stands in for an override here.
        let overridden = store::env_or_platform("PATH", Some("/ignored".into()));
        assert_eq!(
            overridden,
            std::path::PathBuf::from(std::env::var_os("PATH").unwrap())
        );
        assert!(!default_actor().trim().is_empty());
    }

    #[test]
    fn empty_actor_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            Store::open(dir.path(), "  "),
            Err(Error::Invalid(_))
        ));
    }
}
