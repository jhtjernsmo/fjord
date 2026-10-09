//! Azure Boards work items where you were @mentioned recently but that aren't
//! assigned to you, with who mentioned you and what they wrote. Read-only.

use serde::Serialize;

use crate::azure::{
    ApiComments, ApiWorkItem, Auth, HOST, check_org, encode, fetch_work_items, field_str,
    find_auth, html_to_markdown, my_identity_id, read_json, wiql_ids,
};
use crate::github::{HttpError, agent, checked};
use crate::{Result, VcsError};

/// Azure DevOps' own "mentioned in the last 30 days" list (Azure DevOps Services only).
pub const MENTIONS_QUERY: &str = "SELECT [System.Id] FROM WorkItems \
     WHERE [System.Id] IN (@RecentMentions) ORDER BY [System.ChangedDate] DESC";
/// The Overview shows a handful; older mentions are rarely still relevant.
const MAX_MENTIONS: usize = 20;
const SNIPPET_CHARS: usize = 160;
const MENTION_FIELDS: &str = "System.Id,System.Title,System.TeamProject,System.WorkItemType,\
     System.State,System.AssignedTo,System.ChangedBy,System.ChangedDate";
const COMMENTS_TO_SCAN: u32 = 20;

/// A work item you were mentioned in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mention {
    pub org: String,
    pub id: u64,
    pub project: String,
    pub kind: String,
    pub state: String,
    pub title: String,
    pub url: String,
    /// Who mentioned you (or, if the comment can't be found, who changed it last).
    pub by: String,
    /// ISO 8601
    pub at: String,
    /// The start of the comment that mentions you, as plain text.
    pub snippet: String,
}

/// Mentions across organizations, newest first.
pub fn recent_mentions(orgs: &[String]) -> Result<Vec<Mention>> {
    let mut all = Vec::new();
    for org in orgs {
        all.extend(org_mentions(org)?);
    }
    all.sort_by(|a, b| b.at.cmp(&a.at));
    Ok(all)
}

fn org_mentions(org: &str) -> Result<Vec<Mention>> {
    let org = check_org(org)?;
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let ids: Vec<u64> = wiql_ids(org, &auth, MENTIONS_QUERY)?
        .into_iter()
        .take(MAX_MENTIONS)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let me = my_identity_id(org, &auth)?;
    let items = fetch_work_items(org, &auth, &ids, MENTION_FIELDS)?;
    Ok(items
        .into_iter()
        .filter(|item| !assigned_to(item, me.as_deref()))
        .map(|item| mention_from(org, &auth, me.as_deref(), item))
        .collect())
}

fn identity_field<'a>(item: &'a ApiWorkItem, key: &str, part: &str) -> Option<&'a str> {
    item.fields.get(key)?.get(part)?.as_str()
}

fn assigned_to(item: &ApiWorkItem, me: Option<&str>) -> bool {
    match (identity_field(item, "System.AssignedTo", "id"), me) {
        (Some(a), Some(me)) => a.eq_ignore_ascii_case(me),
        _ => false,
    }
}

fn mention_from(org: &str, auth: &Auth, me: Option<&str>, item: ApiWorkItem) -> Mention {
    let project = field_str(&item.fields, "System.TeamProject");
    let comment = me.and_then(|me| mentioning_comment(org, auth, &project, item.id, me));
    let (by, at, snippet) = comment.unwrap_or_else(|| {
        (
            identity_field(&item, "System.ChangedBy", "displayName")
                .unwrap_or_default()
                .to_string(),
            field_str(&item.fields, "System.ChangedDate"),
            String::new(),
        )
    });
    Mention {
        org: org.to_string(),
        id: item.id,
        url: format!(
            "{HOST}/{}/{}/_workitems/edit/{}",
            encode(org),
            encode(&project),
            item.id
        ),
        project,
        kind: field_str(&item.fields, "System.WorkItemType"),
        state: field_str(&item.fields, "System.State"),
        title: field_str(&item.fields, "System.Title"),
        by,
        at,
        snippet,
    }
}

/// The newest comment that mentions `me`: (author, date, snippet). None on any
/// failure, so one unreadable discussion doesn't hide the mention itself.
fn mentioning_comment(
    org: &str,
    auth: &Auth,
    project: &str,
    id: u64,
    me: &str,
) -> Option<(String, String, String)> {
    let mut res = agent()
        .get(&format!(
            "{HOST}/{}/{}/_apis/wit/workItems/{id}/comments?$top={COMMENTS_TO_SCAN}&order=desc&api-version=7.1-preview.4",
            encode(org),
            encode(project)
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .ok()?;
    let list: ApiComments = read_json(&mut res, org).ok()?;
    list.comments
        .into_iter()
        .find(|c| mentions(&c.text, me))
        .map(|c| (c.created_by.display_name, c.created_date, snippet(&c.text)))
}

/// Azure marks mentions as `<a data-vss-mention="version:2.0,{identity id}">@Name</a>`.
pub fn mentions(html: &str, identity_id: &str) -> bool {
    let html = html.to_ascii_lowercase();
    let id = identity_id.to_ascii_lowercase();
    html.match_indices("data-vss-mention=").any(|(i, _)| {
        html[i..]
            .split('>')
            .next()
            .is_some_and(|tag| tag.contains(&id))
    })
}

/// A comment as one line of plain text, cut at a word boundary.
pub fn snippet(html: &str) -> String {
    let text = strip_links(&html_to_markdown(html))
        .replace(['*', '`', '#'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() <= SNIPPET_CHARS {
        return text;
    }
    let cut: String = text.chars().take(SNIPPET_CHARS).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{cut}…")
}

/// `[text](url)` → `text`.
fn strip_links(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(open) = rest.find('[') {
        let Some(mid) = rest[open..].find("](").map(|i| open + i) else {
            break;
        };
        let Some(close) = rest[mid..].find(')').map(|i| mid + i) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..mid]);
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "6f1e2d3c-aaaa-bbbb-cccc-1234567890ab";

    #[test]
    fn finds_a_mention_of_this_user_only() {
        let html = format!(
            r##"<div><a href="#" data-vss-mention="version:2.0,{}">@Jonas</a> can you look?</div>"##,
            ME.to_uppercase()
        );
        assert!(mentions(&html, ME));
        let other = r##"<a href="#" data-vss-mention="version:2.0,0000">@Kari</a> and Jonas"##;
        assert!(!mentions(other, ME));
        assert!(!mentions(&format!("plain text with {ME}"), ME));
    }

    #[test]
    fn snippet_is_one_short_line_of_text() {
        let html = "<div><a href=\"#\">@Jonas</a> can you <b>check</b> the\n<br>login flow?</div>";
        assert_eq!(snippet(html), "@Jonas can you check the login flow?");
        let long = format!("<p>{}</p>", "word ".repeat(80));
        let s = snippet(&long);
        assert!(
            s.ends_with('…') && s.chars().count() <= SNIPPET_CHARS + 1,
            "{s}"
        );
    }

    #[test]
    fn query_uses_azures_recent_mentions() {
        assert!(MENTIONS_QUERY.contains("@RecentMentions"));
    }
}
