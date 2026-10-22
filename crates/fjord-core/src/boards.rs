//! Azure team boards: a Fjord project whose columns came from a team's board remembers
//! that board's columns (and which state each maps to per work item type), and each
//! imported item remembers its type and board column. With that, a move in Fjord can
//! send the work item its new board column and the matching state.

use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::store::Store;

const BOARD_KEY_PREFIX: &str = "azure.board.";

/// One column on an Azure team board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardColumn {
    pub name: String,
    /// Work item type → the state a work item of that type gets in this column.
    pub state_mappings: BTreeMap<String, String>,
    /// The board's last column, where finished work goes.
    #[serde(default)]
    pub is_done: bool,
}

/// The team board a Fjord project's columns came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBoard {
    /// The board's own column field, e.g. `WEF_6CB4…_Kanban.Column`.
    pub column_field: String,
    pub columns: Vec<BoardColumn>,
}

impl TeamBoard {
    pub fn column(&self, name: &str) -> Option<&BoardColumn> {
        self.columns
            .iter()
            .find(|c| c.name.trim().eq_ignore_ascii_case(name.trim()))
    }

    /// The first column a work item of `kind` in `state` goes in, for items whose
    /// board column isn't known yet.
    pub fn column_for_state(&self, kind: &str, state: &str) -> Option<&BoardColumn> {
        self.columns.iter().find(|c| {
            c.state_mappings
                .iter()
                .any(|(k, s)| k.eq_ignore_ascii_case(kind) && s.eq_ignore_ascii_case(state))
        })
    }
}

impl Store {
    pub fn team_board(&self, project_id: i64) -> Result<Option<TeamBoard>> {
        Ok(self
            .get_setting(&format!("{BOARD_KEY_PREFIX}{project_id}"))?
            .and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn set_team_board(&mut self, project_id: i64, board: &TeamBoard) -> Result<()> {
        self.get_project(project_id)?;
        let json = serde_json::to_string(board).expect("board serializes");
        self.set_setting(&format!("{BOARD_KEY_PREFIX}{project_id}"), &json)
    }

    /// Forgets the team board, e.g. when the project's columns come from states instead.
    pub fn clear_team_board(&mut self, project_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            [format!("{BOARD_KEY_PREFIX}{project_id}")],
        )?;
        Ok(())
    }

    /// Records an imported item's type.
    pub fn set_external_kind(&mut self, task_id: i64, kind: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE external_links SET kind = ?1 WHERE task_id = ?2",
            params![kind, task_id],
        )?;
        Ok(())
    }

    pub fn external_kind(&self, task_id: i64) -> Result<Option<String>> {
        self.link_text(task_id, "kind")
    }

    pub fn external_board_column(&self, task_id: i64) -> Result<Option<String>> {
        self.link_text(task_id, "board_column")
    }

    /// The last board column recorded for an imported item, and records a new one.
    /// Returns the previous column (None the first time).
    pub fn swap_external_board_column(
        &mut self,
        task_id: i64,
        column: &str,
    ) -> Result<Option<String>> {
        let previous = self.external_board_column(task_id)?;
        self.conn.execute(
            "UPDATE external_links SET board_column = ?1 WHERE task_id = ?2",
            params![column, task_id],
        )?;
        Ok(previous)
    }

    fn link_text(&self, task_id: i64, column: &'static str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {column} FROM external_links WHERE task_id = ?1"),
                [task_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> TeamBoard {
        let col = |name: &str, story: &str, bug: &str| BoardColumn {
            name: name.into(),
            state_mappings: [("User Story", story), ("Bug", bug)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            is_done: name == "Closed",
        };
        TeamBoard {
            column_field: "WEF_1_Kanban.Column".into(),
            columns: vec![
                col("New", "New", "New"),
                col("Ready", "New", "Approved"),
                col("In Review", "Active", "Committed"),
                col("Closed", "Closed", "Done"),
            ],
        }
    }

    #[test]
    fn finds_columns_by_name_and_by_state() {
        let b = board();
        assert_eq!(b.column("in review").unwrap().name, "In Review");
        assert_eq!(b.column_for_state("bug", "approved").unwrap().name, "Ready");
        // The first column mapped to the state wins.
        assert_eq!(b.column_for_state("User Story", "New").unwrap().name, "New");
        assert!(b.column_for_state("Task", "New").is_none());
    }

    #[test]
    fn board_and_item_columns_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path(), "tester").unwrap();
        let p = s
            .create_project(crate::NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        assert_eq!(s.team_board(p).unwrap(), None);
        s.set_team_board(p, &board()).unwrap();
        assert_eq!(s.team_board(p).unwrap(), Some(board()));
        assert!(s.set_team_board(999, &board()).is_err());
        s.clear_team_board(p).unwrap();
        assert_eq!(s.team_board(p).unwrap(), None);
        s.set_team_board(p, &board()).unwrap();

        let (task, _) = s
            .upsert_imported_task(
                p,
                &crate::ExternalItem {
                    source: "azure".into(),
                    external_id: "contoso/1".into(),
                    url: "https://dev.azure.com/contoso/P/_workitems/edit/1".into(),
                    rev: 1,
                    title: "T".into(),
                    body_md: String::new(),
                    priority: 0,
                    due_at: None,
                },
            )
            .unwrap();
        s.set_external_kind(task.id, "Bug").unwrap();
        assert_eq!(s.external_kind(task.id).unwrap().as_deref(), Some("Bug"));
        assert_eq!(
            s.swap_external_board_column(task.id, "Ready").unwrap(),
            None
        );
        assert_eq!(
            s.swap_external_board_column(task.id, "In Review")
                .unwrap()
                .as_deref(),
            Some("Ready")
        );
    }
}
