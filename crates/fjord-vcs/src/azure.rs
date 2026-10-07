//! Minimal Azure DevOps (Azure Repos + Pipelines) REST client: pull requests,
//! their build status, and opening new ones. Same shape as the GitHub client so
//! the rest of Fjord doesn't care where a repository lives.
//!
//! Auth, in order: $AZURE_DEVOPS_EXT_PAT, a personal access token saved in Settings
//! for the organization (OS credential store), or the Azure CLI (`az login`).

use std::process::Command;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::github::{Checks, PullRequest, TokenSource, agent, combine_checks};
use crate::{Result, VcsError, credentials};

const HOST: &str = "https://dev.azure.com";
const API_VERSION: &str = "7.1";
const TOP: u32 = 30;
/// The Azure DevOps resource id, for `az account get-access-token`.
const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

/// (organization, project, repository) from an Azure Repos remote URL:
/// `https://[user@]dev.azure.com/org/project/_git/repo`,
/// `git@ssh.dev.azure.com:v3/org/project/repo`, and the older
/// `https://org.visualstudio.com/[DefaultCollection/]project/_git/repo` and
/// `org@vs-ssh.visualstudio.com:v3/org/project/repo` forms.
pub fn parse_azure_remote(url: &str) -> Option<(String, String, String)> {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let parts: Vec<String> = if let Some(rest) =
        url.strip_prefix("git@ssh.dev.azure.com:v3/").or_else(|| {
            url.split_once("@vs-ssh.visualstudio.com:v3/")
                .map(|(_, r)| r)
        }) {
        rest.split('/').map(decode).collect()
    } else {
        let rest = url.strip_prefix("https://")?;
        let (host, path) = rest.split_once('/')?;
        let host = host.rsplit('@').next()?;
        let segments: Vec<&str> = path.split('/').collect();
        let git = segments.iter().position(|s| *s == "_git")?;
        let repo = segments.get(git + 1)?;
        let (org, project) = if host.eq_ignore_ascii_case("dev.azure.com") {
            (
                segments.first()?.to_string(),
                segments.get(git.checked_sub(1)?)?,
            )
        } else {
            let org = host.strip_suffix(".visualstudio.com")?;
            (org.to_string(), segments.get(git.checked_sub(1)?)?)
        };
        vec![decode(&org), decode(project), decode(repo)]
    };
    match parts.as_slice() {
        [org, project, repo] if [org, project, repo].iter().all(|p| valid(p)) => {
            Some((org.clone(), project.clone(), repo.clone()))
        }
        _ => None,
    }
}

fn valid(part: &str) -> bool {
    !part.is_empty() && !part.starts_with('_') && !part.contains(['/', '?', '#'])
}

/// Percent-decoding for project and repository names (spaces are common: `My%20Project`).
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(b) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Credential store account for an organization's token.
fn account_for(org: &str) -> String {
    format!("azure:{}", org.to_lowercase())
}

#[derive(Clone)]
enum Auth {
    /// Personal access token (HTTP Basic with an empty user name).
    Pat(String),
    /// Microsoft Entra token from the Azure CLI.
    Bearer(String),
}

impl Auth {
    fn header(&self) -> String {
        match self {
            Auth::Pat(pat) => format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(format!(":{pat}"))
            ),
            Auth::Bearer(token) => format!("Bearer {token}"),
        }
    }
}

fn find_auth(org: &str) -> Option<(Auth, TokenSource)> {
    if let Some(t) = std::env::var("AZURE_DEVOPS_EXT_PAT")
        .ok()
        .filter(|t| !t.trim().is_empty())
    {
        return Some((Auth::Pat(t.trim().to_string()), TokenSource::Env));
    }
    if let Some(t) = credentials::get(&account_for(org)) {
        return Some((Auth::Pat(t), TokenSource::Saved));
    }
    let out = Command::new("az")
        .args([
            "account",
            "get-access-token",
            "--resource",
            AZURE_DEVOPS_RESOURCE,
            "--query",
            "accessToken",
            "-o",
            "tsv",
        ])
        .output()
        .ok()?;
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !token.is_empty()).then_some((Auth::Bearer(token), TokenSource::Cli))
}

// ---------- API shapes ----------

