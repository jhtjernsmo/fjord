//! Beta, off by default: writing a user story analysis back to its Azure Boards
//! work item. Only the acceptance criteria and risk fields change, and only after
//! the user has seen a preview. The update carries the revision the preview was
//! made from, so it fails instead of overwriting someone else's newer edit.

use pulldown_cmark::{Event, Options, Parser, html};
use serde::{Deserialize, Serialize};

use crate::azure::{
    API_VERSION, ApiWorkItem, HOST, api_error, check_org, encode, field_str, find_auth,
    html_to_markdown, is_built_in_risk, is_risk_name, read_json,
};
use crate::github::{HttpError, agent, checked};
use crate::{Result, VcsError};

pub const ACCEPTANCE_FIELD: &str = "Microsoft.VSTS.Common.AcceptanceCriteria";

/// Lines a template leaves for filling in; a section with only these is empty.
const EMPTY_LINES: &[&str] = &["- [ ]", "-", "*"];

/// One field the update would change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    /// Reference name, e.g. `Microsoft.VSTS.Common.AcceptanceCriteria`.
    pub field: String,
    /// What the work item type calls it, e.g. "Acceptance Criteria".
    pub label: String,
    pub current_md: String,
    pub new_md: String,
}

/// What sending the analysis would do.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdatePreview {
    pub org: String,
    pub id: u64,
    /// The work item's title in Azure, so the user can see it's the right one.
    pub title: String,
    pub url: String,
    /// The revision the preview was made from; the update only applies on top of it.
    pub rev: i64,
    pub changes: Vec<FieldChange>,
    /// Sections that can't be sent, and why.
    pub skipped: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiField {
    reference_name: String,
    name: String,
}

#[derive(Deserialize)]
struct ApiFields {
    value: Vec<ApiField>,
}

/// Markdown → the HTML Azure stores in rich text fields. Raw HTML in the
/// markdown is shown as text rather than passed through.
pub fn markdown_to_html(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let events = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out.trim_end().to_string()
}

/// The section's text, or empty if it only has the template's blank list items.
pub fn filled_in(section: &str) -> String {
    let has_content = section
        .lines()
        .map(str::trim)
        .any(|l| !l.is_empty() && !EMPTY_LINES.contains(&l));
    if has_content {
        section.trim().to_string()
    } else {
        String::new()
    }
}

/// The field that takes the risk description: the one named in the import
/// settings, else a free-text field on the work item type with "risk" in its name.
fn risk_target<'a>(fields: &'a [ApiField], preferred: &str) -> Option<&'a ApiField> {
    let preferred = preferred.trim();
    if !preferred.is_empty() {
        return fields
            .iter()
            .find(|f| f.reference_name.eq_ignore_ascii_case(preferred));
    }
    fields
        .iter()
        .find(|f| is_risk_name(&f.reference_name) && !is_built_in_risk(&f.reference_name))
}

fn same_text(current_html: &str, new_md: &str) -> bool {
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    normalize(&html_to_markdown(current_html)) == normalize(new_md)
        || normalize(current_html) == normalize(&markdown_to_html(new_md))
}

fn get_json<T: serde::de::DeserializeOwned>(org: &str, url: &str) -> Result<T> {
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let mut res = agent()
        .get(url)
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| api_error(e, org))?;
    read_json(&mut res, org)
}

/// Compares the analysis with the work item as it is now. Network only.
pub fn preview_update(
    org: &str,
    id: u64,
    acceptance_md: &str,
    risk_md: &str,
    risk_field: &str,
) -> Result<UpdatePreview> {
    let org = check_org(org)?;
    let item: ApiWorkItem = get_json(
        org,
        &format!(
            "{HOST}/{}/_apis/wit/workitems/{id}?api-version={API_VERSION}",
            encode(org)
        ),
    )?;
    let project = field_str(&item.fields, "System.TeamProject");
    let kind = field_str(&item.fields, "System.WorkItemType");
    let type_fields: ApiFields = get_json(
        org,
        &format!(
            "{HOST}/{}/{}/_apis/wit/workitemtypes/{}/fields?api-version={API_VERSION}",
            encode(org),
            encode(&project),
            encode(&kind)
        ),
    )?;

    let mut changes = Vec::new();
    let mut skipped = Vec::new();
    let mut consider = |field: Option<&ApiField>, new_md: &str, missing: String| {
        let new_md = filled_in(new_md);
        if new_md.is_empty() {
            return;
        }
        let Some(field) = field else {
            skipped.push(missing);
            return;
        };
        let current_html = field_str(&item.fields, &field.reference_name);
        if !same_text(&current_html, &new_md) {
            changes.push(FieldChange {
                field: field.reference_name.clone(),
                label: field.name.clone(),
                current_md: html_to_markdown(&current_html),
                new_md,
            });
        }
    };
    let acceptance = type_fields
        .value
        .iter()
        .find(|f| f.reference_name == ACCEPTANCE_FIELD);
    consider(
        acceptance,
        acceptance_md,
        format!("{kind} has no Acceptance Criteria field"),
    );
    consider(
        risk_target(&type_fields.value, risk_field),
        risk_md,
        format!("No free-text risk field on {kind}; name it under Settings → Integrations"),
    );

    Ok(UpdatePreview {
        org: org.to_string(),
        id,
        title: field_str(&item.fields, "System.Title"),
        url: format!(
            "{HOST}/{}/{}/_workitems/edit/{id}",
            encode(org),
            encode(&project)
        ),
        rev: item.rev(),
        changes,
        skipped,
    })
}

