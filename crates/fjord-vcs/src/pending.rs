//! Pending tasks: waiting on someone else, typically a reply from the client. A task
//! from Azure Boards follows its work item both ways: the work item entering the
//! pending state (Settings → Integrations, "Pending" by default) marks the task
//! pending, and with "Send state changes to Azure" on, marking the task pending in
//! Fjord moves the work item there.

use fjord_core::{ImportSettings, Store, Task};

use crate::{Result, split_external_id};

/// A state change to send to Azure Boards.
#[derive(Debug, Clone, PartialEq)]
pub struct StatePush {
    pub task_id: i64,
    pub org: String,
    pub id: u64,
    pub state: String,
    /// True when the push takes the work item out of the pending state.
    pub resuming: bool,
    /// The work item's new column on a team board: (board column field, column).
    /// With a column, an empty `state` means only the column changes.
    pub column: Option<(String, String)>,
}

impl StatePush {
    /// The Fjord column this push is for.
    pub fn column_name(&self) -> &str {
        self.column
            .as_ref()
            .map_or(self.state.as_str(), |(_, c)| c.as_str())
    }
}

/// What Azure needs after a task was marked pending or not.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingSync {
    /// Not from Azure, writing to Azure is off, or the work item already has the right state.
    Nothing,
    Push(StatePush),
    /// The work item is pending in Azure, but Fjord doesn't know which state it came
    /// from, so it has to be moved back in Azure by hand.
    UnknownResumeState {
        url: String,
    },
}

/// Store only: marks the task pending or not, and works out what to send to Azure.
pub fn mark_pending(store: &mut Store, task_id: i64, pending: bool) -> Result<(Task, PendingSync)> {
    let task = store.set_task_pending(task_id, pending)?;
    let settings = store.import_settings()?;
    let sync = sync_for(store, &settings, task_id, pending)?;
    Ok((task, sync))
}

fn sync_for(
    store: &mut Store,
    settings: &ImportSettings,
    task_id: i64,
    pending: bool,
) -> Result<PendingSync> {
    let Some(link) = store
        .external_link_for_task(task_id)?
        .filter(|l| l.source == "azure")
    else {
        return Ok(PendingSync::Nothing);
    };
    let Some((org, id)) = split_external_id(&link.external_id) else {
        return Ok(PendingSync::Nothing);
    };
    if !settings.write_states || settings.pending_state.trim().is_empty() {
        return Ok(PendingSync::Nothing);
    }
    let current = store.external_state(task_id)?;
    let in_pending = current
        .as_deref()
        .is_some_and(|s| settings.is_pending_state(s));
    if pending == in_pending {
        return Ok(PendingSync::Nothing);
    }
    if pending {
        store.set_resume_state(task_id, current.as_deref())?;
        return Ok(PendingSync::Push(StatePush {
            task_id,
            org,
            id,
            state: settings.pending_state.trim().to_string(),
            resuming: false,
            column: None,
        }));
    }
    match store.resume_state(task_id)? {
        Some(state) => Ok(PendingSync::Push(StatePush {
            task_id,
            org,
            id,
            state,
            resuming: true,
            column: None,
        })),
        None => Ok(PendingSync::UnknownResumeState { url: link.url }),
    }
}

/// Network only: sends the state change.
pub fn push_state(push: &StatePush) -> Result<()> {
    let column = push.column.as_ref().map(|(f, c)| (f.as_str(), c.as_str()));
    crate::azure_write::set_state_and_column(&push.org, push.id, &push.state, column)
}

/// Store only: records the state Azure now has, so the next import doesn't see a change.
pub fn record_pushed(store: &mut Store, push: &StatePush) -> Result<()> {
    if !push.state.trim().is_empty() {
        store.swap_external_state(push.task_id, &push.state)?;
    }
    if let Some((_, column)) = &push.column {
        store.swap_external_board_column(push.task_id, column)?;
    }
    if push.resuming {
        store.set_resume_state(push.task_id, None)?;
    }
    Ok(())
}

/// Store and network, for the CLI and agents: marks the task and updates Azure.
/// Returns the task and, when Azure couldn't follow, why (the task is marked anyway).
pub fn set_pending(
    store: &mut Store,
    task_id: i64,
    pending: bool,
) -> Result<(Task, Option<String>)> {
    let (task, sync) = mark_pending(store, task_id, pending)?;
    let warning = match sync {
        PendingSync::Nothing => None,
        PendingSync::Push(push) => match push_state(&push) {
            Ok(()) => {
                record_pushed(store, &push)?;
                None
            }
            Err(e) => Some(format!("Azure wasn't updated: {e}")),
        },
        PendingSync::UnknownResumeState { url } => Some(format!(
            "the work item is still pending in Azure; move it back there: {url}"
        )),
    };
    Ok((task, warning))
}

/// During import: follows the work item into or out of the pending state, but only
/// when its state changed in Azure, so a task marked in Fjord isn't undone.
pub(crate) fn follow_azure(
    store: &mut Store,
    settings: &ImportSettings,
    task: Task,
    previous: Option<&str>,
    state: &str,
) -> Result<Task> {
    let now_pending = settings.is_pending_state(state);
    let was_pending = previous.is_some_and(|p| settings.is_pending_state(p));
    if now_pending && !was_pending {
        store.set_resume_state(task.id, previous)?;
        Ok(store.set_task_pending(task.id, true)?)
    } else if was_pending && !now_pending {
        store.set_resume_state(task.id, None)?;
        Ok(store.set_task_pending(task.id, false)?)
    } else {
        Ok(task)
    }
}
