//! Human-readable or JSON output for the CLI.

use anyhow::Result;
use fjord_core::{Activity, Attachment, Note, Project, ProjectSummary, SearchHit, Status, Task};
use serde::Serialize;

const PRIORITY: [&str; 4] = ["", "↓ low", "→ medium", "↑ high"];

pub struct Printer {
    pub json: bool,
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn progress_bar(done: i64, total: i64) -> String {
    const WIDTH: i64 = 10;
    let filled = if total == 0 { 0 } else { done * WIDTH / total };
    format!("{}{}", "█".repeat(filled as usize), "░".repeat((WIDTH - filled) as usize))
}

fn task_line(t: &Task) -> String {
    let mut line = format!("#{:<4} {}", t.id, t.title);
    if t.priority > 0 {
        line.push_str(&format!("  {}", PRIORITY[t.priority as usize]));
    }
    if let Some(due) = &t.due_at {
        line.push_str(&format!("  due {due}"));
    }
    line
}

/// English sentence for an activity entry (the GUI localizes its own).
fn describe(a: &Activity) -> String {
    let subject = &a.subject;
    let detail = a.detail.as_deref().unwrap_or("");
    match a.action.as_str() {
        "project.create" => format!("created project “{subject}”"),
        "project.update" => format!("updated project “{subject}”"),
        "project.archive" => format!("archived project “{subject}”"),
        "project.restore" => format!("restored project “{subject}”"),
        "task.create" => format!("added “{subject}”"),
        "task.update" => format!("updated “{subject}”"),
        "task.move" => format!("moved “{subject}” to {detail}"),
        "task.archive" => format!("archived “{subject}”"),
        "task.restore" => format!("restored “{subject}”"),
        "file.attach" => format!("attached “{subject}”"),
        "file.detach" => format!("removed file “{subject}”"),
        "note.create" => format!("wrote note “{subject}”"),
        "note.update" => format!("updated note “{subject}”"),
        other => format!("{other} “{subject}”"),
    }
}

impl Printer {
    pub fn projects(&self, list: &[ProjectSummary]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        if list.is_empty() {
            println!("No projects yet. Create one with: fjord project add \"Name\"");
        }
        for s in list {
            let p = &s.project;
            let archived = if p.archived_at.is_some() { "  (archived)" } else { "" };
            let overdue = if s.overdue_count > 0 { format!("  ! {} overdue", s.overdue_count) } else { String::new() };
            println!(
                "{} {:<24} {} {}/{}{}{}  [{}]",
                p.icon,
                p.name,
                progress_bar(s.done_count, s.task_count),
                s.done_count,
                s.task_count,
                overdue,
                archived,
                p.slug
            );
        }
        Ok(())
    }

    pub fn project(&self, p: &Project) -> Result<()> {
        if self.json {
            return print_json(p);
        }
        println!("{} {}  [{}]  #{}", p.icon, p.name, p.slug, p.id);
        if !p.description.is_empty() {
            println!("{}", p.description);
        }
        Ok(())
    }

    pub fn board(&self, p: &Project, statuses: &[Status], tasks: &[Task]) -> Result<()> {
        if self.json {
            #[derive(Serialize)]
            struct Column<'a> {
                status: &'a Status,
                tasks: Vec<&'a Task>,
            }
            #[derive(Serialize)]
            struct Board<'a> {
                project: &'a Project,
                columns: Vec<Column<'a>>,
            }
            let columns = statuses
                .iter()
                .map(|s| Column { status: s, tasks: tasks.iter().filter(|t| t.status_id == s.id).collect() })
                .collect();
            return print_json(&Board { project: p, columns });
        }
        self.project(p)?;
        for s in statuses {
            let in_column: Vec<&Task> = tasks.iter().filter(|t| t.status_id == s.id).collect();
            println!("\n── {} ({}) ──", s.name, in_column.len());
            for t in in_column {
                println!("  {}", task_line(t));
            }
        }
        Ok(())
    }

    pub fn task(&self, t: &Task) -> Result<()> {
        if self.json {
            return print_json(t);
        }
        println!("{}", task_line(t));
        if !t.body_md.is_empty() {
            println!("\n{}", t.body_md);
        }
        Ok(())
    }

    pub fn attachment(&self, a: &Attachment) -> Result<()> {
        if self.json {
            return print_json(a);
        }
        println!("[file] #{} {} ({} B)", a.id, a.original_name, a.size);
        Ok(())
    }

    pub fn attachments(&self, list: &[Attachment]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        if list.is_empty() {
            println!("No files.");
        }
        list.iter().try_for_each(|a| self.attachment(a))
    }

    pub fn note(&self, n: &Note) -> Result<()> {
        if self.json {
            return print_json(n);
        }
        println!("[note] #{} {}", n.id, n.title);
        Ok(())
    }

    pub fn search(&self, hits: &[SearchHit]) -> Result<()> {
        if self.json {
            return print_json(hits);
        }
        if hits.is_empty() {
            println!("No matches.");
        }
        for h in hits {
            let icon = match h.kind.as_str() {
                "task" => "[task]",
                "note" => "[note]",
                _ => "[file]",
            };
            println!("{icon} #{} {}  {}", h.ref_id, h.title, h.snippet);
        }
        Ok(())
    }

    pub fn activity(&self, list: &[Activity]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        for a in list {
            let when = a.created_at.get(..16).unwrap_or(&a.created_at).replace('T', " ");
            println!("{when}  {} {}", a.actor, describe(a));
        }
        Ok(())
    }
}
