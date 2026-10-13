//! User story analysis: a note made from a template, filled in from a task and
//! linked back to it with `[[#id]]`, so it shows up under "Mentioned in".
//!
//! The template is markdown with placeholders:
//! - `{{title}}`, `{{id}}`, `{{project}}`, `{{date}}`, `{{author}}`
//! - `{{link}}`: a wiki link to the task, `[[#12]]`
//! - `{{source}}`: a link to the work item when the task was imported, else empty
//! - `{{description}}`: the task's description, or `_No description._`
//! - `{{acceptance}}`, `{{risk}}`: the description's "Acceptance criteria" and "Risk"
//!   sections (an Azure import writes them from the work item's fields), or an empty
//!   list item to fill in. With section splitting off in Settings, `{{description}}`
//!   keeps those sections and these two are always empty.

use rusqlite::OptionalExtension;

use crate::error::Result;
use crate::models::{Note, Task};
use crate::store::Store;

const TEMPLATE_KEY: &str = "analysis.template";
const SPLIT_KEY: &str = "analysis.split_sections";

/// Azure imports end the description with this footer; `{{source}}` links there instead.
const IMPORT_FOOTER: &str = "---\nAzure DevOps:";
const EMPTY_CHECKLIST: &str = "- [ ] ";
const EMPTY_LIST: &str = "- ";

/// Where new analyses go in the project's notes.
pub const ANALYSIS_FOLDER: &str = "Analysis";

/// Analysis notes are titled `Analysis: <task title>`.
const TITLE_PREFIX: &str = "Analysis: ";

/// The title an analysis of a task with this title gets.
pub fn analysis_title(task_title: &str) -> String {
    format!("{TITLE_PREFIX}{task_title}")
}

pub const DEFAULT_ANALYSIS_TEMPLATE: &str = "\
**Story:** {{link}}{{source}}

**Project:** {{project}} · **Analysed:** {{date}} by {{author}}

## The story

{{description}}

## Goal and value

Who is it for, and what changes for them when it's done?

## Acceptance criteria

{{acceptance}}

## Assumptions

-

## Dependencies

-

## Edge cases

-

## Risks

{{risk}}

## Open questions

-

## Suggested breakdown

- [ ]

## Estimate
";

/// Everything a template can refer to, already as text.
pub struct AnalysisContext<'a> {
    pub task: &'a Task,
    pub project: &'a str,
    pub date: &'a str,
    pub author: &'a str,
    pub source_url: Option<&'a str>,
    /// Move the description's acceptance criteria and risk sections into their own placeholders.
    pub split_sections: bool,
}

/// A task description cut into the parts a template can place separately.
#[derive(Debug, Default, PartialEq)]
pub struct StoryParts {
    pub description: String,
    pub acceptance: String,
    pub risk: String,
}

#[derive(Clone, Copy)]
enum Part {
    Description,
    Acceptance,
    Risk,
}

/// A markdown heading's level (number of `#`) and the part it starts, if it's one we split out.
fn heading(line: &str) -> Option<(usize, Option<Part>)> {
    let text = line.trim_start();
    let level = text.chars().take_while(|c| *c == '#').count();
    if level == 0 || !text[level..].starts_with([' ', '\t']) {
        return None;
    }
    let name = text[level..].trim().trim_end_matches(':').to_lowercase();
    let part = if matches!(name.as_str(), "acceptance criteria" | "akseptansekriterier") {
        Some(Part::Acceptance)
    } else if name.starts_with("risk") || name.starts_with("risiko") {
        Some(Part::Risk)
    } else {
        None
    };
    Some((level, part))
}

/// Splits a description into its main text and its "Acceptance criteria" and "Risk…"
/// sections, dropping the footer an Azure import adds. A section runs until the next
/// heading of the same or a higher level; what follows goes back to the main text.
pub fn split_story(body: &str) -> StoryParts {
    let body = without_footer(body);
    let mut parts: [Vec<&str>; 3] = Default::default();
    let mut current = (Part::Description, 0);
    for line in body.lines() {
        match heading(line) {
            Some((level, Some(part))) => current = (part, level),
            Some((level, None)) if level <= current.1 => {
                current = (Part::Description, 0);
                parts[Part::Description as usize].push(line);
            }
            _ => parts[current.0 as usize].push(line),
        }
    }
    let [description, acceptance, risk] = parts.map(|lines| lines.join("\n").trim().to_string());
    StoryParts {
        description,
        acceptance,
        risk,
    }
}

/// The description without the link footer an Azure import adds at the end.
fn without_footer(body: &str) -> &str {
    match body.rfind(IMPORT_FOOTER) {
        Some(i) if i == 0 || body[..i].ends_with('\n') => &body[..i],
        _ => body,
    }
}

