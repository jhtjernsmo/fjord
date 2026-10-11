//! Pull requests waiting for you on GitHub and Azure DevOps: reviews you were asked for, your own
//! pull requests with changes requested, and your own pull requests whose checks failed. Also the
//! GitHub issues and pull requests that @mention you, for the standup draft. Read-only.

use serde::{Deserialize, Serialize};

use crate::azure::{
    API_VERSION, Auth, HOST, List, check_org, encode, find_auth, my_identity_id, read_json,
};
use crate::github::{HttpError, agent, api_error, checked, combine_checks};
use crate::{Checks, Result, VcsError};

const GITHUB_API: &str = "https://api.github.com";
/// Enough for a glance on the Overview; the full lists live on GitHub and Azure DevOps.
const MAX_ITEMS: u32 = 20;
/// Checking CI costs a request or two per pull request, so only the most recent ones are checked.
const MAX_CHECKED: usize = 10;
/// Azure DevOps reviewer votes: waiting for author, and rejected.
const VOTE_WAITING_FOR_AUTHOR: i32 = -5;
const VOTE_REJECTED: i32 = -10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaitingKind {
    /// Someone asked you to review their pull request.
    Review,
    /// A reviewer asked for changes on your pull request.
    Changes,
    /// The checks on your pull request failed.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WaitingItem {
    pub kind: WaitingKind,
    pub title: String,
    /// owner/repo on GitHub, project/repo on Azure DevOps.
    pub repo: String,
    pub number: Option<u64>,
    pub url: String,
    /// Who opened it (reviews) or who asked for changes.
    pub by: Option<String>,
    /// ISO 8601
    pub at: String,
}

/// A GitHub issue or pull request that @mentions you.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitHubMention {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    /// Who opened the issue or pull request.
    pub by: String,
    /// ISO 8601, last update.
    pub at: String,
    pub pull_request: bool,
}

// ---------- GitHub ----------

