//! Git & GitHub integration for Fjord: link a project to a repository, start
//! branches from tasks, list pull requests with CI state, open PRs, and move
//! tasks along the board as their PRs get merged.

pub mod azure;
pub mod credentials;
pub mod git;
pub mod github;

use std::path::Path;

use fjord_core::{ImportOutcome, ImportSettings, ProjectRepo, RemoteHost, Store, Task};
use serde::Serialize;
use thiserror::Error;

pub use azure::{
    AzureAccount, AzureDevOps, WorkItem, assigned_work_items, azure_account, connect_azure,
    disconnect_azure, parse_azure_remote,
};
pub use git::{Branch, Commit, Git, branch_name_for_task, task_id_from_branch};
pub use github::{
    Checks, GitHub, GitHubAccount, PullRequest, TokenSource, connect_github, disconnect_github,
    find_token, github_account, parse_github_remote,
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
                if store.get_task(task_id)?.status_id != done.id {
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
}

/// Network only: open work items assigned to you, for every organization in the
/// mappings. Doesn't touch the store.
pub fn fetch_assigned_work_items(settings: &ImportSettings) -> Result<Vec<WorkItem>> {
    let mut orgs: Vec<String> = settings
        .mappings
        .iter()
        .map(|m| m.org.to_lowercase())
        .collect();
    orgs.sort();
    orgs.dedup();
    let mut items = Vec::new();
    for org in orgs {
        items.extend(assigned_work_items(&org)?);
    }
    Ok(items)
}

/// Creates or refreshes a Fjord task for each mapped work item.
pub fn apply_import(
    store: &mut Store,
    settings: &ImportSettings,
    items: &[WorkItem],
) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    for item in items {
        let target = settings.mappings.iter().find(|m| {
            m.org.eq_ignore_ascii_case(&item.org) && m.project.eq_ignore_ascii_case(&item.project)
        });
        let Some(mapping) = target else {
            report.unmapped += 1;
            continue;
        };
        match store.upsert_imported_task(mapping.fjord_project_id, &item.to_external_item())? {
            (task, ImportOutcome::Created) => report.created.push(task),
            (_, ImportOutcome::Updated) => report.updated += 1,
            (_, ImportOutcome::Unchanged) => report.unchanged += 1,
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
        };
        let items = [wi(1, "Mobile App"), wi(2, "Other team")];
        let report = apply_import(&mut s, &settings, &items).unwrap();
        assert_eq!((report.created.len(), report.unmapped), (1, 1));
        assert_eq!(report.created[0].title, "Item 1");
        let again = apply_import(&mut s, &settings, &items).unwrap();
        assert_eq!((again.created.len(), again.unchanged), (0, 1));
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
