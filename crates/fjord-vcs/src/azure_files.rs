//! Files on Azure Boards work items: attachments and images pasted into the
//! description. Imports download each one once and attach it to the task.
//!
//! The token only ever goes to the work item's own organization: a file URL is
//! fetched only if it's an attachment URL under `https://dev.azure.com/<org>/`.

use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::azure::{HOST, api_error, find_auth};
use crate::github::{HttpError, agent, checked};
use crate::{Result, VcsError};

/// Larger files are skipped rather than stored.
pub const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;
const ATTACHMENTS_PATH: &str = "/_apis/wit/attachments/";
const DEFAULT_IMAGE_NAME: &str = "image.png";
/// A GUID without braces is 36 characters; 32 without dashes.
const MIN_GUID_LEN: usize = 32;

/// A file on a work item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AzureFile {
    /// The attachment's GUID, lowercase; the same file pasted twice has one id.
    pub id: String,
    pub name: String,
    pub url: String,
}

/// A link from a work item to something else; attachments are `AttachedFile`.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ApiRelation {
    pub(crate) rel: String,
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) attributes: serde_json::Map<String, serde_json::Value>,
}

/// The GUID in an attachment URL, if `url` is one in `org`.
pub fn attachment_id(org: &str, url: &str) -> Option<String> {
    let prefix = format!("{HOST}/{org}/");
    let head = url.get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(&prefix) {
        return None;
    }
    let (_, after) = url.split_once(ATTACHMENTS_PATH)?;
    let id: String = after
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect();
    (id.len() >= MIN_GUID_LEN).then(|| id.to_lowercase())
}

/// `fileName` from an attachment URL's query, percent-decoded.
fn file_name_param(url: &str) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("fileName="))
        .map(percent_decode)
        .filter(|n| !n.trim().is_empty())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| s.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (hex, bytes[i]) {
            (Some(b), _) => {
                out.push(b);
                i += 3;
                continue;
            }
            (None, b'+') => out.push(b' '),
            (None, b) => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `src` of every `<img>` in `html`, with `&amp;` decoded.
fn image_sources(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<img") {
        let tag_end = rest[start..].find('>').map_or(rest.len(), |e| start + e);
        let tag = &rest[start..tag_end];
        if let Some(src) = tag.split("src=\"").nth(1).and_then(|s| s.split('"').next()) {
            out.push(src.replace("&amp;", "&"));
        }
        rest = &rest[tag_end..];
    }
    out
}

/// The work item's files: its attachments, then images pasted into `html_fields`
/// that aren't attachments too. Files outside `org` are left out.
pub(crate) fn work_item_files(
    org: &str,
    relations: &[ApiRelation],
    html_fields: &[&str],
) -> Vec<AzureFile> {
    let mut files: Vec<AzureFile> = Vec::new();
    let mut push = |url: &str, name: Option<String>| {
        let Some(id) = attachment_id(org, url) else {
            return;
        };
        if files.iter().any(|f| f.id == id) {
            return;
        }
        let name = name
            .or_else(|| file_name_param(url))
            .unwrap_or_else(|| DEFAULT_IMAGE_NAME.to_string());
        files.push(AzureFile {
            id,
            name,
            url: url.to_string(),
        });
    };
    for r in relations.iter().filter(|r| r.rel == "AttachedFile") {
        let name = r
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        push(&r.url, name);
    }
    for html in html_fields {
        for src in image_sources(html) {
            push(&src, None);
        }
    }
    files
}

/// Downloads one file from the work item's organization.
pub fn download_file(org: &str, file: &AzureFile) -> Result<Vec<u8>> {
    if attachment_id(org, &file.url).as_deref() != Some(file.id.as_str()) {
        return Err(VcsError::Azure(format!(
            "not an attachment in «{org}»: {}",
            file.url
        )));
    }
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let mut res = agent()
        .get(&file.url)
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| api_error(e, org))?;
    // A bad token gets an HTML sign-in page with status 203 instead of the file.
    if res.status().as_u16() == 203 {
        return Err(VcsError::Azure(format!(
            "Azure DevOps did not accept the token for «{org}»"
        )));
    }
    let mut bytes = Vec::new();
    res.body_mut()
        .as_reader()
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| VcsError::Azure(e.to_string()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(VcsError::Azure(format!(
            "{} is larger than {} MB",
            file.name,
            MAX_FILE_BYTES / 1024 / 1024
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUID: &str = "3F2504E0-4F89-11D3-9A0C-0305E82C3301";

    #[test]
    fn attachment_ids_only_come_from_the_same_org() {
        let url =
            format!("https://dev.azure.com/contoso/_apis/wit/attachments/{GUID}?fileName=a.png");
        let id = GUID.to_lowercase();
        assert_eq!(attachment_id("contoso", &url).as_deref(), Some(id.as_str()));
        assert_eq!(attachment_id("Contoso", &url).as_deref(), Some(id.as_str()));
        assert_eq!(attachment_id("fabrikam", &url), None);
        let elsewhere = format!("https://evil.example/contoso/_apis/wit/attachments/{GUID}");
        assert_eq!(attachment_id("contoso", &elsewhere), None);
        let short = "https://dev.azure.com/contoso/_apis/wit/attachments/abc";
        assert_eq!(attachment_id("contoso", short), None);
        let lookalike =
            format!("https://dev.azure.com/contoso.evil.example/_apis/wit/attachments/{GUID}");
        assert_eq!(attachment_id("contoso", &lookalike), None);
    }

    #[test]
    fn collects_attachments_and_pasted_images_once() {
        let attached = format!("https://dev.azure.com/contoso/a1b2/_apis/wit/attachments/{GUID}");
        let relations = vec![
            ApiRelation {
                rel: "AttachedFile".into(),
                url: attached.clone(),
                attributes: serde_json::from_str(r#"{"name":"log.txt"}"#).unwrap(),
            },
            ApiRelation {
                rel: "System.LinkTypes.Hierarchy-Reverse".into(),
                url: "https://dev.azure.com/contoso/_apis/wit/workItems/1".into(),
                ..Default::default()
            },
        ];
        let other = "11111111-2222-3333-4444-555555555555";
        let html = format!(
            r#"<p>See <img src="{attached}?fileName=dup.png"> and <img alt="x" src="https://dev.azure.com/contoso/_apis/wit/attachments/{other}?fileName=Skjerm%20bilde.png&amp;download=false"> and <img src="https://example.com/cat.png"></p>"#
        );
        let files = work_item_files("contoso", &relations, &[&html, ""]);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["log.txt", "Skjerm bilde.png"]);
        assert_eq!(files[1].id, other);
        assert!(files[1].url.ends_with("&download=false"));
    }

    #[test]
    fn pasted_images_without_a_name_get_a_default() {
        let html =
            format!(r#"<img src="https://dev.azure.com/contoso/_apis/wit/attachments/{GUID}">"#);
        assert_eq!(
            work_item_files("contoso", &[], &[&html])[0].name,
            "image.png"
        );
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("%C3%A6+b"), "æ b");
    }
}