#[derive(Deserialize)]
struct SearchResult {
    items: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    number: u64,
    title: String,
    html_url: String,
    /// https://api.github.com/repos/owner/repo
    repository_url: String,
    user: SearchUser,
    updated_at: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SearchUser {
    login: String,
}

#[derive(Deserialize)]
struct PullHead {
    head: HeadRef,
}

#[derive(Deserialize)]
struct HeadRef {
    sha: String,
}

#[derive(Deserialize)]
struct CheckRuns {
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
struct CheckRun {
    status: String,
    conclusion: Option<String>,
}

fn repo_name(api_url: &str) -> String {
    api_url
        .strip_prefix(&format!("{GITHUB_API}/repos/"))
        .unwrap_or(api_url)
        .to_string()
}

fn github_item(kind: WaitingKind, item: SearchItem, by: Option<String>) -> WaitingItem {
    WaitingItem {
        kind,
        title: item.title,
        repo: repo_name(&item.repository_url),
        number: Some(item.number),
        url: item.html_url,
        by,
        at: item.updated_at,
    }
}

fn github_get<T: serde::de::DeserializeOwned>(token: &str, path: &str) -> Result<T> {
    let mut res = agent()
        .get(&format!("{GITHUB_API}{path}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "fjord")
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(api_error)?;
    res.body_mut()
        .read_json()
        .map_err(|e| VcsError::GitHub(e.to_string()))
}

fn search(token: &str, query: &str) -> Result<Vec<SearchItem>> {
    let result: SearchResult = github_get(
        token,
        &format!(
            "/search/issues?q={}&sort=updated&order=desc&per_page={MAX_ITEMS}",
            encode(query)
        ),
    )?;
    Ok(result.items)
}

fn checks_failed(token: &str, repo: &str, number: u64) -> Result<bool> {
    let pull: PullHead = github_get(token, &format!("/repos/{repo}/pulls/{number}"))?;
    let runs: CheckRuns = github_get(
        token,
        &format!(
            "/repos/{repo}/commits/{}/check-runs?per_page=100",
            pull.head.sha
        ),
    )?;
    let state = combine_checks(
        runs.check_runs
            .iter()
            .map(|r| (r.status.as_str(), r.conclusion.as_deref())),
    );
    Ok(state == Checks::Failure)
}

/// What's waiting for you on GitHub, across every repository you can see.
pub fn github_waiting(token: &str) -> Result<Vec<WaitingItem>> {
    let open = "is:pr is:open archived:false";
    let mut items: Vec<WaitingItem> = search(token, &format!("{open} review-requested:@me"))?
        .into_iter()
        .map(|i| {
            let by = i.user.login.clone();
            github_item(WaitingKind::Review, i, Some(by))
        })
        .collect();
    items.extend(
        search(
            token,
            &format!("{open} author:@me review:changes_requested"),
        )?
        .into_iter()
        .map(|i| github_item(WaitingKind::Changes, i, None)),
    );
    for pr in search(token, &format!("{open} author:@me"))?
        .into_iter()
        .take(MAX_CHECKED)
    {
        let repo = repo_name(&pr.repository_url);
        // One repository refusing access shouldn't hide the rest.
        if checks_failed(token, &repo, pr.number).unwrap_or(false) {
            items.push(github_item(WaitingKind::Failed, pr, None));
        }
    }
    Ok(items)
}

/// `YYYY-MM-DD`, so it can go straight into a GitHub search.
fn check_date(date: &str) -> Result<&str> {
    let ok = date.len() == 10
        && date.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    if ok {
        Ok(date)
    } else {
        Err(VcsError::GitHub(format!("not a date: {date}")))
    }
}

fn github_mention(item: SearchItem) -> GitHubMention {
    GitHubMention {
        repo: repo_name(&item.repository_url),
        number: item.number,
        title: item.title,
        url: item.html_url,
        by: item.user.login,
        at: item.updated_at,
        pull_request: item.pull_request.is_some(),
    }
}

/// Issues and pull requests that @mention you, updated on or after `since` (YYYY-MM-DD).
pub fn github_mentions(token: &str, since: &str) -> Result<Vec<GitHubMention>> {
    let since = check_date(since)?;
    Ok(search(token, &format!("mentions:@me updated:>={since}"))?
        .into_iter()
        .map(github_mention)
        .collect())
}

// ---------- Azure DevOps ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzPull {
    pull_request_id: u64,
    title: String,
    created_by: AzIdentity,
    creation_date: String,
    repository: AzRepo,
    #[serde(default)]
    reviewers: Vec<AzReviewer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzIdentity {
    display_name: String,
    #[serde(default)]
    id: String,
}

#[derive(Deserialize)]
struct AzRepo {
    name: String,
    project: AzProject,
}

#[derive(Deserialize)]
struct AzProject {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzReviewer {
    #[serde(default)]
    id: String,
    display_name: String,
    #[serde(default)]
    vote: i32,
}

#[derive(Deserialize)]
struct AzBuild {
    status: String,
    result: Option<String>,
}

fn azure_item(org: &str, kind: WaitingKind, pull: &AzPull, by: Option<String>) -> WaitingItem {
    let project = &pull.repository.project.name;
    let repo = &pull.repository.name;
    WaitingItem {
        kind,
        title: pull.title.clone(),
        repo: format!("{project}/{repo}"),
        number: Some(pull.pull_request_id),
        url: format!(
            "{HOST}/{}/{}/_git/{}/pullrequest/{}",
            encode(org),
            encode(project),
            encode(repo),
            pull.pull_request_id
        ),
        by,
        at: pull.creation_date.clone(),
    }
}

/// Pull requests where you're a reviewer who hasn't voted yet, not counting your own.
fn reviews_for(org: &str, me: &str, pulls: &[AzPull]) -> Vec<WaitingItem> {
    pulls
        .iter()
        .filter(|p| !p.created_by.id.eq_ignore_ascii_case(me))
        .filter(|p| {
            p.reviewers
                .iter()
                .any(|r| r.id.eq_ignore_ascii_case(me) && r.vote == 0)
        })
        .map(|p| {
            azure_item(
                org,
                WaitingKind::Review,
                p,
                Some(p.created_by.display_name.clone()),
            )
        })
        .collect()
}

/// Your pull requests where a reviewer is waiting for you or rejected them.
fn changes_for(org: &str, pulls: &[AzPull]) -> Vec<WaitingItem> {
    pulls
        .iter()
        .filter_map(|p| {
            let reviewer = p
                .reviewers
                .iter()
                .find(|r| r.vote == VOTE_WAITING_FOR_AUTHOR || r.vote == VOTE_REJECTED)?;
            Some(azure_item(
                org,
                WaitingKind::Changes,
                p,
                Some(reviewer.display_name.clone()),
            ))
        })
        .collect()
}

fn azure_get<T: serde::de::DeserializeOwned>(org: &str, auth: &Auth, url: &str) -> Result<T> {
    let sep = if url.contains('?') { '&' } else { '?' };
    let mut res = agent()
        .get(&format!("{url}{sep}api-version={API_VERSION}"))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| crate::azure::api_error(e, org))?;
    read_json(&mut res, org)
}

fn active_pulls(org: &str, auth: &Auth, criteria: &str, me: &str) -> Result<Vec<AzPull>> {
    let list: List<AzPull> = azure_get(
        org,
        auth,
        &format!(
            "{HOST}/{}/_apis/git/pullrequests?searchCriteria.{criteria}={}&searchCriteria.status=active&$top={MAX_ITEMS}",
            encode(org),
            encode(me)
        ),
    )?;
    Ok(list.value)
}

/// The latest build of the pull request's merge ref (what branch policies run) failed.
fn build_failed(org: &str, auth: &Auth, pull: &AzPull) -> Result<bool> {
    let builds: List<AzBuild> = azure_get(
        org,
        auth,
        &format!(
            "{HOST}/{}/{}/_apis/build/builds?branchName=refs/pull/{}/merge&$top=1&queryOrder=queueTimeDescending",
            encode(org),
            encode(&pull.repository.project.name),
            pull.pull_request_id
        ),
    )?;
    Ok(builds
        .value
        .first()
        .is_some_and(|b| b.status == "completed" && b.result.as_deref() == Some("failed")))
}

/// What's waiting for you in one Azure DevOps organization.
pub fn azure_waiting(org: &str) -> Result<Vec<WaitingItem>> {
    let org = check_org(org)?;
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let Some(me) = my_identity_id(org, &auth)? else {
        return Ok(Vec::new());
    };
    let mut items = reviews_for(org, &me, &active_pulls(org, &auth, "reviewerId", &me)?);
    let mine = active_pulls(org, &auth, "creatorId", &me)?;
    items.extend(changes_for(org, &mine));
    for pull in mine.iter().take(MAX_CHECKED) {
        if build_failed(org, &auth, pull).unwrap_or(false) {
            items.push(azure_item(org, WaitingKind::Failed, pull, None));
        }
    }
    Ok(items)
}

/// Everything waiting for you, newest first. A source that fails is skipped as long as another
/// one answers, so an expired Azure token doesn't hide your GitHub reviews.
pub fn waiting_on_you(
    github_token: Option<&str>,
    azure_orgs: &[String],
) -> Result<Vec<WaitingItem>> {
    let mut results = Vec::new();
    if let Some(token) = github_token {
        results.push(github_waiting(token));
    }
    results.extend(azure_orgs.iter().map(|org| azure_waiting(org)));
    let mut items = Vec::new();
    let mut first_error = None;
    let mut answered = false;
    for result in results {
        match result {
            Ok(found) => {
                answered = true;
                items.extend(found);
            }
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    match first_error {
        Some(e) if !answered => Err(e),
        _ => {
            items.sort_by(|a, b| b.at.cmp(&a.at));
            Ok(items)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search_item(number: u64, user: &str, pr: bool) -> SearchItem {
        serde_json::from_value(serde_json::json!({
            "number": number,
            "title": format!("Item {number}"),
            "html_url": format!("https://github.com/acme/api/pull/{number}"),
            "repository_url": "https://api.github.com/repos/acme/api",
            "user": { "login": user },
            "updated_at": "2026-10-11T10:00:00Z",
            "pull_request": if pr { serde_json::json!({ "url": "x" }) } else { serde_json::Value::Null },
        }))
        .unwrap()
    }

    fn pull(id: u64, creator: &str, reviewers: &[(&str, &str, i32)]) -> AzPull {
        serde_json::from_value(serde_json::json!({
            "pullRequestId": id,
            "title": format!("PR {id}"),
            "createdBy": { "displayName": format!("Name {creator}"), "id": creator },
            "creationDate": "2026-10-11T08:00:00Z",
            "repository": { "name": "case-api", "project": { "name": "Platform" } },
            "reviewers": reviewers.iter().map(|(id, name, vote)| serde_json::json!({ "id": id, "displayName": name, "vote": vote })).collect::<Vec<_>>(),
        }))
        .unwrap()
    }

    #[test]
    fn github_items_name_the_repository() {
        let item = github_item(
            WaitingKind::Review,
            search_item(142, "kari", true),
            Some("kari".into()),
        );
        assert_eq!(item.repo, "acme/api");
        assert_eq!(item.number, Some(142));
        assert_eq!(item.by.as_deref(), Some("kari"));
    }

    #[test]
    fn mentions_know_pull_requests_from_issues() {
        assert!(github_mention(search_item(1, "ola", true)).pull_request);
        assert!(!github_mention(search_item(2, "ola", false)).pull_request);
    }

    #[test]
    fn mention_dates_must_be_plain_dates() {
        assert!(check_date("2026-10-09").is_ok());
        assert!(check_date("2026-10-09 OR is:private").is_err());
        assert!(check_date("09.10.2026").is_err());
    }

    #[test]
    fn azure_reviews_skip_own_pulls_and_ones_already_voted_on() {
        let pulls = vec![
            pull(1, "kari", &[("me", "Me", 0)]),
            pull(2, "kari", &[("me", "Me", 10)]),
            pull(3, "me", &[("me", "Me", 0)]),
            pull(4, "ola", &[("someone", "Someone", 0)]),
        ];
        let items = reviews_for("contoso", "ME", &pulls);
        assert_eq!(
            items.iter().map(|i| i.number).collect::<Vec<_>>(),
            vec![Some(1)]
        );
        assert_eq!(items[0].by.as_deref(), Some("Name kari"));
        assert_eq!(items[0].repo, "Platform/case-api");
        assert_eq!(
            items[0].url,
            "https://dev.azure.com/contoso/Platform/_git/case-api/pullrequest/1"
        );
    }

    #[test]
    fn azure_changes_are_waiting_for_author_or_rejected() {
        let pulls = vec![
            pull(1, "me", &[("kari", "Kari", VOTE_WAITING_FOR_AUTHOR)]),
            pull(2, "me", &[("ola", "Ola", 5)]),
            pull(3, "me", &[("ola", "Ola", VOTE_REJECTED)]),
        ];
        let items = changes_for("contoso", &pulls);
        assert_eq!(
            items
                .iter()
                .map(|i| (i.number, i.by.clone()))
                .collect::<Vec<_>>(),
            vec![
                (Some(1), Some("Kari".into())),
                (Some(3), Some("Ola".into()))
            ]
        );
    }

    #[test]
    fn no_sources_means_nothing_waiting() {
        assert_eq!(waiting_on_you(None, &[]).unwrap(), Vec::new());
    }

    #[test]
    fn a_failing_source_is_an_error_only_when_nothing_answered() {
        assert!(waiting_on_you(None, &["not a valid org!".to_string()]).is_err());
    }
}
