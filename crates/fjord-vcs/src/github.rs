//! Minimal GitHub REST client: pull requests and their CI checks.
//! Auth comes from $GH_TOKEN / $GITHUB_TOKEN, a token saved in Settings (kept in the
//! OS credential store) or the GitHub CLI (`gh auth token`);
//! public repositories can also be read without a token.

use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Result, VcsError, credentials};

const API: &str = "https://api.github.com";
const TIMEOUT: Duration = Duration::from_secs(20);
const PER_PAGE: u32 = 30;

/// Combined CI state of a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Checks {
    None,
    Pending,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    /// "open", "closed" or "merged"
    pub state: String,
    pub draft: bool,
    pub url: String,
    pub author: String,
    pub head: String,
    pub base: String,
    pub head_sha: String,
    pub updated_at: String,
    pub checks: Checks,
}

#[derive(Deserialize)]
struct ApiUser {
    login: String,
}

#[derive(Deserialize)]
struct ApiRef {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
}

#[derive(Deserialize)]
struct ApiPull {
    number: u64,
    title: String,
    state: String,
    #[serde(default)]
    draft: bool,
    merged_at: Option<String>,
    html_url: String,
    user: ApiUser,
    head: ApiRef,
    base: ApiRef,
    updated_at: String,
}

#[derive(Deserialize)]
struct ApiCheckRun {
    status: String,
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct ApiCheckRuns {
    check_runs: Vec<ApiCheckRun>,
}

#[derive(Deserialize)]
struct ApiRepo {
    default_branch: String,
}

impl From<ApiPull> for PullRequest {
    fn from(p: ApiPull) -> Self {
        let state = if p.merged_at.is_some() {
            "merged".to_string()
        } else {
            p.state
        };
        PullRequest {
            number: p.number,
            title: p.title,
            state,
            draft: p.draft,
            url: p.html_url,
            author: p.user.login,
            head: p.head.name,
            base: p.base.name,
            head_sha: p.head.sha,
            updated_at: p.updated_at,
            checks: Checks::None,
        }
    }
}

/// Folds individual check runs into one state: any failure wins, then pending.
pub fn combine_checks<'a>(runs: impl IntoIterator<Item = (&'a str, Option<&'a str>)>) -> Checks {
    let mut seen = false;
    let mut pending = false;
    for (status, conclusion) in runs {
        seen = true;
        if status != "completed" {
            pending = true;
        } else if matches!(
            conclusion,
            Some("failure" | "cancelled" | "timed_out" | "action_required")
        ) {
            return Checks::Failure;
        }
    }
    match (seen, pending) {
        (false, _) => Checks::None,
        (true, true) => Checks::Pending,
        (true, false) => Checks::Success,
    }
}

/// ("owner", "repo") from an https or ssh GitHub remote URL.
pub fn parse_github_remote(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let path = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| {
            // https://user@github.com/owner/repo
            url.strip_prefix("https://")
                .and_then(|r| r.split_once("@github.com/").map(|(_, p)| p))
        })?;
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    let (owner, repo) = path.split_once('/')?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (valid(owner) && valid(repo)).then(|| (owner.to_string(), repo.to_string()))
}

/// Where the GitHub token came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    Env,
    Saved,
    GhCli,
    /// Azure CLI (`az login`).
    Cli,
}

/// Finds a token without ever printing or storing it.
pub fn find_token() -> Option<String> {
    find_token_with_source().map(|(t, _)| t)
}

fn find_token_with_source() -> Option<(String, TokenSource)> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Some(t) = std::env::var(var).ok().filter(|t| !t.trim().is_empty()) {
            return Some((t.trim().to_string(), TokenSource::Env));
        }
    }
    if let Some(t) = credentials::get(credentials::GITHUB) {
        return Some((t, TokenSource::Saved));
    }
    let out = Command::new("gh").args(["auth", "token"]).output().ok()?;
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !token.is_empty()).then_some((token, TokenSource::GhCli))
}

