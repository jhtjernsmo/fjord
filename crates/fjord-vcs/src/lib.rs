//! Git & GitHub integration for Fjord: link a project to a repository, start
//! branches from tasks, list pull requests with CI state, open PRs, and move
//! tasks along the board as their PRs get merged.

pub mod azure;
pub mod credentials;
pub mod git;
pub mod github;
mod process;

use std::collections::HashMap;
use std::path::Path;

use fjord_core::{ImportOutcome, ImportSettings, ProjectRepo, RemoteHost, Store, Task};
use serde::Serialize;
use thiserror::Error;

pub use azure::{
    AzureAccount, AzureDevOps, WorkItem, WorkItemInfo, assigned_work_items, azure_account,
    connect_azure, connect_azure_cli, disconnect_azure, parse_azure_remote, work_item_info,
};
pub use git::{Branch, Commit, Git, branch_name_for_task, task_id_from_branch};
pub use github::{
    Checks, DeviceLogin, DevicePoll, GitHub, GitHubAccount, PullRequest, TokenSource,
    connect_github, disconnect_github, find_token, github_account, parse_github_remote,
    poll_github_login, start_github_login,
};

const RECENT_COMMITS: usize = 15;

#[derive(Debug, Error)]
pub enum VcsError {
    #[error("{0} is not inside a git repository")]
    NotARepo(String),
    #[error("git is not installed or could not run: {0}")]
    GitMissing(String),
    #[error("git: {0}")]
    Git(String),
    #[error("invalid branch name «{0}»")]
    InvalidBranch(String),
    #[error("GitHub is not connected: connect it in Settings (or run `gh auth login`)")]
    NoToken,
    #[error("credential store: {0}")]
    Credentials(String),
    #[error("GitHub: {0}")]
    GitHub(String),
    #[error("Azure DevOps is not connected for «{0}»: add a token in Settings (or run `az login`)")]
    NoAzureToken(String),
    #[error("Azure DevOps: {0}")]
    Azure(String),
    #[error("this project is not linked to a git repository")]
    NotLinked,
    #[error("the linked repository's origin is not on GitHub or Azure DevOps")]
    NoGitHub,
    #[error("task {0} has no branch yet; start one first")]
    NoBranch(i64),
    #[error(transparent)]
    Core(#[from] fjord_core::Error),
}

pub type Result<T> = std::result::Result<T, VcsError>;

#[derive(Debug, Serialize)]
pub struct GitOverview {
    pub repo: ProjectRepo,
    pub current_branch: String,
    pub dirty: bool,
    pub branches: Vec<Branch>,
    pub commits: Vec<Commit>,
}

#[derive(Debug, Serialize)]
pub struct LinkedPullRequest {
    #[serde(flatten)]
    pub pr: PullRequest,
    /// The Fjord task working in this PR's branch, if any.
    pub task_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct StartedBranch {
    pub task: Task,
    pub branch: String,
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncReport {
    pub pull_requests: Vec<LinkedPullRequest>,
    /// Tasks moved to done because their PR was merged.
    pub completed: Vec<Task>,
}

/// GitHub or Azure DevOps, from a remote URL.
pub fn detect_host(url: &str) -> Option<RemoteHost> {
    if let Some((owner, repo)) = parse_github_remote(url) {
        return Some(RemoteHost::GitHub { owner, repo });
    }
    parse_azure_remote(url).map(|(org, project, repo)| RemoteHost::AzureDevOps {
        org,
        project,
        repo,
    })
}

/// Links a project to the repository containing `path` and detects where its
/// pull requests live (GitHub or Azure DevOps).
pub fn link_repo(store: &mut Store, project_id: i64, path: &Path) -> Result<ProjectRepo> {
    let git = Git::open(path)?;
    let host = git.remote_url("origin").and_then(|u| detect_host(&u));
    let root = git.root().display().to_string();
    Ok(store.set_project_repo(project_id, &root, host.as_ref())?)
}

fn linked(store: &Store, project_id: i64) -> Result<(ProjectRepo, Git)> {
    let repo = store
        .get_project_repo(project_id)?
        .ok_or(VcsError::NotLinked)?;
    let git = Git::open(Path::new(&repo.path))?;
    Ok((repo, git))
}

/// The service hosting a linked repository's pull requests.
enum Forge {
    GitHub(GitHub),
    Azure(AzureDevOps),
}

impl Forge {
    fn for_repo(repo: &ProjectRepo) -> Result<Self> {
        if let (Some(owner), Some(name)) = (&repo.github_owner, &repo.github_repo) {
            return Ok(Forge::GitHub(GitHub::new(owner, name, find_token())));
        }
        if let (Some(org), Some(project), Some(name)) =
            (&repo.azure_org, &repo.azure_project, &repo.azure_repo)
        {
            return Ok(Forge::Azure(AzureDevOps::new(org, project, name)));
        }
        Err(VcsError::NoGitHub)
    }

    fn require_token(&self) -> Result<()> {
        match self {
            Forge::GitHub(gh) if !gh.has_token() => Err(VcsError::NoToken),
            Forge::Azure(az) if !az.has_token() => Err(VcsError::NoAzureToken(az.org().into())),
            _ => Ok(()),
        }
    }

    fn pull_requests(&self) -> Result<Vec<PullRequest>> {
        match self {
            Forge::GitHub(gh) => gh.pull_requests(),
            Forge::Azure(az) => az.pull_requests(),
        }
    }

    fn default_branch(&self) -> Result<String> {
        match self {
            Forge::GitHub(gh) => gh.default_branch(),
            Forge::Azure(az) => az.default_branch(),
        }
    }

    fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<PullRequest> {
        match self {
            Forge::GitHub(gh) => gh.create_pull_request(head, base, title, body, draft),
            Forge::Azure(az) => az.create_pull_request(head, base, title, body, draft),
        }
    }
}

pub fn overview(store: &Store, project_id: i64) -> Result<GitOverview> {
    let repo = store
        .get_project_repo(project_id)?
        .ok_or(VcsError::NotLinked)?;
    overview_for(repo)
}

/// Git only; doesn't touch the store.
pub fn overview_for(repo: ProjectRepo) -> Result<GitOverview> {
    let git = Git::open(Path::new(&repo.path))?;
    Ok(GitOverview {
        current_branch: git.current_branch()?,
        dirty: git.has_uncommitted_changes()?,
        branches: git.branches()?,
        commits: git.commits(RECENT_COMMITS)?,
        repo,
    })
}

/// Checks out the task's branch (creating `fjord/<id>-<slug>` the first time)
/// and, with auto-move on, moves a task from the first column to the second.
pub fn start_branch(store: &mut Store, task_id: i64) -> Result<StartedBranch> {
    let task = store.get_task(task_id)?;
    let (repo, git) = linked(store, task.project_id)?;
    let branch = task
        .branch
        .clone()
        .unwrap_or_else(|| branch_name_for_task(task.id, &task.title));
    let created = git.switch_or_create(&branch)?;
    let mut task = store.set_task_branch(task.id, Some(&branch))?;
    if repo.auto_move {
        let columns = store.list_statuses(task.project_id)?;
        if columns.len() > 2 && columns[0].id == task.status_id {
            task = store.move_task(task.id, columns[1].id, None)?;
        }
    }
    Ok(StartedBranch {
        task,
        branch,
        created,
    })
}

/// Branches a task can be linked to: local ones, then ones only on `origin`.
pub fn branch_choices(store: &Store, project_id: i64) -> Result<Vec<String>> {
    let (_, git) = linked(store, project_id)?;
    git.branch_names()
}

/// Links a task to an existing branch (or unlinks it with `None`), so pull
/// requests from that branch show up on the task. Nothing is checked out.
pub fn link_branch(store: &mut Store, task_id: i64, branch: Option<&str>) -> Result<Task> {
    let task = store.get_task(task_id)?;
    let Some(name) = branch.map(str::trim).filter(|b| !b.is_empty()) else {
        return Ok(store.set_task_branch(task.id, None)?);
    };
    let (_, git) = linked(store, task.project_id)?;
    if !git.branch_names()?.iter().any(|b| b == name) {
        return Err(VcsError::Git(format!(
            "no branch named «{name}» in this repository"
        )));
    }
    if let Some(other) = store
        .list_tasks(task.project_id)?
        .into_iter()
        .find(|t| t.id != task.id && t.branch.as_deref() == Some(name))
    {
        return Err(VcsError::Git(format!(
            "«{name}» is already linked to task #{} ({})",
            other.id, other.title
        )));
    }
    Ok(store.set_task_branch(task.id, Some(name))?)
}

fn link_tasks(
    store: &Store,
    project_id: i64,
    prs: Vec<PullRequest>,
) -> Result<Vec<LinkedPullRequest>> {
    let tasks = store.list_tasks(project_id)?;
    Ok(prs
        .into_iter()
        .map(|pr| {
            let task_id = tasks
                .iter()
                .find(|t| t.branch.as_deref() == Some(pr.head.as_str()))
                .or_else(|| {
                    task_id_from_branch(&pr.head).and_then(|id| tasks.iter().find(|t| t.id == id))
                })
                .map(|t| t.id);
            LinkedPullRequest { pr, task_id }
        })
        .collect())
}

/// Network only: recent pull requests of a linked repo. Doesn't touch the
/// store, so callers can run it without holding a database lock.
pub fn fetch_pull_requests(repo: &ProjectRepo) -> Result<Vec<PullRequest>> {
    Forge::for_repo(repo)?.pull_requests()
}

/// Fetches pull requests and applies them (see [`apply_sync`]).
pub fn sync(store: &mut Store, project_id: i64) -> Result<SyncReport> {
    let repo = store
        .get_project_repo(project_id)?
        .ok_or(VcsError::NotLinked)?;
    let prs = fetch_pull_requests(&repo)?;
    apply_sync(store, project_id, prs)
}

/// Links pull requests to tasks and, with auto-move on, moves tasks whose PR
/// was merged into the project's done column.
pub fn apply_sync(store: &mut Store, project_id: i64, prs: Vec<PullRequest>) -> Result<SyncReport> {
    let repo = store
        .get_project_repo(project_id)?
        .ok_or(VcsError::NotLinked)?;
    let linked = link_tasks(store, project_id, prs)?;
    let mut completed = Vec::new();
    if repo.auto_move {
        let columns = store.list_statuses(project_id)?;
        if let Some(done) = columns.iter().find(|c| c.is_done) {
            for lp in linked.iter().filter(|lp| lp.pr.state == "merged") {
                let Some(task_id) = lp.task_id else { continue };
                // Already in any done column (e.g. "Resolved"): leave it there.
                let status = store.get_task(task_id)?.status_id;
                if !columns.iter().any(|c| c.id == status && c.is_done) {
                    completed.push(store.move_task(task_id, done.id, None)?);
                }
            }
        }
    }
    Ok(SyncReport {
        pull_requests: linked,
        completed,
    })
}

/// Pushes the task's branch and opens a pull request for it.
pub fn open_pull_request(store: &Store, task_id: i64, draft: bool) -> Result<PullRequest> {
    let task = store.get_task(task_id)?;
    let repo = store
        .get_project_repo(task.project_id)?
        .ok_or(VcsError::NotLinked)?;
    open_pull_request_for(&task, &repo, draft)
}

/// Network + git push only; doesn't touch the store.
pub fn open_pull_request_for(task: &Task, repo: &ProjectRepo, draft: bool) -> Result<PullRequest> {
    let branch = task.branch.clone().ok_or(VcsError::NoBranch(task.id))?;
    let git = Git::open(Path::new(&repo.path))?;
    let forge = Forge::for_repo(repo)?;
    forge.require_token()?;
    git.push_upstream(&branch)?;
    let base = forge.default_branch()?;
    forge.create_pull_request(&branch, &base, &task.title, &pr_body(task), draft)
}

/// Result of importing Azure Boards work items.
#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    pub created: Vec<Task>,
    pub updated: usize,
    pub unchanged: usize,
    /// Work items in Azure projects that aren't mapped to a Fjord project.
    pub unmapped: usize,
    /// Tasks moved under (or out from) a parent to match Azure's hierarchy.
    pub regrouped: usize,
    /// Imported tasks moved to done because their work item was closed in Azure.
    pub closed: usize,
}

/// Everything the import needs from Azure, fetched without touching the store.
#[derive(Debug, Default)]
pub struct ImportFetch {
    pub items: Vec<WorkItem>,
    /// Parent and state of other relevant work items: ancestors of the items
    /// above, and previously imported items that are no longer assigned/open.
    pub info: HashMap<(String, u64), WorkItemInfo>,
}

/// How far up the hierarchy to look for an imported ancestor (Task → Story →
/// Feature → Epic is three steps).
const MAX_HIERARCHY_DEPTH: usize = 4;

fn split_external_id(id: &str) -> Option<(String, u64)> {
    let (org, num) = id.rsplit_once('/')?;
    Some((org.to_string(), num.parse().ok()?))
}

/// Network only: open work items assigned to you for every organization in the
/// mappings, plus what's needed to nest them and to notice closed ones.
/// `known` are external ids already imported (`org/id`).
pub fn fetch_import(settings: &ImportSettings, known: &[String]) -> Result<ImportFetch> {
    let mut orgs: Vec<String> = settings
        .mappings
        .iter()
        .map(|m| m.org.to_lowercase())
        .collect();
    orgs.sort();
    orgs.dedup();
    let mut fetch = ImportFetch::default();
    for org in orgs {
        let items = assigned_work_items(&org)?;
        let ids: std::collections::HashSet<u64> = items.iter().map(|i| i.id).collect();
        // Walk up from parents that aren't assigned to you, to find an ancestor that is.
        let mut seen = ids.clone();
        let mut frontier: Vec<u64> = items
            .iter()
            .filter_map(|i| i.parent)
            .filter(|p| seen.insert(*p))
            .collect();
        for _ in 0..MAX_HIERARCHY_DEPTH {
            if frontier.is_empty() {
                break;
            }
            let info = work_item_info(&org, &frontier)?;
            frontier = info
                .values()
                .filter_map(|i| i.parent)
                .filter(|p| seen.insert(*p))
                .collect();
            fetch
                .info
                .extend(info.into_iter().map(|(id, i)| ((org.clone(), id), i)));
        }
        // Previously imported items that dropped out (closed, or reassigned).
        let gone: Vec<u64> = known
            .iter()
            .filter_map(|k| split_external_id(k))
            .filter(|(o, id)| {
                *o == org && !ids.contains(id) && !fetch.info.contains_key(&(org.clone(), *id))
            })
            .map(|(_, id)| id)
            .collect();
        if !gone.is_empty() {
            let info = work_item_info(&org, &gone)?;
            fetch
                .info
                .extend(info.into_iter().map(|(id, i)| ((org.clone(), id), i)));
        }
        fetch.items.extend(items);
    }
    Ok(fetch)
}

/// Network only, without hierarchy: open work items assigned to you.
pub fn fetch_assigned_work_items(settings: &ImportSettings) -> Result<Vec<WorkItem>> {
    Ok(fetch_import(settings, &[])?.items)
}

/// The topmost ancestor of `item` that is also being imported, if any.
fn imported_root(
    item: &WorkItem,
    fetch: &ImportFetch,
    imported: &HashMap<(String, u64), &WorkItem>,
) -> Option<u64> {
    let org = item.org.to_lowercase();
    let parent_of = |id: u64| {
        imported
            .get(&(org.clone(), id))
            .map(|w| w.parent)
            .or_else(|| fetch.info.get(&(org.clone(), id)).map(|i| i.parent))
            .flatten()
    };
    let mut root = None;
    let mut next = item.parent;
    for _ in 0..=MAX_HIERARCHY_DEPTH {
        let Some(id) = next else { break };
        if imported.contains_key(&(org.clone(), id)) {
            root = Some(id);
        }
        next = parent_of(id);
    }
    root
}

/// Creates or refreshes a Fjord task for each mapped work item, nests tasks under
/// their topmost imported ancestor (Fjord has one level of subtasks), and moves
/// tasks to done when their work item was closed in Azure.
pub fn apply_import(
    store: &mut Store,
    settings: &ImportSettings,
    fetch: &ImportFetch,
) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    let mut task_of: HashMap<(String, u64), Task> = HashMap::new();
    for item in &fetch.items {
        let target = settings.mappings.iter().find(|m| {
            m.org.eq_ignore_ascii_case(&item.org) && m.project.eq_ignore_ascii_case(&item.project)
        });
        let Some(mapping) = target else {
            report.unmapped += 1;
            continue;
        };
        let (task, outcome) =
            store.upsert_imported_task(mapping.fjord_project_id, &item.to_external_item())?;
        match outcome {
            ImportOutcome::Created => report.created.push(task.clone()),
            ImportOutcome::Updated => report.updated += 1,
            ImportOutcome::Unchanged => report.unchanged += 1,
        }
        task_of.insert((item.org.to_lowercase(), item.id), task);
    }

