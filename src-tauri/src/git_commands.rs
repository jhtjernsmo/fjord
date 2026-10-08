//! Git & GitHub commands. Local git calls are quick and run inline; network
//! calls run on a worker thread without holding the store lock, so the UI and
//! other commands stay responsive while GitHub answers.

use std::path::PathBuf;
use std::sync::Arc;

use fjord_core::{ProjectRepo, Task};
use fjord_vcs::{AzureAccount, GitHubAccount, GitOverview, PullRequest, StartedBranch, SyncReport};
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

/// Runs `f` with the store on a worker thread (for git commands that may be slow,
/// e.g. on Windows or in big repositories), so the window never freezes.
async fn with_store_bg<T: Send + 'static>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut fjord_core::Store) -> fjord_vcs::Result<T> + Send + 'static,
) -> CmdResult<T> {
    let shared = Arc::clone(&state.0);
    off_thread(move || {
        let mut store = shared
            .lock()
            .map_err(|_| fjord_vcs::VcsError::Git("store lock poisoned".into()))?;
        f(&mut store)
    })
    .await
}

#[tauri::command]
pub fn get_project_repo(state: State<AppState>, project_id: i64) -> CmdResult<Option<ProjectRepo>> {
    with_store(&state, |s| Ok(s.get_project_repo(project_id)?))
}

#[tauri::command]
pub async fn link_repo(
    state: State<'_, AppState>,
    project_id: i64,
    path: PathBuf,
) -> CmdResult<ProjectRepo> {
    with_store_bg(&state, move |s| fjord_vcs::link_repo(s, project_id, &path)).await
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
pub async fn git_overview(state: State<'_, AppState>, project_id: i64) -> CmdResult<GitOverview> {
    // Read the link quickly, then run git without holding the lock.
    let repo = with_store(&state, |s| Ok(s.get_project_repo(project_id)?))?
        .ok_or("this project is not linked to a git repository")?;
    off_thread(move || fjord_vcs::overview_for(repo)).await
}

#[tauri::command]
pub async fn start_branch(state: State<'_, AppState>, task_id: i64) -> CmdResult<StartedBranch> {
    with_store_bg(&state, move |s| fjord_vcs::start_branch(s, task_id)).await
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

/// Opens a web or mail link in the system's default browser or mail app.
#[tauri::command]
pub fn open_url(url: String) -> CmdResult<()> {
    let allowed = ["https://", "http://", "mailto:"];
    if !allowed
        .iter()
        .any(|p| url.to_ascii_lowercase().starts_with(p))
    {
        return Err("only web and mail links can be opened".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// The GitHub account Fjord uses (None when not connected).
#[tauri::command]
pub async fn github_account() -> CmdResult<Option<GitHubAccount>> {
    off_thread(fjord_vcs::github_account).await
}

/// Verifies a personal access token and saves it in the OS credential store.
#[tauri::command]
pub async fn connect_github(token: String) -> CmdResult<GitHubAccount> {
    off_thread(move || fjord_vcs::connect_github(&token)).await
}

#[tauri::command]
pub async fn disconnect_github() -> CmdResult<()> {
    off_thread(fjord_vcs::disconnect_github).await
}

/// The Azure DevOps identity Fjord uses for an organization (None when not connected).
#[tauri::command]
pub async fn azure_account(org: String) -> CmdResult<Option<AzureAccount>> {
    off_thread(move || fjord_vcs::azure_account(&org)).await
}

/// Verifies a personal access token for an organization and saves it in the OS credential store.
#[tauri::command]
pub async fn connect_azure(org: String, token: String) -> CmdResult<AzureAccount> {
    off_thread(move || fjord_vcs::connect_azure(&org, &token)).await
}

#[tauri::command]
pub async fn disconnect_azure(org: String) -> CmdResult<()> {
    off_thread(move || fjord_vcs::disconnect_azure(&org)).await
}

#[tauri::command]
pub fn get_import_settings(state: State<AppState>) -> CmdResult<fjord_core::ImportSettings> {
    with_store(&state, |s| Ok(s.import_settings()?))
}

#[tauri::command]
pub fn set_import_settings(
    state: State<AppState>,
    settings: fjord_core::ImportSettings,
) -> CmdResult<()> {
    with_store(&state, |s| Ok(s.set_import_settings(&settings)?))
}

/// Fetches work items assigned to you in Azure Boards (off the UI thread) and
/// creates or refreshes tasks in the mapped projects.
#[tauri::command]
pub async fn run_azure_import(state: State<'_, AppState>) -> CmdResult<fjord_vcs::ImportReport> {
    let (settings, known) = with_store(&state, |s| {
        let known: Vec<String> = s
            .external_ids("azure")?
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        Ok((s.import_settings()?, known))
    })?;
    let fetch_settings = settings.clone();
    let fetch = off_thread(move || fjord_vcs::fetch_import(&fetch_settings, &known)).await?;
    with_store(&state, |s| fjord_vcs::apply_import(s, &settings, &fetch))
}

/// Starts "Sign in with GitHub": returns the code the user types on github.com.
#[tauri::command]
pub async fn start_github_login() -> CmdResult<fjord_vcs::DeviceLogin> {
    off_thread(fjord_vcs::start_github_login).await
}

/// Polls the sign-in once; the UI calls this every `interval` seconds.
#[tauri::command]
pub async fn poll_github_login() -> CmdResult<fjord_vcs::DevicePoll> {
    off_thread(fjord_vcs::poll_github_login).await
}

/// Adds an Azure DevOps organization using the Azure CLI's sign-in (no token).
#[tauri::command]
pub async fn connect_azure_cli(org: String) -> CmdResult<AzureAccount> {
    off_thread(move || fjord_vcs::connect_azure_cli(&org)).await
}

/// Branches a task can be linked to (local first, then remote-only).
#[tauri::command]
pub async fn list_branches(state: State<'_, AppState>, project_id: i64) -> CmdResult<Vec<String>> {
    with_store_bg(&state, move |s| fjord_vcs::branch_choices(s, project_id)).await
}

/// Links a task to an existing branch, or unlinks it with `branch: null`.
#[tauri::command]
pub async fn link_task_branch(
    state: State<'_, AppState>,
    task_id: i64,
    branch: Option<String>,
) -> CmdResult<Task> {
    with_store_bg(&state, move |s| {
        fjord_vcs::link_branch(s, task_id, branch.as_deref())
    })
    .await
}

/// The Azure DevOps Discussion of an imported task's work item (None if the task
/// wasn't imported from Azure). Fetched off the UI thread; read-only.
#[tauri::command]
pub async fn azure_discussion(
    state: State<'_, AppState>,
    task_id: i64,
) -> CmdResult<Option<Vec<fjord_vcs::WorkItemComment>>> {
    let link = with_store(&state, |s| Ok(s.external_link_for_task(task_id)?))?;
    let Some((org, project, id)) = link
        .filter(|l| l.source == "azure")
        .and_then(|l| fjord_vcs::parse_work_item_url(&l.url))
    else {
        return Ok(None);
    };
    off_thread(move || fjord_vcs::work_item_comments(&org, &project, id).map(Some)).await
}

/// Where an imported task came from (to show its link), or None.
#[tauri::command]
pub fn task_external_link(
    state: State<AppState>,
    task_id: i64,
) -> CmdResult<Option<fjord_core::ExternalLink>> {
    with_store(&state, |s| Ok(s.external_link_for_task(task_id)?))
}
