//! Creating a GitHub repository for a project: a fresh one cloned into a folder,
//! or publishing a local repository that has no remote yet.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::git::{Git, run_with_github_token};
use crate::github::{
    HttpError, TokenSource, agent, api_error, checked, find_token, whoami, with_message,
};
use crate::{Result, VcsError};

const API: &str = "https://api.github.com";
const MAX_NAME: usize = 100;

/// `.gitignore` templates offered in the UI (GitHub's own template names).
pub const GITIGNORE_TEMPLATES: &[&str] = &[
    "Rust",
    "Node",
    "Python",
    "Go",
    "Java",
    "Kotlin",
    "Swift",
    "Dart",
    "C++",
    "VisualStudio",
    "Unity",
];

/// Licenses offered in the UI (GitHub license keys).
pub const LICENSES: &[&str] = &[
    "mit",
    "apache-2.0",
    "gpl-3.0",
    "bsd-3-clause",
    "mpl-2.0",
    "unlicense",
];

/// What to create on GitHub.
#[derive(Debug, Clone, Deserialize)]
pub struct NewRepo {
    /// The user's own login or an organization they belong to.
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub private: bool,
    /// Starter files GitHub adds in the first commit (ignored when publishing).
    #[serde(default)]
    pub readme: bool,
    pub gitignore: Option<String>,
    pub license: Option<String>,
}

/// An account a repository can be created under.
#[derive(Debug, Clone, Serialize)]
pub struct RepoOwner {
    pub login: String,
    pub org: bool,
}

#[derive(Deserialize)]
struct ApiOrg {
    login: String,
}

#[derive(Deserialize)]
struct ApiCreatedRepo {
    clone_url: String,
}

/// A repository name GitHub accepts: letters, digits, `.`, `-` and `_`.
pub fn validate_repo_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= MAX_NAME
        && name != "."
        && name != ".."
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if ok {
        Ok(())
    } else {
        Err(VcsError::GitHub(format!(
            "\"{name}\" isn't a valid repository name: use letters, digits, '.', '-' and '_'"
        )))
    }
}

fn check(repo: &NewRepo) -> Result<()> {
    validate_repo_name(&repo.name)?;
    let owner_ok = !repo.owner.is_empty()
        && repo
            .owner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !owner_ok {
        return Err(VcsError::GitHub(
            "choose who should own the repository".into(),
        ));
    }
    if let Some(g) = &repo.gitignore
        && !GITIGNORE_TEMPLATES.contains(&g.as_str())
    {
        return Err(VcsError::GitHub(format!(
            "unknown .gitignore template \"{g}\""
        )));
    }
    if let Some(l) = &repo.license
        && !LICENSES.contains(&l.as_str())
    {
        return Err(VcsError::GitHub(format!("unknown license \"{l}\"")));
    }
    Ok(())
}

fn token() -> Result<String> {
    find_token().ok_or(VcsError::NoToken)
}

