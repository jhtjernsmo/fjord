//! Column moves and Azure Boards: a move saves at once, and the work item's new state
//! is sent afterwards on one worker thread, in order, so dragging never waits on the
//! network. When Azure doesn't follow, the UI gets an `azure-warning` event.

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};

use fjord_core::{Store, Task};
use fjord_vcs::azure_states::{
    StateColumnsReport, add_board_columns, add_state_columns, project_states,
};
use fjord_vcs::azure_teams::{AzureTeam, my_teams, team_board};
use fjord_vcs::pending::{StatePush, push_state, record_pushed};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

pub const AZURE_WARNING_EVENT: &str = "azure-warning";

struct Job {
    app: AppHandle,
    store: Arc<Mutex<Store>>,
    push: StatePush,
}

/// One worker for every state change, so two quick moves reach Azure in the order made.
fn queue() -> &'static Mutex<Sender<Job>> {
    static QUEUE: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::spawn(move || {
            for job in rx {
                if let Some(warning) = send(&job) {
                    let _ = job.app.emit(AZURE_WARNING_EVENT, warning);
                }
            }
        });
        Mutex::new(tx)
    })
}

/// Sends one state change, unless the task has moved on since (a newer job follows).
fn send(job: &Job) -> Option<String> {
    let still_wanted = job.store.lock().ok().is_some_and(|s| {
        s.get_task(job.push.task_id)
            .and_then(|t| s.get_status(t.status_id))
            .is_ok_and(|c| c.name.trim().eq_ignore_ascii_case(job.push.column_name()))
    });
    if !still_wanted {
        return None;
    }
    if let Err(e) = push_state(&job.push) {
        return Some(format!("Azure wasn't updated: {e}"));
    }
    let mut store = job.store.lock().ok()?;
    record_pushed(&mut store, &job.push)
        .err()
        .map(|e| e.to_string())
}

/// Moves a task to `status_id`, placed before `before_task_id` or last. A task from
/// Azure Boards sends its work item the column's state when writing to Azure is on.
#[tauri::command]
pub fn move_task(
    app: AppHandle,
    state: State<AppState>,
    id: i64,
    status_id: i64,
    before_task_id: Option<i64>,
) -> CmdResult<Task> {
    let (task, push) = {
        let mut store = state
            .0
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        fjord_vcs::state_sync::move_task(&mut store, id, status_id, before_task_id)
            .map_err(|e| e.to_string())?
    };
    if let Some(push) = push {
        let job = Job {
            app,
            store: Arc::clone(&state.0),
            push,
        };
        queue()
            .lock()
            .map_err(|_| "Azure queue lock poisoned".to_string())?
            .send(job)
            .map_err(|_| "the Azure queue stopped".to_string())?;
    }
    Ok(task)
}

/// Gives a Fjord project a column for each state of an Azure DevOps project's work
/// items, or with `team`, for each column of that team's board, and puts imported tasks
/// in their column. Only reads from Azure.
#[tauri::command]
pub async fn azure_state_columns(
    state: State<'_, AppState>,
    org: String,
    project: String,
    project_id: i64,
    team: Option<String>,
) -> CmdResult<StateColumnsReport> {
    let team = team.filter(|t| !t.trim().is_empty());
    let columns = tauri::async_runtime::spawn_blocking(move || match team {
        Some(team) => team_board(&org, &project, &team).map(Columns::Board),
        None => project_states(&org, &project).map(Columns::States),
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let mut store = state
        .0
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    match columns {
        Columns::Board(board) => add_board_columns(&mut store, project_id, &board),
        Columns::States(states) => add_state_columns(&mut store, project_id, &states),
    }
    .map_err(|e| e.to_string())
}

enum Columns {
    Board(fjord_core::TeamBoard),
    States(Vec<fjord_vcs::azure_states::AzureState>),
}

/// The Azure DevOps teams you're a member of in `org`.
#[tauri::command]
pub async fn azure_my_teams(org: String) -> CmdResult<Vec<AzureTeam>> {
    tauri::async_runtime::spawn_blocking(move || my_teams(&org))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
