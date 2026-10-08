//! Minimal Azure DevOps (Azure Repos + Pipelines) REST client: pull requests,
//! their build status, and opening new ones. Same shape as the GitHub client so
//! the rest of Fjord doesn't care where a repository lives.
//!
//! Auth, in order: $AZURE_DEVOPS_EXT_PAT, a personal access token saved in Settings
//! for the organization (OS credential store), or the Azure CLI (`az login`).

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::github::{
    Checks, HttpError, PullRequest, TokenSource, agent, checked, combine_checks, with_message,
};
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
    az_cli_token()
}

/// A Microsoft Entra token for Azure DevOps from the Azure CLI's sign-in.
fn az_cli_token() -> Option<(Auth, TokenSource)> {
    let out = crate::process::tool("az")
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
            .map_err(HttpError::from)
            .and_then(checked)
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
            .map_err(HttpError::from)
            .and_then(checked)
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

fn api_error(e: HttpError, org: &str) -> VcsError {
    match e {
        HttpError::Status(401, _) => VcsError::NoAzureToken(org.to_string()),
        HttpError::Status(403, message) => VcsError::Azure(with_message(
            "access denied (403)",
            &message.or_else(|| {
                Some(
                    "the token needs Code (Read & Write), Build (Read) and Work Items (Read)"
                        .into(),
                )
            }),
        )),
        HttpError::Status(404, message) => VcsError::Azure(with_message(
            "organization, project or repository not found (404)",
            &message,
        )),
        HttpError::Status(409, message) => VcsError::Azure(with_message(
            "Azure DevOps rejected the request (409): is the branch pushed, or does a PR already exist?",
            &message,
        )),
        HttpError::Status(code, message) => {
            VcsError::Azure(with_message(&format!("HTTP {code}"), &message))
        }
        HttpError::Transport(e) => VcsError::Azure(e),
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
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| match e {
            HttpError::Status(401, _) => VcsError::Azure(format!(
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

/// Adds an organization without a token, using the Azure CLI's sign-in (`az login`).
pub fn connect_azure_cli(org: &str) -> Result<AzureAccount> {
    let org = check_org(org)?;
    let (token, _) = az_cli_token().ok_or_else(|| {
        VcsError::Azure(
            "the Azure CLI isn't installed or signed in: run `az login`, or paste a token instead"
                .into(),
        )
    })?;
    whoami(org, &token, TokenSource::Cli)
}

pub fn disconnect_azure(org: &str) -> Result<()> {
    credentials::delete(&account_for(check_org(org)?))
}

// ---------- Azure Boards (work items assigned to you) ----------

/// Open work items assigned to the signed-in user, newest change first.
/// Work items assigned to you: open ones, plus finished ones changed in the last
/// `closed_days` days (0 = open only), newest change first.
pub fn assigned_query(closed_days: u32) -> String {
    let open = "[System.State] NOT IN ('Closed', 'Done', 'Removed', 'Resolved', 'Completed')";
    let states = if closed_days == 0 {
        open.to_string()
    } else {
        format!("({open} OR [System.ChangedDate] >= @Today - {closed_days})")
    };
    format!(
        "SELECT [System.Id] FROM WorkItems WHERE [System.AssignedTo] = @Me AND {states} \
         ORDER BY [System.ChangedDate] DESC"
    )
}
const MAX_WORK_ITEMS: usize = 200;
const WORK_ITEM_FIELDS: &str = "System.Id,System.Rev,System.Parent,System.Title,System.Description,System.TeamProject,\
     System.AreaPath,System.WorkItemType,System.State,Microsoft.VSTS.Common.Priority,\
     Microsoft.VSTS.Scheduling.DueDate,Microsoft.VSTS.Scheduling.TargetDate,\
     Microsoft.VSTS.Common.AcceptanceCriteria,Microsoft.VSTS.TCM.ReproSteps";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WiqlResult {
    work_items: Vec<WiqlRef>,
}

#[derive(Deserialize)]
struct WiqlRef {
    id: u64,
}

#[derive(Deserialize)]
struct ApiWorkItem {
    id: u64,
    rev: i64,
    fields: serde_json::Map<String, serde_json::Value>,
}

/// An Azure Boards work item, with HTML fields already turned into markdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkItem {
    pub org: String,
    pub id: u64,
    pub rev: i64,
    pub project: String,
    pub kind: String,
    pub state: String,
    pub area: String,
    pub title: String,
    pub description_md: String,
    pub acceptance_md: String,
    /// Azure priority 1 (highest) to 4.
    pub priority: Option<i64>,
    /// YYYY-MM-DD
    pub due: Option<String>,
    /// The parent work item (e.g. the User Story above a Task).
    pub parent: Option<u64>,
}

impl WorkItem {
    pub fn url(&self) -> String {
        format!(
            "{HOST}/{}/{}/_workitems/edit/{}",
            encode(&self.org),
            encode(&self.project),
            self.id
        )
    }

    /// Fjord's fields: Azure 1/2/3/4 → high/medium/low/none, plus a footer linking back.
    pub fn to_external_item(&self) -> fjord_core::ExternalItem {
        let mut body = self.description_md.trim().to_string();
        if !self.acceptance_md.trim().is_empty() {
            body.push_str("\n\n### Acceptance criteria\n\n");
            body.push_str(self.acceptance_md.trim());
        }
        body.push_str(&format!(
            "\n\n---\nAzure DevOps: [{} {}]({}) · {} · {}",
            self.kind,
            self.id,
            self.url(),
            self.state,
            self.area
        ));
        fjord_core::ExternalItem {
            source: "azure".into(),
            external_id: format!("{}/{}", self.org.to_lowercase(), self.id),
            url: self.url(),
            rev: self.rev,
            title: self.title.clone(),
            body_md: body.trim_start().to_string(),
            priority: match self.priority {
                Some(1) => 3,
                Some(2) => 2,
                Some(3) => 1,
                _ => 0,
            },
            due_at: self.due.clone(),
        }
    }
}

fn field_str(fields: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    fields
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn work_item_from(org: &str, item: ApiWorkItem) -> WorkItem {
    let f = &item.fields;
    let description = match field_str(f, "System.Description") {
        d if d.trim().is_empty() => field_str(f, "Microsoft.VSTS.TCM.ReproSteps"),
        d => d,
    };
    let due = [
        field_str(f, "Microsoft.VSTS.Scheduling.DueDate"),
        field_str(f, "Microsoft.VSTS.Scheduling.TargetDate"),
    ]
    .into_iter()
    .find(|d| d.len() >= 10)
    .map(|d| d[..10].to_string());
    WorkItem {
        org: org.to_string(),
        id: item.id,
        rev: item.rev,
        project: field_str(f, "System.TeamProject"),
        kind: field_str(f, "System.WorkItemType"),
        state: field_str(f, "System.State"),
        area: field_str(f, "System.AreaPath"),
        title: field_str(f, "System.Title"),
        description_md: html_to_markdown(&description),
        acceptance_md: html_to_markdown(&field_str(f, "Microsoft.VSTS.Common.AcceptanceCriteria")),
        priority: f
            .get("Microsoft.VSTS.Common.Priority")
            .and_then(|v| v.as_i64()),
        due,
        parent: f.get("System.Parent").and_then(|v| v.as_u64()),
    }
}

/// Open work items assigned to you in one organization.
pub fn assigned_work_items(org: &str, closed_days: u32) -> Result<Vec<WorkItem>> {
    let org = check_org(org)?;
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let agent = agent();
    let mut res = agent
        .post(&format!(
            "{HOST}/{}/_apis/wit/wiql?api-version={API_VERSION}",
            encode(org)
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .send_json(json!({ "query": assigned_query(closed_days) }))
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| api_error(e, org))?;
    let wiql: WiqlResult = read_json(&mut res, org)?;
    let ids: Vec<String> = wiql
        .work_items
        .iter()
        .take(MAX_WORK_ITEMS)
        .map(|w| w.id.to_string())
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut res = agent
        .get(&format!(
            "{HOST}/{}/_apis/wit/workitems?ids={}&fields={WORK_ITEM_FIELDS}&api-version={API_VERSION}",
            encode(org),
            ids.join(",")
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
            .and_then(checked)
            .map_err(|e| api_error(e, org))?;
    let items: List<ApiWorkItem> = read_json(&mut res, org)?;
    Ok(items
        .value
        .into_iter()
        .map(|i| work_item_from(org, i))
        .collect())
}

/// States that mean a work item is finished.
pub const CLOSED_STATES: &[&str] = &["Closed", "Done", "Removed", "Resolved", "Completed"];

/// Parent and state of a work item, for walking up the hierarchy and noticing
/// items that were closed since they were imported.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkItemInfo {
    pub parent: Option<u64>,
    pub state: String,
}

/// Parent and state for each of `ids` (deleted or inaccessible items are left out).
pub fn work_item_info(
    org: &str,
    ids: &[u64],
) -> Result<std::collections::HashMap<u64, WorkItemInfo>> {
    let org = check_org(org)?;
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let agent = agent();
    let mut out = std::collections::HashMap::new();
    for chunk in ids.chunks(MAX_WORK_ITEMS) {
        let list: Vec<String> = chunk.iter().map(u64::to_string).collect();
        let mut res = agent
            .get(&format!(
                "{HOST}/{}/_apis/wit/workitems?ids={}&fields=System.Id,System.Parent,System.State&errorPolicy=omit&api-version={API_VERSION}",
                encode(org),
                list.join(",")
            ))
            .header("Accept", "application/json")
            .header("Authorization", &auth.header())
            .call()
            .map_err(HttpError::from)
            .and_then(checked)
            .map_err(|e| api_error(e, org))?;
        let items: List<Option<ApiWorkItem>> = read_json(&mut res, org)?;
        for item in items.value.into_iter().flatten() {
            let info = WorkItemInfo {
                parent: item.fields.get("System.Parent").and_then(|v| v.as_u64()),
                state: field_str(&item.fields, "System.State"),
            };
            out.insert(item.id, info);
        }
    }
    Ok(out)
}

/// A comment in a work item's Discussion.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkItemComment {
    pub author: String,
    /// ISO 8601
    pub created: String,
    pub text_md: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiComment {
    text: String,
    created_by: ApiIdentity,
    created_date: String,
}

#[derive(Deserialize)]
struct ApiComments {
    comments: Vec<ApiComment>,
}

/// (org, project, id) from a work item's web URL, as stored for imported tasks.
pub fn parse_work_item_url(url: &str) -> Option<(String, String, u64)> {
    let rest = url.strip_prefix(&format!("{HOST}/"))?;
    let mut parts = rest.split('/');
    let (org, project) = (parts.next()?, parts.next()?);
    let id = rest.rsplit('/').next()?.parse().ok()?;
    rest.contains("/_workitems/edit/")
        .then(|| (decode(org), decode(project), id))
}

fn comments_from(list: ApiComments) -> Vec<WorkItemComment> {
    list.comments
        .into_iter()
        .map(|c| WorkItemComment {
            author: c.created_by.display_name,
            created: c.created_date,
            text_md: html_to_markdown(&c.text),
        })
        .collect()
}

/// The Discussion of a work item, oldest first. Read-only.
pub fn work_item_comments(org: &str, project: &str, id: u64) -> Result<Vec<WorkItemComment>> {
    let org = check_org(org)?;
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let mut res = agent()
        .get(&format!(
            "{HOST}/{}/{}/_apis/wit/workItems/{id}/comments?$top=200&order=asc&api-version=7.1-preview.4",
            encode(org),
            encode(project)
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
            .and_then(checked)
            .map_err(|e| api_error(e, org))?;
    Ok(comments_from(read_json(&mut res, org)?))
}

/// Good-enough HTML → markdown for work item descriptions: paragraphs, line breaks,
/// lists, bold/italic, links and code; other tags are dropped, entities decoded.
pub fn html_to_markdown(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    let mut link_href: Option<String> = None;
    let mut list_depth: usize = 0;
    while let Some(start) = rest.find('<') {
        out.push_str(&decode_entities(&rest[..start]));
        let Some(end) = rest[start..].find('>') else {
            rest = &rest[start..];
            break;
        };
        let tag = &rest[start + 1..start + end];
        rest = &rest[start + end + 1..];
        let closing = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        match (name.as_str(), closing) {
            ("br", _) => out.push('\n'),
            ("p" | "div" | "h1" | "h2" | "h3" | "h4" | "tr", true) => out.push_str("\n\n"),
            ("p" | "div", false) if !out.is_empty() && !out.ends_with('\n') => out.push('\n'),
            ("h1" | "h2" | "h3" | "h4", false) => out.push_str("\n### "),
            ("ul" | "ol", false) => list_depth += 1,
            ("ul" | "ol", true) => {
                list_depth = list_depth.saturating_sub(1);
                // A blank line ends the list, so following text isn't swallowed by the last item.
                out.push_str("\n\n");
            }
            ("li", false) => {
                if !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                out.push_str("- ");
            }
            ("b" | "strong", _) => out.push_str("**"),
            ("i" | "em", _) => out.push('*'),
            ("code", _) => out.push('`'),
            ("td" | "th", true) => out.push_str(" | "),
            ("a", false) => {
                link_href = tag
                    .split("href=\"")
                    .nth(1)
                    .and_then(|h| h.split('"').next())
                    .map(decode_entities);
                out.push('[');
            }
            ("a", true) => match link_href.take() {
                Some(href) => out.push_str(&format!("]({href})")),
                None => out.push(']'),
            },
            _ => {}
        }
    }
    out.push_str(&decode_entities(rest));
    // Tidy whitespace: trim lines, collapse 3+ newlines.
    let lines: Vec<&str> = out.lines().map(str::trim_end).collect();
    let mut tidy = String::new();
    let mut blank = 0;
    for line in lines {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        tidy.push_str(line);
        tidy.push('\n');
    }
    tidy.trim().to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
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

    #[test]
    fn converts_work_item_html_to_markdown() {
        let html = "<div>Users get logged out.</div><div><br></div><div><b>Steps</b>:</div>\
            <ol><li>Open app</li><li>Wait &amp; see</li></ol>\
            <div>See <a href=\"https://example.com/log\">the log</a> &lt;3</div>";
        assert_eq!(
            html_to_markdown(html),
            "Users get logged out.\n\n**Steps**:\n\n- Open app\n- Wait & see\n\nSee [the log](https://example.com/log) <3"
        );
        assert_eq!(html_to_markdown(""), "");
    }

    #[test]
    fn maps_work_items_from_the_documented_shape() {
        // Trimmed from the "Work Items - List" example response.
        let json = r#"{"count":1,"value":[{"id":297,"rev":4,"fields":{
            "System.Id":297,"System.TeamProject":"Mobile App","System.AreaPath":"Mobile App\\iOS",
            "System.WorkItemType":"Bug","System.State":"Active","System.Title":"Crash on login",
            "System.Description":"<div>App crashes</div>",
            "Microsoft.VSTS.Common.AcceptanceCriteria":"<ul><li>No crash</li></ul>",
            "Microsoft.VSTS.Common.Priority":1,"Microsoft.VSTS.Scheduling.DueDate":"2026-10-20T00:00:00Z","System.Parent":280}}]}"#;
        let list: List<ApiWorkItem> = serde_json::from_str(json).unwrap();
        let wi = work_item_from("Contoso", list.value.into_iter().next().unwrap());
        assert_eq!(
            (wi.id, wi.rev, wi.kind.as_str(), wi.due.as_deref()),
            (297, 4, "Bug", Some("2026-10-20"))
        );
        assert_eq!(
            wi.url(),
            "https://dev.azure.com/Contoso/Mobile%20App/_workitems/edit/297"
        );
        assert_eq!(wi.parent, Some(280));

        let item = wi.to_external_item();
        assert_eq!(
            (item.external_id.as_str(), item.priority, item.rev),
            ("contoso/297", 3, 4)
        );
        assert!(
            item.body_md
                .starts_with("App crashes\n\n### Acceptance criteria\n\n- No crash")
        );
        assert!(item.body_md.ends_with("Azure DevOps: [Bug 297](https://dev.azure.com/Contoso/Mobile%20App/_workitems/edit/297) · Active · Mobile App\\iOS"));
    }

    #[test]
    fn reads_the_discussion_of_a_work_item() {
        assert_eq!(
            parse_work_item_url("https://dev.azure.com/contoso/Mobile%20App/_workitems/edit/297"),
            Some(("contoso".to_string(), "Mobile App".to_string(), 297))
        );
        assert_eq!(parse_work_item_url("https://github.com/a/b/issues/1"), None);
        // Trimmed from the "Comments - Get Comments" example response.
        let json = r#"{"totalCount":2,"count":2,"comments":[
          {"id":1,"text":"<div>Repro on <b>iOS 18</b></div>","createdBy":{"displayName":"Ada"},"createdDate":"2026-10-07T09:00:00Z"},
          {"id":2,"text":"<p>Fixed in PR 14</p>","createdBy":{"displayName":"Jonas"},"createdDate":"2026-10-08T09:00:00Z"}]}"#;
        let comments = comments_from(serde_json::from_str(json).unwrap());
        assert_eq!(comments.len(), 2);
        assert_eq!(
            (comments[0].author.as_str(), comments[0].text_md.as_str()),
            ("Ada", "Repro on **iOS 18**")
        );
        assert_eq!(comments[1].created, "2026-10-08T09:00:00Z");
    }

    #[test]
    fn query_includes_recently_finished_items_when_asked() {
        assert!(!assigned_query(0).contains("ChangedDate] >="));
        let q = assigned_query(30);
        assert!(q.contains("OR [System.ChangedDate] >= @Today - 30"));
        assert!(q.contains("[System.AssignedTo] = @Me"));
    }
}
