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
    format!(
        "{}{}",
        "█".repeat(filled as usize),
        "░".repeat((WIDTH - filled) as usize)
    )
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
        "status.create" => format!("added column “{subject}”"),
        "status.update" => format!("updated column “{subject}”"),
        "status.move" => format!("moved column “{subject}”"),
        "status.delete" => format!("removed column “{subject}”"),
        "repo.link" => format!("linked “{subject}” to {detail}"),
        "repo.unlink" => format!("unlinked the repository from “{subject}”"),
        "task.branch" => format!("started branch {detail} for “{subject}”"),
        other => format!("{other} “{subject}”"),
    }
}

impl Printer {
    pub fn value<T: Serialize>(&self, v: &T) -> Result<()> {
        if self.json {
            return print_json(v);
        }
        println!("{}", serde_json::to_string_pretty(v)?);
        Ok(())
    }

    pub fn message(&self, text: &str) -> Result<()> {
        if self.json {
            return print_json(&serde_json::json!({ "ok": true, "message": text }));
        }
        println!("{text}");
        Ok(())
    }

    pub fn git_overview(&self, o: &fjord_vcs::GitOverview) -> Result<()> {
        if self.json {
            return print_json(o);
        }
        let gh = match (&o.repo.github_owner, &o.repo.github_repo) {
            (Some(a), Some(b)) => format!("  github.com/{a}/{b}"),
            _ => String::new(),
        };
        println!("{}{gh}", o.repo.path);
        println!(
            "on {}{}\n",
            o.current_branch,
            if o.dirty {
                "  (uncommitted changes)"
            } else {
                ""
            }
        );
        for b in &o.branches {
            let mark = if b.current { "*" } else { " " };
            let track = match (b.ahead, b.behind) {
                (0, 0) => String::new(),
                (a, 0) => format!("  ↑{a}"),
                (0, z) => format!("  ↓{z}"),
                (a, z) => format!("  ↑{a} ↓{z}"),
            };
            println!("{mark} {:<36} {}{track}", b.name, b.sha);
        }
        println!();
        for c in &o.commits {
            println!("{}  {}  ({})", c.sha, c.subject, c.author);
        }
        Ok(())
    }

    pub fn started_branch(&self, b: &fjord_vcs::StartedBranch) -> Result<()> {
        if self.json {
            return print_json(b);
        }
        let verb = if b.created {
            "Created and switched to"
        } else {
            "Switched to"
        };
        println!("{verb} {} for #{} {}", b.branch, b.task.id, b.task.title);
        Ok(())
    }

    pub fn pull_request(&self, pr: &fjord_vcs::PullRequest) -> Result<()> {
        if self.json {
            return print_json(pr);
        }
        println!("#{} {}  [{}]  {}", pr.number, pr.title, pr.state, pr.url);
        Ok(())
    }

    pub fn sync_report(&self, r: &fjord_vcs::SyncReport) -> Result<()> {
        if self.json {
            return print_json(r);
        }
        if r.pull_requests.is_empty() {
            println!("No pull requests.");
        }
        for lp in &r.pull_requests {
            let checks = match lp.pr.checks {
                fjord_vcs::Checks::Success => "✓",
                fjord_vcs::Checks::Failure => "✗",
                fjord_vcs::Checks::Pending => "…",
                fjord_vcs::Checks::None => " ",
            };
            let task = lp
                .task_id
                .map(|id| format!("  task #{id}"))
                .unwrap_or_default();
            println!(
                "{checks} #{:<5} {:<8} {}  ({}){task}",
                lp.pr.number, lp.pr.state, lp.pr.title, lp.pr.head
            );
        }
        for t in &r.completed {
            println!("→ moved #{} {} to done (PR merged)", t.id, t.title);
        }
        Ok(())
    }

    pub fn projects(&self, list: &[ProjectSummary]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        if list.is_empty() {
            println!("No projects yet. Create one with: fjord project add \"Name\"");
        }
        for s in list {
            let p = &s.project;
            let archived = if p.archived_at.is_some() {
                "  (archived)"
            } else {
                ""
            };
            let overdue = if s.overdue_count > 0 {
                format!("  ! {} overdue", s.overdue_count)
            } else {
                String::new()
            };
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
                .map(|s| Column {
                    status: s,
                    tasks: tasks.iter().filter(|t| t.status_id == s.id).collect(),
                })
                .collect();
            return print_json(&Board {
                project: p,
                columns,
            });
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

    pub fn tasks(&self, list: &[Task]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        if list.is_empty() {
            println!("No tasks.");
        }
        list.iter().for_each(|t| println!("{}", task_line(t)));
        Ok(())
    }

    pub fn statuses(&self, list: &[Status]) -> Result<()> {
        if self.json {
            return print_json(list);
        }
        for s in list {
            let done = if s.is_done { "  (done)" } else { "" };
            println!(
                "#{:<4} {}. {}  {}{}",
                s.id, s.position, s.name, s.color, done
            );
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
            let when = a
                .created_at
                .get(..16)
                .unwrap_or(&a.created_at)
                .replace('T', " ");
            println!("{when}  {} {}", a.actor, describe(a));
        }
        Ok(())
    }
}