/// The signed-in user plus the organizations they belong to.
pub fn repo_owners() -> Result<Vec<RepoOwner>> {
    let token = token()?;
    let me = whoami(&token, TokenSource::Saved)?;
    let mut res = agent()
        .get(&format!("{API}/user/orgs?per_page=100"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fjord")
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(api_error)?;
    let orgs: Vec<ApiOrg> = res
        .body_mut()
        .read_json()
        .map_err(|e| VcsError::GitHub(e.to_string()))?;
    let mut owners = vec![RepoOwner {
        login: me.login,
        org: false,
    }];
    owners.extend(orgs.into_iter().map(|o| RepoOwner {
        login: o.login,
        org: true,
    }));
    Ok(owners)
}

/// Creates the repository on GitHub and returns its HTTPS clone URL.
fn create_on_github(token: &str, repo: &NewRepo, with_files: bool) -> Result<String> {
    let me = whoami(token, TokenSource::Saved)?.login;
    let url = if repo.owner.eq_ignore_ascii_case(&me) {
        format!("{API}/user/repos")
    } else {
        format!("{API}/orgs/{}/repos", repo.owner)
    };
    let mut payload = json!({
        "name": repo.name,
        "description": repo.description.trim(),
        "private": repo.private,
        "auto_init": with_files && repo.readme,
    });
    if with_files {
        if let Some(g) = &repo.gitignore {
            payload["gitignore_template"] = json!(g);
        }
        if let Some(l) = &repo.license {
            payload["license_template"] = json!(l);
        }
    }
    let mut res = agent()
        .post(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "fjord")
        .header("Authorization", &format!("Bearer {token}"))
        .send_json(&payload)
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| match e {
            HttpError::Status(422, message) => VcsError::GitHub(with_message(
                &format!(
                    "GitHub couldn't create {}/{} (422): is the name already taken?",
                    repo.owner, repo.name
                ),
                &message,
            )),
            HttpError::Status(403 | 404, message) => VcsError::GitHub(with_message(
                &format!("you can't create repositories under {}", repo.owner),
                &message,
            )),
            other => api_error(other),
        })?;
    let created: ApiCreatedRepo = res
        .body_mut()
        .read_json()
        .map_err(|e| VcsError::GitHub(e.to_string()))?;
    Ok(created.clone_url)
}

/// Creates a new repository on GitHub and clones it to `parent/<name>`.
/// Returns the folder it was cloned into.
pub fn create_and_clone(repo: &NewRepo, parent: &Path) -> Result<PathBuf> {
    check(repo)?;
    if !parent.is_dir() {
        return Err(VcsError::Git(format!(
            "{} is not a folder",
            parent.display()
        )));
    }
    let target = parent.join(&repo.name);
    if target.exists() {
        return Err(VcsError::Git(format!(
            "{} already exists; choose another folder or name",
            target.display()
        )));
    }
    let token = token()?;
    let clone_url = create_on_github(&token, repo, true)?;
    run_with_github_token(parent, &["clone", "--", &clone_url, &repo.name], &token).map_err(
        |e| {
            VcsError::Git(format!(
                "the repository was created on GitHub, but cloning it failed: {e}"
            ))
        },
    )?;
    Ok(target)
}

/// Creates an empty repository on GitHub for the local repository at `path`,
/// adds it as `origin` and pushes the current branch. Returns the repository root.
pub fn publish(repo: &NewRepo, path: &Path) -> Result<PathBuf> {
    check(repo)?;
    let git = Git::open(path)?;
    if git.remote_url("origin").is_some() {
        return Err(VcsError::Git(
            "this repository already has an 'origin' remote; link it instead".into(),
        ));
    }
    if !git.has_commits() {
        return Err(VcsError::Git(
            "make a first commit before publishing this repository".into(),
        ));
    }
    let branch = git.current_branch()?;
    if branch.is_empty() {
        return Err(VcsError::Git(
            "check out a branch before publishing (HEAD is detached)".into(),
        ));
    }
    let token = token()?;
    let clone_url = create_on_github(&token, repo, false)?;
    let root = git.root().to_path_buf();
    git.add_remote("origin", &clone_url)?;
    run_with_github_token(
        &root,
        &["push", "--set-upstream", "origin", &branch],
        &token,
    )
    .map_err(|e| {
        VcsError::Git(format!(
            "the repository was created on GitHub, but pushing to it failed: {e}"
        ))
    })?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str) -> NewRepo {
        NewRepo {
            owner: "jonas".into(),
            name: name.into(),
            description: String::new(),
            private: true,
            readme: true,
            gitignore: Some("Rust".into()),
            license: Some("mit".into()),
        }
    }

    #[test]
    fn accepts_names_github_accepts() {
        for name in ["fjord", "core-ui", "my_app.rs", "A1"] {
            assert!(validate_repo_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_names_that_could_escape_the_folder_or_be_options() {
        let long = "x".repeat(101);
        for name in [
            "",
            ".",
            "..",
            "../x",
            "a/b",
            "-x",
            "has space",
            "æøå",
            &long,
        ] {
            assert!(validate_repo_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn only_offered_templates_and_licenses_are_allowed() {
        assert!(check(&repo("ok")).is_ok());
        let mut bad = repo("ok");
        bad.gitignore = Some("../../etc".into());
        assert!(check(&bad).is_err());
        let mut bad = repo("ok");
        bad.license = Some("wtfpl".into());
        assert!(check(&bad).is_err());
        let mut bad = repo("ok");
        bad.owner = "../user".into();
        assert!(check(&bad).is_err());
    }

    #[test]
    fn refuses_to_clone_over_an_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("taken")).unwrap();
        let err = create_and_clone(&repo("taken"), dir.path()).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
    }

    #[test]
    fn refuses_to_publish_a_repository_without_commits() {
        let dir = tempfile::tempdir().unwrap();
        crate::process::tool("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        let err = publish(&repo("x"), dir.path()).unwrap_err();
        assert!(err.to_string().contains("first commit"), "{err}");
    }
}