/// The signed-in GitHub user, as shown in Settings.
#[derive(Debug, Clone, Serialize)]
pub struct GitHubAccount {
    pub login: String,
    pub source: TokenSource,
    /// False when a classic token lacks the `repo` scope, so private repositories won't show.
    pub private_repos: bool,
}

pub(crate) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

fn whoami(token: &str, source: TokenSource) -> Result<GitHubAccount> {
    let mut res = agent()
        .get(&format!("{API}/user"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fjord")
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(401) => {
                VcsError::GitHub("GitHub did not accept this token (401)".into())
            }
            other => api_error(other),
        })?;
    // Classic tokens list their scopes; fine-grained tokens don't send the header.
    let private_repos = res
        .headers()
        .get("x-oauth-scopes")
        .and_then(|v| v.to_str().ok())
        .is_none_or(|scopes| scopes.split(',').any(|s| s.trim() == "repo"));
    let user: ApiUser = res
        .body_mut()
        .read_json()
        .map_err(|e| VcsError::GitHub(e.to_string()))?;
    Ok(GitHubAccount {
        login: user.login,
        source,
        private_repos,
    })
}

/// Who Fjord talks to GitHub as, or None when no token is available.
pub fn github_account() -> Result<Option<GitHubAccount>> {
    match find_token_with_source() {
        Some((token, source)) => whoami(&token, source).map(Some),
        None => Ok(None),
    }
}

/// Checks a personal access token with GitHub, then saves it in the OS credential store.
pub fn connect_github(token: &str) -> Result<GitHubAccount> {
    let token = token.trim();
    if token.is_empty() || token.chars().any(char::is_whitespace) {
        return Err(VcsError::GitHub(
            "that doesn't look like a GitHub token".into(),
        ));
    }
    let account = whoami(token, TokenSource::Saved)?;
    credentials::set(credentials::GITHUB, token)?;
    Ok(account)
}

/// Forgets the token saved in Settings (env vars and the gh CLI are left alone).
pub fn disconnect_github() -> Result<()> {
    credentials::delete(credentials::GITHUB)
}

pub struct GitHub {
    owner: String,
    repo: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl GitHub {
    pub fn new(owner: &str, repo: &str, token: Option<String>) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            token,
            agent: agent(),
        }
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    fn url(&self, path: &str) -> String {
        format!("{API}/repos/{}/{}{path}", self.owner, self.repo)
    }

    fn with_headers<B>(&self, req: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        let req = req
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "fjord");
        match &self.token {
            Some(t) => req.header("Authorization", &format!("Bearer {t}")),
            None => req,
        }
    }

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let mut res = self
            .with_headers(self.agent.get(&self.url(path)))
            .call()
            .map_err(api_error)?;
        res.body_mut()
            .read_json()
            .map_err(|e| VcsError::GitHub(e.to_string()))
    }

    pub fn default_branch(&self) -> Result<String> {
        Ok(self.get::<ApiRepo>("")?.default_branch)
    }

    /// Recent pull requests (open first), with CI state for the open ones.
    pub fn pull_requests(&self) -> Result<Vec<PullRequest>> {
        let pulls: Vec<ApiPull> = self.get(&format!(
            "/pulls?state=all&sort=updated&direction=desc&per_page={PER_PAGE}"
        ))?;
        let mut prs: Vec<PullRequest> = pulls.into_iter().map(PullRequest::from).collect();
        for pr in prs.iter_mut().filter(|p| p.state == "open") {
            pr.checks = self.checks(&pr.head_sha).unwrap_or(Checks::None);
        }
        prs.sort_by_key(|p| p.state != "open");
        Ok(prs)
    }

    pub fn checks(&self, sha: &str) -> Result<Checks> {
        let runs: ApiCheckRuns = self.get(&format!("/commits/{sha}/check-runs?per_page=100"))?;
        Ok(combine_checks(
            runs.check_runs
                .iter()
                .map(|r| (r.status.as_str(), r.conclusion.as_deref())),
        ))
    }

    pub fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<PullRequest> {
        if self.token.is_none() {
            return Err(VcsError::NoToken);
        }
        let payload =
            json!({ "title": title, "head": head, "base": base, "body": body, "draft": draft });
        let mut res = self
            .with_headers(self.agent.post(&self.url("/pulls")))
            .send_json(&payload)
            .map_err(api_error)?;
        let pr: ApiPull = res
            .body_mut()
            .read_json()
            .map_err(|e| VcsError::GitHub(e.to_string()))?;
        Ok(pr.into())
    }
}