/// The JSON Patch for an update: fails unless the work item is still at `rev`.
fn patch_body(rev: i64, changes: &[FieldChange]) -> serde_json::Value {
    let mut ops = vec![serde_json::json!({ "op": "test", "path": "/rev", "value": rev })];
    ops.extend(changes.iter().map(|c| {
        serde_json::json!({
            "op": "add",
            "path": format!("/fields/{}", c.field),
            "value": markdown_to_html(&c.new_md),
        })
    }));
    serde_json::Value::Array(ops)
}

/// Only the acceptance criteria and a risk field (the configured one, or one named
/// like a risk field) may be written.
fn writable(field: &str, risk_field: &str) -> bool {
    field == ACCEPTANCE_FIELD
        || (!risk_field.trim().is_empty() && field.eq_ignore_ascii_case(risk_field.trim()))
        || (is_risk_name(field) && !is_built_in_risk(field))
}

/// Writes the changes, if the work item hasn't changed since `rev`. Returns the new revision.
pub fn apply_update(
    org: &str,
    id: u64,
    rev: i64,
    changes: &[FieldChange],
    risk_field: &str,
) -> Result<i64> {
    let org = check_org(org)?;
    if let Some(c) = changes.iter().find(|c| !writable(&c.field, risk_field)) {
        return Err(VcsError::Azure(format!(
            "Fjord only updates acceptance criteria and risk, not {}",
            c.field
        )));
    }
    if changes.is_empty() {
        return Ok(rev);
    }
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let body = patch_body(rev, changes).to_string();
    let res = agent()
        .patch(&format!(
            "{HOST}/{}/_apis/wit/workitems/{id}?api-version={API_VERSION}",
            encode(org)
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .content_type("application/json-patch+json")
        .send(body.as_str())
        .map_err(HttpError::from)
        .and_then(checked);
    let mut res = match res {
        Ok(res) => res,
        // A failed "test" on /rev: someone saved the work item after the preview.
        Err(HttpError::Status(400 | 409 | 412, Some(m)))
            if m.contains("rev") || m.contains("TF26071") =>
        {
            return Err(VcsError::Azure(
                "the work item changed in Azure after the preview; open the preview again".into(),
            ));
        }
        Err(e) => return Err(api_error(e, org)),
    };
    let item: ApiWorkItem = read_json(&mut res, org)?;
    Ok(item.rev())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(reference: &str, name: &str) -> ApiField {
        ApiField {
            reference_name: reference.into(),
            name: name.into(),
        }
    }

    #[test]
    fn markdown_becomes_html_without_passing_raw_html_through() {
        assert_eq!(
            markdown_to_html("- **Two**"),
            "<ul>\n<li><strong>Two</strong></li>\n</ul>"
        );
        assert_eq!(
            markdown_to_html("Hi <script>x</script>"),
            "<p>Hi &lt;script&gt;x&lt;/script&gt;</p>"
        );
    }

    #[test]
    fn template_blanks_count_as_empty() {
        assert_eq!(filled_in("- [ ] \n\n- "), "");
        assert_eq!(filled_in("\n- [ ] Works offline\n"), "- [ ] Works offline");
    }

    #[test]
    fn risk_goes_to_the_named_field_or_a_free_text_one() {
        let fields = vec![
            field("Microsoft.VSTS.Common.Risk", "Risk"),
            field("Custom.RiskDescription", "Risk Description"),
            field("System.Title", "Title"),
        ];
        assert_eq!(
            risk_target(&fields, "").map(|f| f.name.as_str()),
            Some("Risk Description")
        );
        assert_eq!(
            risk_target(&fields, "system.title").map(|f| f.name.as_str()),
            Some("Title")
        );
        assert!(risk_target(&fields[..1], "").is_none());
    }

    #[test]
    fn only_acceptance_and_risk_fields_are_writable() {
        assert!(writable(ACCEPTANCE_FIELD, ""));
        assert!(writable("Custom.RiskDescription", ""));
        assert!(writable("Custom.Notes", "custom.notes"));
        assert!(!writable("System.Title", ""));
        assert!(!writable("Microsoft.VSTS.Common.Risk", ""));
        let title = FieldChange {
            field: "System.Title".into(),
            label: "Title".into(),
            current_md: String::new(),
            new_md: "x".into(),
        };
        assert!(apply_update("contoso", 1, 1, &[title], "").is_err());
    }

    #[test]
    fn unchanged_text_is_not_sent_again() {
        assert!(same_text("<ul><li>No crash</li></ul>", "- No crash"));
        assert!(!same_text(
            "<ul><li>No crash</li></ul>",
            "- No crash\n- Fast"
        ));
    }

    #[test]
    fn the_patch_tests_the_revision_first() {
        let change = FieldChange {
            field: ACCEPTANCE_FIELD.into(),
            label: "Acceptance Criteria".into(),
            current_md: String::new(),
            new_md: "- Done".into(),
        };
        let body = patch_body(7, &[change]);
        assert_eq!(
            body[0],
            serde_json::json!({ "op": "test", "path": "/rev", "value": 7 })
        );
        assert_eq!(
            body[1]["path"],
            "/fields/Microsoft.VSTS.Common.AcceptanceCriteria"
        );
        assert_eq!(body[1]["value"], "<ul>\n<li>Done</li>\n</ul>");
    }
}
