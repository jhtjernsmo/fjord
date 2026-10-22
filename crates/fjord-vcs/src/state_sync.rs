//! Column moves follow to Azure Boards (Settings → Integrations → "Send state changes
//! to Azure", off by default): moving a task imported from a work item to another
//! column sets that work item's state to the column's name, or, when the project's
//! columns came from a team board, its board column and the state that column means.
//! One move writes one work item, the one the task was imported from, by its id.
//! Reordering within a column, imports, subtasks and the parent never write, and
//! nothing in Fjord deletes work items.

use fjord_core::{Store, Task};

use crate::pending::{StatePush, push_state, record_pushed};
use crate::{Result, split_external_id};

/// Store only: moves the task, and works out the state to send to Azure, if any.
pub fn move_task(
    store: &mut Store,
    task_id: i64,
    status_id: i64,
    before_task_id: Option<i64>,
) -> Result<(Task, Option<StatePush>)> {
    let from = store.get_task(task_id)?.status_id;
    let task = store.move_task(task_id, status_id, before_task_id)?;
    if task.status_id == from {
        return Ok((task, None));
    }
    let push = push_for(store, &task)?;
    Ok((task, push))
}

fn push_for(store: &mut Store, task: &Task) -> Result<Option<StatePush>> {
    let settings = store.import_settings()?;
    if !settings.write_states {
        return Ok(None);
    }
    let Some(link) = store
        .external_link_for_task(task.id)?
        .filter(|l| l.source == "azure")
    else {
        return Ok(None);
    };
    let Some((org, id)) = split_external_id(&link.external_id) else {
        return Ok(None);
    };
    let column = store.get_status(task.status_id)?.name.trim().to_string();
    // Pending is the pending toggle's job, not a column's.
    if settings.is_pending_state(&column) {
        return Ok(None);
    }
    let current = store.external_state(task.id)?;
    let Some((state, board_column)) = target_for(store, task, &column, current.as_deref())? else {
        return Ok(None);
    };
    // A work item pending in Azure stays there; it goes to the new column's state
    // when the task stops pending.
    if task.pending_since.is_some()
        && current
            .as_deref()
            .is_some_and(|s| settings.is_pending_state(s))
    {
        if !state.is_empty() {
            store.set_resume_state(task.id, Some(&state))?;
        }
        return Ok(None);
    }
    Ok(Some(StatePush {
        task_id: task.id,
        org,
        id,
        state,
        resuming: false,
        column: board_column,
    }))
}

/// A state to send (empty: leave it), and maybe a board column (field, column).
type Target = (String, Option<(String, String)>);

/// What the work item should get for the Fjord column `column`: on a project whose
/// columns came from a team board, that board column and the state it maps to for the
/// item's type; otherwise the state named like the column. None when it already has it.
fn target_for(
    store: &Store,
    task: &Task,
    column: &str,
    current_state: Option<&str>,
) -> Result<Option<Target>> {
    let same = |a: Option<&str>, b: &str| a.is_some_and(|a| a.trim().eq_ignore_ascii_case(b));
    let board = store.team_board(task.project_id)?;
    let Some((board, board_column)) = board
        .as_ref()
        .and_then(|b| b.column(column).map(|c| (b, c)))
    else {
        return Ok((!same(current_state, column)).then(|| (column.to_string(), None)));
    };
    if same(
        store.external_board_column(task.id)?.as_deref(),
        &board_column.name,
    ) {
        return Ok(None);
    }
    let kind = store.external_kind(task.id)?.unwrap_or_default();
    let state = board_column
        .state_mappings
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&kind))
        .map(|(_, s)| s.clone())
        .filter(|s| !same(current_state, s))
        .unwrap_or_default();
    Ok(Some((
        state,
        Some((board.column_field.clone(), board_column.name.clone())),
    )))
}

/// Store and network, for the CLI and agents: moves the task and updates Azure.
/// Returns the task and, when Azure couldn't follow, why (the task moves anyway).
pub fn move_and_sync(
    store: &mut Store,
    task_id: i64,
    status_id: i64,
    before_task_id: Option<i64>,
) -> Result<(Task, Option<String>)> {
    let (task, push) = move_task(store, task_id, status_id, before_task_id)?;
    let warning = match push {
        None => None,
        Some(push) => match push_state(&push) {
            Ok(()) => {
                record_pushed(store, &push)?;
                None
            }
            Err(e) => Some(format!("Azure wasn't updated: {e}")),
        },
    };
    Ok((task, warning))
}