fn api_error(e: ureq::Error) -> VcsError {
    match e {
        ureq::Error::StatusCode(401) => VcsError::NoToken,
        ureq::Error::StatusCode(403) => {
            VcsError::GitHub("access denied or rate limited (403)".into())
        }
        ureq::Error::StatusCode(404) => {
            VcsError::GitHub(
                "repository not found (404). If it is private, connect GitHub in Settings with a token that can read it"
                    .into(),
            )
        }
        ureq::Error::StatusCode(422) => VcsError::GitHub(
            "GitHub rejected the request (422): is the branch pushed, or does a PR already exist?"
                .into(),
        ),
        other => VcsError::GitHub(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_rejects_malformed_tokens_without_calling_github() {
        for bad in ["", "   ", "ghp_abc def"] {
            assert!(matches!(connect_github(bad), Err(VcsError::GitHub(_))));
        }
    }

    #[test]
    fn parses_github_remotes() {
        let want = Some(("jhtjernsmo".to_string(), "fjord".to_string()));
        for url in [
            "https://github.com/jhtjernsmo/fjord.git",
            "https://github.com/jhtjernsmo/fjord",
            "https://github.com/jhtjernsmo/fjord/",
            "git@github.com:jhtjernsmo/fjord.git",
            "ssh://git@github.com/jhtjernsmo/fjord.git",
            "https://jonas@github.com/jhtjernsmo/fjord.git",
        ] {
            assert_eq!(parse_github_remote(url), want, "{url}");
        }
        assert_eq!(parse_github_remote("https://gitlab.com/a/b.git"), None);
        assert_eq!(parse_github_remote("https://github.com/only-owner"), None);
        assert_eq!(parse_github_remote("https://github.com/a/b c"), None);
    }

    #[test]
    fn combines_check_runs() {
        assert_eq!(combine_checks([]), Checks::None);
        assert_eq!(
            combine_checks([
                ("completed", Some("success")),
                ("completed", Some("skipped"))
            ]),
            Checks::Success
        );
        assert_eq!(
            combine_checks([("completed", Some("success")), ("in_progress", None)]),
            Checks::Pending
        );
        assert_eq!(
            combine_checks([("in_progress", None), ("completed", Some("failure"))]),
            Checks::Failure
        );
        assert_eq!(
            combine_checks([("completed", Some("cancelled"))]),
            Checks::Failure
        );
    }

    #[test]
    fn merged_pull_requests_are_marked_merged() {
        let raw = r#"{"number":7,"title":"Fix push","state":"closed","draft":false,"merged_at":"2026-10-07T10:00:00Z",
            "html_url":"https://github.com/a/b/pull/7","user":{"login":"jonas"},
            "head":{"ref":"fjord/12-fix-push","sha":"abc"},"base":{"ref":"main","sha":"def"},
            "updated_at":"2026-10-07T10:00:00Z"}"#;
        let pr: PullRequest = serde_json::from_str::<ApiPull>(raw).unwrap().into();
        assert_eq!(
            (pr.state.as_str(), pr.head.as_str(), pr.base.as_str()),
            ("merged", "fjord/12-fix-push", "main")
        );
    }
}