fn or_default(text: String, fallback: &str) -> String {
    if text.is_empty() {
        fallback.to_string()
    } else {
        text
    }
}

/// Fills in the placeholders. Unknown `{{…}}` stay as they are.
pub fn render_analysis(template: &str, ctx: &AnalysisContext) -> String {
    let source = ctx
        .source_url
        .map(|url| format!(" · [Work item]({url})"))
        .unwrap_or_default();
    let parts = if ctx.split_sections {
        split_story(&ctx.task.body_md)
    } else {
        StoryParts {
            description: without_footer(&ctx.task.body_md).trim().to_string(),
            ..StoryParts::default()
        }
    };
    let description = or_default(parts.description, "_No description._");
    [
        ("{{title}}", ctx.task.title.clone()),
        ("{{id}}", ctx.task.id.to_string()),
        ("{{link}}", format!("[[#{}]]", ctx.task.id)),
        ("{{source}}", source),
        ("{{project}}", ctx.project.to_string()),
        ("{{date}}", ctx.date.to_string()),
        ("{{author}}", ctx.author.to_string()),
        ("{{description}}", description),
        (
            "{{acceptance}}",
            or_default(parts.acceptance, EMPTY_CHECKLIST),
        ),
        ("{{risk}}", or_default(parts.risk, EMPTY_LIST)),
    ]
    .iter()
    .fold(template.to_string(), |text, (key, value)| {
        text.replace(key, value)
    })
}

