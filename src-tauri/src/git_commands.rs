//! Git & GitHub commands. Local git calls are quick and run inline; network
//! calls run on a worker thread without holding the store lock, so the UI and
//! other commands stay responsive while GitHub answers.

use std::path::PathBuf;

use fjord_core::{ProjectRepo, Task};
use fjord_vcs::{GitOverview, PullRequest, StartedBranch, SyncReport};
use tauri::State;

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn with_store<T>(
    state: &State<AppState>,
    f: impl FnOnce(&mut fjord_core::Store) -> fjord_vcs::Result<T>,
) -> CmdResult<T> {
    let mut store = state
        .0
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    f(&mut store).map_err(|e| e.to_string())
}

async fn off_thread<T: Send + 'static>(
    f: impl FnOnce() -> fjord_vcs::Result<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project_repo(state: State<AppState>, project_id: i64) -> CmdResult<Option<ProjectRepo>> {
    with_store(&state, |s| Ok(s.get_project_repo(project_id)?))
}

#[tauri::command]
pub fn link_repo(state: State<AppState>, project_id: i64, path: PathBuf) -> CmdResult<ProjectRepo> {
    with_store(&state, |s| fjord_vcs::link_repo(s, project_id, &path))
}

#[tauri::command]
pub fn unlink_repo(state: State<AppState>, project_id: i64) -> CmdResult<()> {
    with_store(&state, |s| Ok(s.unlink_project_repo(project_id)?))
}

#[tauri::command]
pub fn set_repo_auto_move(
    state: State<AppState>,
    project_id: i64,
    auto_move: bool,
) -> CmdResult<()> {
    with_store(&state, |s| Ok(s.set_repo_auto_move(project_id, auto_move)?))
}

#[tauri::command]
pub fn git_overview(state: State<AppState>, project_id: i64) -> CmdResult<GitOverview> {
    with_store(&state, |s| fjord_vcs::overview(s, project_id))
}

#[tauri::command]
pub fn start_branch(state: State<AppState>, task_id: i64) -> CmdResult<StartedBranch> {
    with_store(&state, |s| fjord_vcs::start_branch(s, task_id))
}

#[tauri::command]
pub async fn sync_pull_requests(
    state: State<'_, AppState>,
    project_id: i64,
) -> CmdResult<SyncReport> {
    let repo = with_store(&state, |s| Ok(s.get_project_repo(project_id)?))?
        .ok_or("this project is not linked to a git repository")?;
    let prs = off_thread(move || fjord_vcs::fetch_pull_requests(&repo)).await?;
    with_store(&state, |s| fjord_vcs::apply_sync(s, project_id, prs))
}

#[tauri::command]
pub async fn open_pull_request(
    state: State<'_, AppState>,
    task_id: i64,
    draft: bool,
) -> CmdResult<PullRequest> {
    let (task, repo): (Task, ProjectRepo) = with_store(&state, |s| {
        let task = s.get_task(task_id)?;
        let repo = s
            .get_project_repo(task.project_id)?
            .ok_or(fjord_vcs::VcsError::NotLinked)?;
        Ok((task, repo))
    })?;
    off_thread(move || fjord_vcs::open_pull_request_for(&task, &repo, draft)).await
}

/// Opens a GitHub link in the default browser (only https URLs).
#[tauri::command]
pub fn open_url(url: String) -> CmdResult<()> {
    if !url.starts_with("https://") {
        return Err("only https links can be opened".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}