#[derive(Deserialize)]
struct List<T> {
    value: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiIdentity {
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiCommitRef {
    commit_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPull {
    pull_request_id: u64,
    title: String,
    /// active | completed | abandoned
    status: String,
    #[serde(default)]
    is_draft: bool,
    created_by: ApiIdentity,
    source_ref_name: String,
    target_ref_name: String,
    last_merge_source_commit: Option<ApiCommitRef>,
    creation_date: String,
    closed_date: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiBuild {
    /// notStarted | inProgress | completed | cancelling | postponed
    status: String,
    /// succeeded | partiallySucceeded | failed | canceled
    result: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRepository {
    default_branch: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiConnection {
    authenticated_user: ApiConnectionUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiConnectionUser {
    provider_display_name: Option<String>,
    custom_display_name: Option<String>,
}

fn branch(refname: &str) -> String {
    refname
        .strip_prefix("refs/heads/")
        .unwrap_or(refname)
        .to_string()
}

/// Maps a build to GitHub-style (status, conclusion) so [`combine_checks`] can fold it.
fn build_check(b: &ApiBuild) -> (&'static str, Option<&'static str>) {
    match (b.status.as_str(), b.result.as_deref()) {
        ("completed", Some("succeeded" | "partiallySucceeded")) => ("completed", Some("success")),
        ("completed", Some("canceled")) => ("completed", Some("cancelled")),
        ("completed", _) => ("completed", Some("failure")),
        _ => ("in_progress", None),
    }
}

// ---------- client ----------

pub struct AzureDevOps {
    org: String,
    project: String,
    repo: String,
    auth: Option<Auth>,
    agent: ureq::Agent,
}

impl AzureDevOps {
    pub fn new(org: &str, project: &str, repo: &str) -> Self {
        Self {
            org: org.to_string(),
            project: project.to_string(),
            repo: repo.to_string(),
            auth: find_auth(org).map(|(a, _)| a),
            agent: agent(),
        }
    }

    pub fn has_token(&self) -> bool {
        self.auth.is_some()
    }

    pub fn org(&self) -> &str {
        &self.org
    }

    fn project_url(&self, path: &str) -> String {
        format!(
            "{HOST}/{}/{}/_apis{path}",
            encode(&self.org),
            encode(&self.project)
        )
    }

    fn repo_url(&self, path: &str) -> String {
        self.project_url(&format!("/git/repositories/{}{path}", encode(&self.repo)))
    }

    fn web_url(&self) -> String {
        format!(
            "{HOST}/{}/{}/_git/{}",
            encode(&self.org),
            encode(&self.project),
            encode(&self.repo)
        )
    }

    fn with_query(url: String) -> String {
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}api-version={API_VERSION}")
    }

    fn auth(&self) -> Result<&Auth> {
        self.auth
            .as_ref()
            .ok_or(VcsError::NoAzureToken(self.org.clone()))
    }

    fn get<T: serde::de::DeserializeOwned>(&self, url: String) -> Result<T> {
        let mut res = self
            .agent
            .get(&Self::with_query(url))
            .header("Accept", "application/json")
            .header("Authorization", &self.auth()?.header())
            .call()
            .map_err(|e| api_error(e, &self.org))?;
        read_json(&mut res, &self.org)
    }

    pub fn default_branch(&self) -> Result<String> {
        let repo: ApiRepository = self.get(self.repo_url(""))?;
        Ok(branch(
            repo.default_branch.as_deref().unwrap_or("refs/heads/main"),
        ))
    }

    pub fn pull_requests(&self) -> Result<Vec<PullRequest>> {
        let list: List<ApiPull> = self.get(self.repo_url(&format!(
            "/pullrequests?searchCriteria.status=all&$top={TOP}"
        )))?;
        let mut prs: Vec<PullRequest> = list.value.into_iter().map(|p| self.to_pr(p)).collect();
        for pr in prs.iter_mut().filter(|p| p.state == "open") {
            pr.checks = self.checks(pr.number).unwrap_or(Checks::None);
        }
        prs.sort_by_key(|p| p.state != "open");
        Ok(prs)
    }

    /// Latest pipeline run for the PR's merge ref (what branch policies build).
    pub fn checks(&self, pr: u64) -> Result<Checks> {
        let builds: List<ApiBuild> = self.get(self.project_url(&format!(
            "/build/builds?branchName=refs/pull/{pr}/merge&$top=1&queryOrder=queueTimeDescending"
        )))?;
        Ok(combine_checks(builds.value.iter().map(build_check)))
    }

    pub fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<PullRequest> {
        let payload = json!({
            "sourceRefName": format!("refs/heads/{head}"),
            "targetRefName": format!("refs/heads/{base}"),
            "title": title,
            "description": body,
            "isDraft": draft,
        });
        let mut res = self
            .agent
            .post(&Self::with_query(self.repo_url("/pullrequests")))
            .header("Accept", "application/json")
            .header("Authorization", &self.auth()?.header())
            .send_json(&payload)
            .map_err(|e| api_error(e, &self.org))?;
        let pr: ApiPull = read_json(&mut res, &self.org)?;
        Ok(self.to_pr(pr))
    }

    fn to_pr(&self, p: ApiPull) -> PullRequest {
        let state = match p.status.as_str() {
            "completed" => "merged",
            "abandoned" => "closed",
            _ => "open",
        };
        PullRequest {
            number: p.pull_request_id,
            title: p.title,
            state: state.to_string(),
            draft: p.is_draft,
            url: format!("{}/pullrequest/{}", self.web_url(), p.pull_request_id),
            author: p.created_by.display_name,
            head: branch(&p.source_ref_name),
            base: branch(&p.target_ref_name),
            head_sha: p
                .last_merge_source_commit
                .map(|c| c.commit_id)
                .unwrap_or_default(),
            updated_at: p.closed_date.unwrap_or(p.creation_date),
            checks: Checks::None,
        }
    }
}

/// Azure DevOps answers a bad or expired PAT with a 203 sign-in page instead of
/// a 401, so anything that isn't JSON is treated as an authentication problem.
fn read_json<T: serde::de::DeserializeOwned>(
    res: &mut ureq::http::Response<ureq::Body>,
    org: &str,
) -> Result<T> {
    let is_json = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("json"));
    if res.status().as_u16() == 203 || !is_json {
        return Err(VcsError::Azure(format!(
            "Azure DevOps did not accept the token for «{org}» (expired, or missing scopes)"
        )));
    }
    res.body_mut()
        .read_json()
        .map_err(|e| VcsError::Azure(e.to_string()))
}

fn api_error(e: ureq::Error, org: &str) -> VcsError {
    match e {
        ureq::Error::StatusCode(401) => VcsError::NoAzureToken(org.to_string()),
        ureq::Error::StatusCode(403) => VcsError::Azure(
            "access denied (403): the token needs Code (Read & Write) and Build (Read)".into(),
        ),
        ureq::Error::StatusCode(404) => {
            VcsError::Azure("organization, project or repository not found (404)".into())
        }
        ureq::Error::StatusCode(409) => VcsError::Azure(
            "Azure DevOps rejected the request (409): is the branch pushed, or does a PR already exist?"
                .into(),
        ),
        other => VcsError::Azure(other.to_string()),
    }
}

// ---------- accounts (Settings) ----------

/// An organization Fjord can reach, as shown in Settings.
#[derive(Debug, Clone, Serialize)]
pub struct AzureAccount {
    pub org: String,
    pub user: String,
    pub source: TokenSource,
}

fn whoami(org: &str, auth: &Auth, source: TokenSource) -> Result<AzureAccount> {
    let mut res = agent()
        .get(&format!("{HOST}/{}/_apis/connectionData", encode(org)))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(401) => VcsError::Azure(format!(
                "Azure DevOps did not accept this token for «{org}» (401)"
            )),
            other => api_error(other, org),
        })?;
    let data: ApiConnection = read_json(&mut res, org)?;
    let u = data.authenticated_user;
    Ok(AzureAccount {
        org: org.to_string(),
        user: u
            .custom_display_name
            .or(u.provider_display_name)
            .unwrap_or_else(|| "?".into()),
        source,
    })
}

fn check_org(org: &str) -> Result<&str> {
    let org = org.trim();
    if org.is_empty()
        || !org
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(VcsError::Azure(
            "enter the organization name from dev.azure.com/<name>".into(),
        ));
    }
    Ok(org)
}

/// Who Fjord talks to an organization as, or None when there is no token for it.
pub fn azure_account(org: &str) -> Result<Option<AzureAccount>> {
    let org = check_org(org)?;
    match find_auth(org) {
        Some((auth, source)) => whoami(org, &auth, source).map(Some),
        None => Ok(None),
    }
}

/// Checks a personal access token against the organization, then saves it in the
/// OS credential store.
pub fn connect_azure(org: &str, pat: &str) -> Result<AzureAccount> {
    let org = check_org(org)?;
    let pat = pat.trim();
    if pat.is_empty() || pat.chars().any(char::is_whitespace) {
        return Err(VcsError::Azure(
            "that doesn't look like a personal access token".into(),
        ));
    }
    let account = whoami(org, &Auth::Pat(pat.to_string()), TokenSource::Saved)?;
    credentials::set(&account_for(org), pat)?;
    Ok(account)
}

pub fn disconnect_azure(org: &str) -> Result<()> {
    credentials::delete(&account_for(check_org(org)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_azure_remotes() {
        let expected = Some((
            "contoso".to_string(),
            "Mobile App".to_string(),
            "bokost".to_string(),
        ));
        for url in [
            "https://dev.azure.com/contoso/Mobile%20App/_git/bokost",
            "https://contoso@dev.azure.com/contoso/Mobile%20App/_git/bokost",
            "https://dev.azure.com/contoso/Mobile%20App/_git/bokost/",
            "git@ssh.dev.azure.com:v3/contoso/Mobile%20App/bokost",
            "https://contoso.visualstudio.com/Mobile%20App/_git/bokost",
            "https://contoso.visualstudio.com/DefaultCollection/Mobile%20App/_git/bokost",
            "contoso@vs-ssh.visualstudio.com:v3/contoso/Mobile%20App/bokost",
        ] {
            assert_eq!(parse_azure_remote(url), expected, "{url}");
        }
        for url in [
            "https://github.com/jhtjernsmo/fjord.git",
            "https://dev.azure.com/contoso/project",
            "https://dev.azure.com/contoso/_git/",
            "git@ssh.dev.azure.com:v3/contoso/project",
        ] {
            assert_eq!(parse_azure_remote(url), None, "{url}");
        }
    }

    #[test]
    fn encodes_names_for_urls() {
        assert_eq!(encode("Mobile App"), "Mobile%20App");
        assert_eq!(decode(&encode("Prosjekt æøå & co")), "Prosjekt æøå & co");
    }

    #[test]
    fn maps_pull_requests_from_the_documented_shape() {
        let az = AzureDevOps {
            org: "contoso".into(),
            project: "Mobile App".into(),
            repo: "bokost".into(),
            auth: None,
            agent: agent(),
        };
        // Trimmed from the "Pull Requests - Get Pull Requests" example response.
        let json = r#"{"value":[
          {"pullRequestId":22,"title":"Fix push","status":"active","isDraft":true,
           "createdBy":{"displayName":"Jonas"},"sourceRefName":"refs/heads/fjord/1-fix-push",
           "targetRefName":"refs/heads/main","lastMergeSourceCommit":{"commitId":"abc123"},
           "creationDate":"2026-10-07T10:00:00Z"},
          {"pullRequestId":21,"title":"Old","status":"completed","createdBy":{"displayName":"Ada"},
           "sourceRefName":"refs/heads/feature","targetRefName":"refs/heads/main",
           "creationDate":"2026-10-01T10:00:00Z","closedDate":"2026-10-02T10:00:00Z"},
          {"pullRequestId":20,"title":"Nope","status":"abandoned","createdBy":{"displayName":"Ada"},
           "sourceRefName":"refs/heads/x","targetRefName":"refs/heads/main",
           "creationDate":"2026-09-01T10:00:00Z"}
        ],"count":3}"#;
        let list: List<ApiPull> = serde_json::from_str(json).unwrap();
        let prs: Vec<PullRequest> = list.value.into_iter().map(|p| az.to_pr(p)).collect();
        assert_eq!(
            prs.iter().map(|p| p.state.as_str()).collect::<Vec<_>>(),
            ["open", "merged", "closed"]
        );
        assert_eq!(
            (prs[0].head.as_str(), prs[0].base.as_str()),
            ("fjord/1-fix-push", "main")
        );
        assert!(prs[0].draft);
        assert_eq!(prs[0].head_sha, "abc123");
        assert_eq!(
            prs[0].url,
            "https://dev.azure.com/contoso/Mobile%20App/_git/bokost/pullrequest/22"
        );
        assert_eq!(prs[1].updated_at, "2026-10-02T10:00:00Z");
    }

    #[test]
    fn folds_builds_into_checks() {
        let b = |status: &str, result: Option<&str>| ApiBuild {
            status: status.into(),
            result: result.map(str::to_string),
        };
        let fold = |builds: &[ApiBuild]| combine_checks(builds.iter().map(build_check));
        assert_eq!(fold(&[]), Checks::None);
        assert_eq!(fold(&[b("inProgress", None)]), Checks::Pending);
        assert_eq!(fold(&[b("completed", Some("succeeded"))]), Checks::Success);
        assert_eq!(fold(&[b("completed", Some("failed"))]), Checks::Failure);
        assert_eq!(fold(&[b("completed", Some("canceled"))]), Checks::Failure);
    }

    #[test]
    fn pat_uses_basic_auth_with_empty_user() {
        assert_eq!(Auth::Pat("abc".into()).header(), "Basic OmFiYw==");
        assert_eq!(Auth::Bearer("t".into()).header(), "Bearer t");
    }

    #[test]
    fn connect_rejects_bad_input_without_network() {
        assert!(connect_azure("", "pat").is_err());
        assert!(connect_azure("dev.azure.com/contoso", "pat").is_err());
        assert!(connect_azure("contoso", "  ").is_err());
    }
}