    // Match Azure's hierarchy.
    let imported: HashMap<(String, u64), &WorkItem> = fetch
        .items
        .iter()
        .filter(|i| task_of.contains_key(&(i.org.to_lowercase(), i.id)))
        .map(|i| ((i.org.to_lowercase(), i.id), i))
        .collect();
    let mut wanted: Vec<(i64, Option<i64>)> = Vec::new();
    for (key, item) in &imported {
        let task = &task_of[key];
        let parent_task = imported_root(item, fetch, &imported)
            .and_then(|root| task_of.get(&(key.0.clone(), root)))
            .filter(|p| p.project_id == task.project_id)
            .map(|p| p.id);
        if task.parent_id != parent_task {
            wanted.push((task.id, parent_task));
        }
    }
    // Release every task that changes place first, so a task that loses its
    // subtasks can itself be nested, then nest.
    for (task_id, _) in &wanted {
        if store.get_task(*task_id)?.parent_id.is_some() {
            store.set_task_parent(*task_id, None)?;
        }
    }
    for (task_id, parent) in wanted {
        if parent.is_none() || store.set_task_parent(task_id, parent).is_ok() {
            report.regrouped += 1;
        }
    }

    // Work items closed in Azure since they were imported.
    for (external_id, task_id) in store.external_ids("azure")? {
        let Some(key) = split_external_id(&external_id) else {
            continue;
        };
        let Some(info) = fetch.info.get(&key) else {
            continue;
        };
        if task_of.contains_key(&key) || !azure::CLOSED_STATES.contains(&info.state.as_str()) {
            continue;
        }
        let task = store.get_task(task_id)?;
        let columns = store.list_statuses(task.project_id)?;
        if task.archived_at.is_some() {
            continue;
        }
        // A column named like the Azure state wins ("Resolved" → Resolved, "Done" → Done).
        // Without one, the first done column, unless the task is already in a done column.
        let target = match columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&info.state))
        {
            Some(same_name) => same_name,
            None if columns.iter().any(|c| c.id == task.status_id && c.is_done) => continue,
            None => match columns.iter().find(|c| c.is_done) {
                Some(done) => done,
                None => continue,
            },
        };
        if target.id != task.status_id {
            store.move_task(task.id, target.id, None)?;
            report.closed += 1;
        }
    }
    Ok(report)
}

