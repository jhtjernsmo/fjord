//! User story analysis: a note made from a template, filled in from a task and
//! linked back to it with `[[#id]]`, so it shows up under "Mentioned in".
//!
//! The template is markdown with placeholders:
//! - `{{title}}`, `{{id}}`, `{{project}}`, `{{date}}`, `{{author}}`
//! - `{{link}}`: a wiki link to the task, `[[#12]]`
//! - `{{source}}`: a link to the work item when the task was imported, else empty
//! - `{{description}}`: the task's description, or `_No description._`

use rusqlite::OptionalExtension;

use crate::error::Result;
use crate::models::{Note, Task};
use crate::store::Store;

const TEMPLATE_KEY: &str = "analysis.template";

/// Where new analyses go in the project's notes.
pub const ANALYSIS_FOLDER: &str = "Analysis";

/// Analysis notes are titled `Analysis: <task title>`.
const TITLE_PREFIX: &str = "Analysis: ";

pub const DEFAULT_ANALYSIS_TEMPLATE: &str = "\
**Story:** {{link}}{{source}}

**Project:** {{project}} · **Analysed:** {{date}} by {{author}}

## The story

{{description}}

## Goal and value

Who is it for, and what changes for them when it's done?

## Acceptance criteria

- [ ]

## Assumptions

-

## Dependencies

-

## Edge cases

-

## Risks

-

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
}

/// Fills in the placeholders. Unknown `{{…}}` stay as they are.
pub fn render_analysis(template: &str, ctx: &AnalysisContext) -> String {
    let description = ctx.task.body_md.trim();
    let source = ctx
        .source_url
        .map(|url| format!(" · [Work item]({url})"))
        .unwrap_or_default();
    let description = if description.is_empty() {
        "_No description._".to_string()
    } else {
        description.to_string()
    };
    [
        ("{{title}}", ctx.task.title.clone()),
        ("{{id}}", ctx.task.id.to_string()),
        ("{{link}}", format!("[[#{}]]", ctx.task.id)),
        ("{{source}}", source),
        ("{{project}}", ctx.project.to_string()),
        ("{{date}}", ctx.date.to_string()),
        ("{{author}}", ctx.author.to_string()),
        ("{{description}}", description),
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
        let body = render_analysis(
            &self.analysis_template()?,
            &AnalysisContext {
                task: &task,
                project: &project.name,
                date: &date,
                author: &author,
                source_url: source_url.as_deref(),
            },
        );
        let title = format!("{TITLE_PREFIX}{}", task.title);
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
        };
        assert_eq!(
            render_analysis("{{description}}|{{source}}", &ctx),
            "_No description._|"
        );
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