impl Store {
    /// The template from Settings, or the built-in one.
    pub fn analysis_template(&self) -> Result<String> {
        Ok(self
            .get_setting(TEMPLATE_KEY)?
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ANALYSIS_TEMPLATE.to_string()))
    }

    /// Saves a custom template; an empty one goes back to the built-in template.
    pub fn set_analysis_template(&mut self, template: &str) -> Result<()> {
        self.set_setting(TEMPLATE_KEY, template)
    }

    /// Whether `{{acceptance}}` and `{{risk}}` take those sections out of the description (on by default).
    pub fn analysis_split_sections(&self) -> Result<bool> {
        Ok(self.get_setting(SPLIT_KEY)?.as_deref() != Some("false"))
    }

    pub fn set_analysis_split_sections(&mut self, on: bool) -> Result<()> {
        self.set_setting(SPLIT_KEY, if on { "true" } else { "false" })
    }

    /// The task's analysis note, if one was made: a note in the same project
    /// that links to the task and has an analysis title.
    pub fn story_analysis(&self, task_id: i64) -> Result<Option<Note>> {
        let task = self.get_task(task_id)?;
        Ok(self
            .backlinks("task", task_id)?
            .into_iter()
            .find(|n| n.project_id == Some(task.project_id) && n.title.starts_with(TITLE_PREFIX)))
    }

    /// Makes the analysis note for a task, or returns the one it already has.
    /// Returns the note and whether it was just created.
    pub fn create_story_analysis(&mut self, task_id: i64) -> Result<(Note, bool)> {
        if let Some(existing) = self.story_analysis(task_id)? {
            return Ok((existing, false));
        }
        let task = self.get_task(task_id)?;
        let project = self.get_project(task.project_id)?;
        let source_url = self.external_link_for_task(task_id)?.map(|l| l.url);
        let date: String = self
            .conn
            .query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        let author = self.actor.clone();
        let split_sections = self.analysis_split_sections()?;
        let body = render_analysis(
            &self.analysis_template()?,
            &AnalysisContext {
                task: &task,
                project: &project.name,
                date: &date,
                author: &author,
                source_url: source_url.as_deref(),
                split_sections,
            },
        );
        let title = analysis_title(&task.title);
        let note = self.add_note(task.project_id, &title, &body)?;
        let note = self.set_note_folder(note.id, ANALYSIS_FOLDER)?;
        Ok((note, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    fn task_in(s: &mut Store, title: &str, body: &str) -> Task {
        let p = s
            .create_project(NewProject {
                name: "Bokost".into(),
                locale: None,
                description: String::new(),
                color: None,
                icon: None,
            })
            .unwrap();
        s.create_task(NewTask {
            project_id: p.id,
            title: title.into(),
            body_md: body.into(),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn renders_placeholders_and_leaves_unknown_ones() {
        let (mut s, _d) = store();
        let task = task_in(&mut s, "Login with BankID", "As a user I want to log in.");
        let out = render_analysis(
            "# {{title}} ({{id}})\n{{link}}{{source}}\n{{description}}\n{{project}} {{date}} {{author}} {{nope}}",
            &AnalysisContext {
                task: &task,
                project: "Bokost",
                date: "2026-10-13",
                author: "jonas",
                source_url: Some("https://dev.azure.com/c/p/_workitems/edit/7"),
                split_sections: true,
            },
        );
        assert_eq!(
            out,
            format!(
                "# Login with BankID ({id})\n[[#{id}]] · [Work item](https://dev.azure.com/c/p/_workitems/edit/7)\nAs a user I want to log in.\nBokost 2026-10-13 jonas {{{{nope}}}}",
                id = task.id
            )
        );
    }

    #[test]
    fn empty_description_says_so() {
        let (mut s, _d) = store();
        let task = task_in(&mut s, "Export", "  ");
        let ctx = AnalysisContext {
            task: &task,
            project: "P",
            date: "d",
            author: "a",
            source_url: None,
            split_sections: true,
        };
        assert_eq!(
            render_analysis("{{description}}|{{source}}", &ctx),
            "_No description._|"
        );
    }

    const IMPORTED: &str = "App crashes on login.\n\n### Acceptance criteria\n\n- No crash\n\n\
        ### Risk description\n\nLogin is shared\n\n---\nAzure DevOps: [Bug 7](https://x) · Active · App";

    #[test]
    fn splits_an_imported_description_into_its_sections() {
        assert_eq!(
            split_story(IMPORTED),
            StoryParts {
                description: "App crashes on login.".into(),
                acceptance: "- No crash".into(),
                risk: "Login is shared".into(),
            }
        );
        let written = "Intro\n\n## Akseptansekriterier:\n- [ ] A\n\n### Edge\n- kept\n\n## Notes\nback to the text\n\n# Risks\n- B";
        let parts = split_story(written);
        assert_eq!(parts.description, "Intro\n\n## Notes\nback to the text");
        assert_eq!(parts.acceptance, "- [ ] A\n\n### Edge\n- kept");
        assert_eq!(parts.risk, "- B");
        // An analysis note made from the template: later sections don't run into the criteria.
        let note = "**Story:** [[#1]]\n\n## Acceptance criteria\n\n- Works\n\n## Assumptions\n\n- None\n\n## Risks\n\n- Slow";
        let parts = split_story(note);
        assert_eq!(
            (parts.acceptance.as_str(), parts.risk.as_str()),
            ("- Works", "- Slow")
        );
        assert!(parts.description.contains("## Assumptions"));
        assert_eq!(split_story("#hashtag").description, "#hashtag");
        assert_eq!(split_story("Just text").description, "Just text");
    }

    #[test]
    fn acceptance_and_risk_placeholders_follow_the_setting() {
        let (mut s, _d) = store();
        let task = task_in(&mut s, "Login", IMPORTED);
        let template = "{{description}}|{{acceptance}}|{{risk}}";
        let mut ctx = AnalysisContext {
            task: &task,
            project: "P",
            date: "d",
            author: "a",
            source_url: None,
            split_sections: true,
        };
        assert_eq!(
            render_analysis(template, &ctx),
            "App crashes on login.|- No crash|Login is shared"
        );
        ctx.split_sections = false;
        assert_eq!(
            render_analysis(template, &ctx),
            "App crashes on login.\n\n### Acceptance criteria\n\n- No crash\n\n### Risk description\n\nLogin is shared|- [ ] |- "
        );
        assert!(s.analysis_split_sections().unwrap());
        s.set_analysis_split_sections(false).unwrap();
        assert!(!s.analysis_split_sections().unwrap());
    }

    #[test]
    fn creates_a_linked_note_in_the_analysis_folder_once() {
        let (mut s, _d) = store();
        let task = task_in(&mut s, "Login with BankID", "As a user I want to log in.");
        let (note, created) = s.create_story_analysis(task.id).unwrap();
        assert!(created);
        assert_eq!(note.title, "Analysis: Login with BankID");
        assert_eq!(note.folder, "Analysis");
        assert_eq!(note.project_id, Some(task.project_id));
        assert!(note.body_md.contains(&format!("[[#{}]]", task.id)));
        assert!(note.body_md.contains("## Acceptance criteria"));
        assert_eq!(s.backlinks("task", task.id).unwrap()[0].id, note.id);

        let (again, created) = s.create_story_analysis(task.id).unwrap();
        assert!(!created);
        assert_eq!(again.id, note.id);
    }

    #[test]
    fn custom_template_and_reset() {
        let (mut s, _d) = store();
        assert_eq!(s.analysis_template().unwrap(), DEFAULT_ANALYSIS_TEMPLATE);
        s.set_analysis_template("Story {{link}}\n\n## Notes")
            .unwrap();
        let task = task_in(&mut s, "Export", "");
        let (note, _) = s.create_story_analysis(task.id).unwrap();
        assert_eq!(note.body_md, format!("Story [[#{}]]\n\n## Notes", task.id));
        s.set_analysis_template("   ").unwrap();
        assert_eq!(s.analysis_template().unwrap(), DEFAULT_ANALYSIS_TEMPLATE);
    }
}