fn pr_body(task: &Task) -> String {
    let description = task.body_md.trim();
    let mut body = String::new();
    if !description.is_empty() {
        body.push_str(description);
        body.push_str("\n\n");
    }
    body.push_str(&format!(
        "---\nFjord task #{} · branch `{}`",
        task.id,
        task.branch.as_deref().unwrap_or("")
    ));
    body
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use fjord_core::{NewProject, NewTask};

    use super::*;

    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(dir.path())
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            )
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "init"]);
        run(&[
            "remote",
            "add",
            "origin",
            "git@github.com:jhtjernsmo/demo.git",
        ]);
        dir
    }

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (Store::open(dir.path(), "tester").unwrap(), dir)
    }

    #[test]
    fn link_start_branch_and_overview() {
        let repo = temp_repo();
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "Fix push!".into(),
                ..Default::default()
            })
            .unwrap();

        assert!(matches!(
            start_branch(&mut s, t.id),
            Err(VcsError::NotLinked)
        ));
        let linked = link_repo(&mut s, p, repo.path()).unwrap();
        assert_eq!(
            (
                linked.github_owner.as_deref(),
                linked.github_repo.as_deref()
            ),
            (Some("jhtjernsmo"), Some("demo"))
        );

        let started = start_branch(&mut s, t.id).unwrap();
        assert_eq!(started.branch, format!("fjord/{}-fix-push", t.id));
        assert!(started.created);
        let in_progress = s.list_statuses(p).unwrap()[1].id;
        assert_eq!(
            started.task.status_id, in_progress,
            "auto-moved to the second column"
        );

        let again = start_branch(&mut s, t.id).unwrap();
        assert!(!again.created);

        let ov = overview(&s, p).unwrap();
        assert_eq!(ov.current_branch, started.branch);
        assert!(ov.branches.iter().any(|b| b.name == "main"));
        assert_eq!(ov.commits[0].subject, "init");
        assert!(!ov.dirty);
    }

    #[test]
    fn link_detects_azure_devops_remotes() {
        let repo = temp_repo();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .args([
                    "remote",
                    "set-url",
                    "origin",
                    "https://contoso@dev.azure.com/contoso/Mobile%20App/_git/bokost"
                ])
                .status()
                .unwrap()
                .success()
        );
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let linked = link_repo(&mut s, p, repo.path()).unwrap();
        assert_eq!(linked.github_owner, None);
        assert_eq!(
            (
                linked.azure_org.as_deref(),
                linked.azure_project.as_deref(),
                linked.azure_repo.as_deref()
            ),
            (Some("contoso"), Some("Mobile App"), Some("bokost"))
        );
        assert!(matches!(Forge::for_repo(&linked), Ok(Forge::Azure(_))));
    }

    #[test]
    fn imports_only_mapped_work_items() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Bokost".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let settings = ImportSettings {
            enabled: true,
            mappings: vec![fjord_core::ImportMapping {
                org: "Contoso".into(),
                project: "mobile app".into(),
                fjord_project_id: p,
            }],
        };
        let wi = |id: u64, project: &str| WorkItem {
            org: "contoso".into(),
            id,
            rev: 1,
            project: project.into(),
            kind: "Task".into(),
            state: "Active".into(),
            area: project.into(),
            title: format!("Item {id}"),
            description_md: String::new(),
            acceptance_md: String::new(),
            priority: Some(2),
            due: None,
            parent: None,
        };
        let items = [wi(1, "Mobile App"), wi(2, "Other team")];
        let fetch = ImportFetch {
            items: items.to_vec(),
            ..Default::default()
        };
        let report = apply_import(&mut s, &settings, &fetch).unwrap();
        assert_eq!((report.created.len(), report.unmapped), (1, 1));
        assert_eq!(report.created[0].title, "Item 1");
        let again = apply_import(&mut s, &settings, &fetch).unwrap();
        assert_eq!((again.created.len(), again.unchanged), (0, 1));
    }

    #[test]
    fn imported_work_items_follow_the_azure_hierarchy_and_closures() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Bokost".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let settings = ImportSettings {
            enabled: true,
            mappings: vec![fjord_core::ImportMapping {
                org: "contoso".into(),
                project: "App".into(),
                fjord_project_id: p,
            }],
        };
        let wi = |id: u64, parent: Option<u64>| WorkItem {
            org: "contoso".into(),
            id,
            rev: 1,
            project: "App".into(),
            kind: "Task".into(),
            state: "Active".into(),
            area: "App".into(),
            title: format!("Item {id}"),
            description_md: String::new(),
            acceptance_md: String::new(),
            priority: None,
            due: None,
            parent,
        };
        let info = |parent: Option<u64>, state: &str| WorkItemInfo {
            parent,
            state: state.into(),
        };

        // Without knowing Feature 2's parent, Task 4 nests under Story 3 (both mine).
        let flat = ImportFetch {
            items: vec![wi(1, None), wi(3, Some(2)), wi(4, Some(3)), wi(5, None)],
            ..Default::default()
        };
        apply_import(&mut s, &settings, &flat).unwrap();
        let link = |s: &Store, n: u64| {
            s.external_link("azure", &format!("contoso/{n}"))
                .unwrap()
                .unwrap()
                .task_id
        };
        let (story, task4) = (
            s.get_task(link(&s, 3)).unwrap(),
            s.get_task(link(&s, 4)).unwrap(),
        );
        assert_eq!((story.parent_id, task4.parent_id), (None, Some(story.id)));

        // Epic 1 (mine) → Feature 2 (not mine) → Story 3 (mine) → Task 4 (mine):
        // 3 and 4 both become subtasks of 1, Fjord's single level.
        let mut tree = ImportFetch {
            items: vec![wi(1, None), wi(3, Some(2)), wi(4, Some(3)), wi(5, None)],
            ..Default::default()
        };
        tree.info
            .insert(("contoso".into(), 2), info(Some(1), "Active"));
        let report = apply_import(&mut s, &settings, &tree).unwrap();
        let task = |s: &Store, n: u64| {
            s.get_task(
                s.external_link("azure", &format!("contoso/{n}"))
                    .unwrap()
                    .unwrap()
                    .task_id,
            )
            .unwrap()
        };
        let epic = task(&s, 1);
        assert_eq!(
            (task(&s, 3).parent_id, task(&s, 4).parent_id),
            (Some(epic.id), Some(epic.id))
        );
        assert_eq!(report.regrouped, 2);

        // Item 5 was closed in Azure: it drops out of "assigned to me" and is checked off.
        let mut later = ImportFetch {
            items: vec![wi(1, None), wi(3, Some(2)), wi(4, Some(3))],
            info: tree.info.clone(),
        };
        later
            .info
            .insert(("contoso".into(), 5), info(None, "Closed"));
        let report = apply_import(&mut s, &settings, &later).unwrap();
        let done = s
            .list_statuses(p)
            .unwrap()
            .into_iter()
            .find(|st| st.is_done)
            .unwrap()
            .id;
        assert_eq!((report.closed, task(&s, 5).status_id), (1, done));
        assert_eq!(
            apply_import(&mut s, &settings, &later).unwrap().closed,
            0,
            "only once"
        );
    }

    #[test]
    fn link_existing_branch_to_a_task() {
        let repo = temp_repo();
        let run = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(repo.path())
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            )
        };
        run(&["branch", "feature/login"]);
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let a = s
            .create_task(NewTask {
                project_id: p,
                title: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = s
            .create_task(NewTask {
                project_id: p,
                title: "B".into(),
                ..Default::default()
            })
            .unwrap();
        link_repo(&mut s, p, repo.path()).unwrap();

        assert!(
            branch_choices(&s, p)
                .unwrap()
                .contains(&"feature/login".to_string())
        );
        assert_eq!(
            link_branch(&mut s, a.id, Some("feature/login"))
                .unwrap()
                .branch
                .as_deref(),
            Some("feature/login")
        );
        assert!(
            matches!(
                link_branch(&mut s, a.id, Some("nope")),
                Err(VcsError::Git(_))
            ),
            "unknown branch"
        );
        assert!(
            matches!(
                link_branch(&mut s, b.id, Some("feature/login")),
                Err(VcsError::Git(_))
            ),
            "already linked elsewhere"
        );
        assert_eq!(link_branch(&mut s, a.id, None).unwrap().branch, None);
        assert_eq!(
            link_branch(&mut s, b.id, Some("feature/login"))
                .unwrap()
                .branch
                .as_deref(),
            Some("feature/login")
        );
    }

    #[test]
    fn auto_moves_leave_tasks_in_any_done_column() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let resolved = s.create_status(p, "Resolved", None, false).unwrap();
        assert!(resolved.is_done);
        let done = s
            .list_statuses(p)
            .unwrap()
            .into_iter()
            .find(|c| c.is_done && c.name != "Resolved")
            .unwrap();

        // Azure: a work item resolved there lands in the column of the same name, and stays.
        let settings = ImportSettings {
            enabled: true,
            mappings: vec![fjord_core::ImportMapping {
                org: "contoso".into(),
                project: "App".into(),
                fjord_project_id: p,
            }],
        };
        let item = WorkItem {
            org: "contoso".into(),
            id: 9,
            rev: 1,
            project: "App".into(),
            kind: "Bug".into(),
            state: "Active".into(),
            area: "App".into(),
            title: "Crash".into(),
            description_md: String::new(),
            acceptance_md: String::new(),
            priority: None,
            due: None,
            parent: None,
        };
        apply_import(
            &mut s,
            &settings,
            &ImportFetch {
                items: vec![item],
                ..Default::default()
            },
        )
        .unwrap();
        let mut gone = ImportFetch::default();
        gone.info.insert(
            ("contoso".into(), 9),
            WorkItemInfo {
                parent: None,
                state: "Resolved".into(),
            },
        );
        apply_import(&mut s, &settings, &gone).unwrap();
        let id = s
            .external_link("azure", "contoso/9")
            .unwrap()
            .unwrap()
            .task_id;
        assert_eq!(
            s.get_task(id).unwrap().status_id,
            resolved.id,
            "moved to the same-named column, not Done"
        );
        assert_eq!(apply_import(&mut s, &settings, &gone).unwrap().closed, 0);
        // Later set to Done in Azure: follows to the Done column.
        gone.info.insert(
            ("contoso".into(), 9),
            WorkItemInfo {
                parent: None,
                state: "Done".into(),
            },
        );
        apply_import(&mut s, &settings, &gone).unwrap();
        assert_eq!(s.get_task(id).unwrap().status_id, done.id);
        // Back to Resolved for the Git part below.
        s.move_task(id, resolved.id, None).unwrap();
        gone.info.insert(
            ("contoso".into(), 9),
            WorkItemInfo {
                parent: None,
                state: "Resolved".into(),
            },
        );

        // Git: a merged PR doesn't pull a task out of "Resolved" into "Done".
        let repo = temp_repo();
        link_repo(&mut s, p, repo.path()).unwrap();
        let t = s.set_task_branch(id, Some("fix/crash")).unwrap();
        let pr = PullRequest {
            number: 1,
            title: "t".into(),
            state: "merged".into(),
            draft: false,
            url: String::new(),
            author: "j".into(),
            head: "fix/crash".into(),
            base: "main".into(),
            head_sha: "s".into(),
            updated_at: String::new(),
            checks: Checks::None,
        };
        let report = apply_sync(&mut s, p, vec![pr]).unwrap();
        assert!(report.completed.is_empty());
        assert_eq!(s.get_task(t.id).unwrap().status_id, resolved.id);
        assert_ne!(resolved.id, done.id);
    }

    #[test]
    fn auto_move_can_be_turned_off() {
        let repo = temp_repo();
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "X".into(),
                ..Default::default()
            })
            .unwrap();
        link_repo(&mut s, p, repo.path()).unwrap();
        s.set_repo_auto_move(p, false).unwrap();
        assert_eq!(
            start_branch(&mut s, t.id).unwrap().task.status_id,
            t.status_id
        );
    }

    #[test]
    fn pull_requests_are_linked_to_tasks_by_branch() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let a = s
            .create_task(NewTask {
                project_id: p,
                title: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = s
            .create_task(NewTask {
                project_id: p,
                title: "B".into(),
                ..Default::default()
            })
            .unwrap();
        s.set_task_branch(a.id, Some("custom-branch")).unwrap();
        let pr = |head: &str| PullRequest {
            number: 1,
            title: "t".into(),
            state: "open".into(),
            draft: false,
            url: String::new(),
            author: "j".into(),
            head: head.into(),
            base: "main".into(),
            head_sha: "s".into(),
            updated_at: String::new(),
            checks: Checks::None,
        };
        let heads = vec![
            pr("custom-branch"),
            pr(&format!("fjord/{}-b", b.id)),
            pr("other"),
            pr("fjord/999-x"),
        ];
        let ids: Vec<_> = link_tasks(&s, p, heads)
            .unwrap()
            .iter()
            .map(|l| l.task_id)
            .collect();
        assert_eq!(ids, [Some(a.id), Some(b.id), None, None]);
    }

    #[test]
    fn pr_body_includes_description_and_reference() {
        let (mut s, _d) = store();
        let p = s
            .create_project(NewProject {
                name: "Demo".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "A".into(),
                body_md: "Details".into(),
                ..Default::default()
            })
            .unwrap();
        let t = s.set_task_branch(t.id, Some("fjord/1-a")).unwrap();
        let body = pr_body(&t);
        assert!(body.starts_with("Details\n\n---\nFjord task #"));
        assert!(body.contains("`fjord/1-a`"));
    }
}
