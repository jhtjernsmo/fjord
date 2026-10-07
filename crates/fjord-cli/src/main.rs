//! `fjord` — command-line access to Fjord, for humans, scripts and AI agents.

mod output;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fjord_core::{NewProject, NewTask, ProjectPatch, Store, TaskPatch};

#[derive(Parser)]
#[command(name = "fjord", version, about = "Lokal prosjektstyring — CLI")]
struct Cli {
    /// Who is making changes (shown in the activity log). Default: $FJORD_ACTOR or $USER.
    #[arg(long, global = true)]
    actor: Option<String>,
    /// Data directory. Default: $FJORD_DATA_DIR or ~/.local/share/fjord.
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Print JSON instead of text (for scripts and agents).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage projects
    #[command(subcommand, alias = "p")]
    Project(ProjectCmd),
    /// Manage tasks
    #[command(subcommand, alias = "t")]
    Task(TaskCmd),
    /// Attach a file to a project (optionally to a task)
    Attach {
        project: String,
        path: PathBuf,
        #[arg(long)]
        task: Option<i64>,
    },
    /// List files of a project
    Files {
        project: String,
        #[arg(long)]
        task: Option<i64>,
    },
    /// Add a note to a project
    Note {
        project: String,
        title: String,
        #[arg(long, default_value = "")]
        body: String,
    },
    /// Full-text search across tasks, notes and files
    Search { query: Vec<String> },
    /// Recent activity, for one project or all
    Log {
        project: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// List projects with progress
    #[command(alias = "ls")]
    List {
        /// Include archived projects
        #[arg(long)]
        all: bool,
    },
    /// Create a project
    Add {
        name: String,
        #[arg(long, default_value = "")]
        desc: String,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Show a project's board (statuses and tasks)
    Show { project: String },
    /// Edit a project
    Edit {
        project: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        desc: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Archive a project (reversible)
    Archive { project: String },
    /// Restore an archived project
    Restore { project: String },
}

#[derive(Subcommand)]
enum TaskCmd {
    /// Add a task
    Add {
        project: String,
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        /// 0 = none, 1 = low, 2 = medium, 3 = high
        #[arg(long, short, default_value_t = 0)]
        priority: i64,
        /// YYYY-MM-DD
        #[arg(long)]
        due: Option<String>,
        /// Status name or id (default: first column)
        #[arg(long, short)]
        status: Option<String>,
    },
    /// Show one task
    Show { id: i64 },
    /// Edit a task
    Edit {
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long, short)]
        priority: Option<i64>,
        #[arg(long, conflicts_with = "no_due")]
        due: Option<String>,
        /// Remove the due date
        #[arg(long)]
        no_due: bool,
    },
    /// Move a task to another status (name or id)
    #[command(alias = "mv")]
    Move { id: i64, status: String },
    /// Move a task to the project's done column
    Done { id: i64 },
    /// Archive a task (reversible)
    Archive { id: i64 },
    /// Restore an archived task
    Restore { id: i64 },
}

fn default_actor() -> String {
    std::env::var("FJORD_ACTOR")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".into())
}

fn main() {
    if let Err(err) = run(Cli::parse()) {
        eprintln!("fjord: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let dir = cli.data_dir.clone().unwrap_or_else(Store::default_dir);
    let actor = cli.actor.clone().unwrap_or_else(default_actor);
    let mut store = Store::open(&dir, &actor).with_context(|| format!("could not open {}", dir.display()))?;
    let out = output::Printer { json: cli.json };

    match cli.command {
        Command::Project(cmd) => project(&mut store, &out, cmd),
        Command::Task(cmd) => task(&mut store, &out, cmd),
        Command::Attach { project, path, task } => {
            let p = store.find_project(&project)?;
            out.attachment(&store.attach_file(p.id, task, &path)?)
        }
        Command::Files { project, task } => {
            let p = store.find_project(&project)?;
            out.attachments(&store.list_attachments(p.id, task)?)
        }
        Command::Note { project, title, body } => {
            let p = store.find_project(&project)?;
            out.note(&store.add_note(p.id, &title, &body)?)
        }
        Command::Search { query } => out.search(&store.search(&query.join(" "))?),
        Command::Log { project, limit } => {
            let project_id = project.map(|p| store.find_project(&p)).transpose()?.map(|p| p.id);
            out.activity(&store.recent_activity(project_id, limit)?)
        }
    }
}

fn project(store: &mut Store, out: &output::Printer, cmd: ProjectCmd) -> Result<()> {
    match cmd {
        ProjectCmd::List { all } => out.projects(&store.list_projects(all)?),
        ProjectCmd::Add { name, desc, color, icon } => {
            out.project(&store.create_project(NewProject { name, description: desc, color, icon })?)
        }
        ProjectCmd::Show { project } => {
            let p = store.find_project(&project)?;
            out.board(&p, &store.list_statuses(p.id)?, &store.list_tasks(p.id)?)
        }
        ProjectCmd::Edit { project, name, desc, color, icon } => {
            let p = store.find_project(&project)?;
            let patch = ProjectPatch { name, description: desc, color, icon };
            out.project(&store.update_project(p.id, patch)?)
        }
        ProjectCmd::Archive { project } => {
            let p = store.find_project(&project)?;
            out.project(&store.set_project_archived(p.id, true)?)
        }
        ProjectCmd::Restore { project } => {
            let p = store.find_project(&project)?;
            out.project(&store.set_project_archived(p.id, false)?)
        }
    }
}

fn task(store: &mut Store, out: &output::Printer, cmd: TaskCmd) -> Result<()> {
    match cmd {
        TaskCmd::Add { project, title, body, priority, due, status } => {
            let p = store.find_project(&project)?;
            let status_id = status.map(|s| store.find_status(p.id, &s)).transpose()?.map(|s| s.id);
            let new = NewTask { project_id: p.id, title, body_md: body, priority, due_at: due, status_id };
            out.task(&store.create_task(new)?)
        }
        TaskCmd::Show { id } => out.task(&store.get_task(id)?),
        TaskCmd::Edit { id, title, body, priority, due, no_due } => {
            let due_at = if no_due { Some(None) } else { due.map(Some) };
            out.task(&store.update_task(id, TaskPatch { title, body_md: body, priority, due_at })?)
        }
        TaskCmd::Move { id, status } => {
            let t = store.get_task(id)?;
            let s = store.find_status(t.project_id, &status)?;
            out.task(&store.move_task(id, s.id, None)?)
        }
        TaskCmd::Done { id } => {
            let t = store.get_task(id)?;
            let done = store
                .list_statuses(t.project_id)?
                .into_iter()
                .find(|s| s.is_done)
                .context("project has no done column")?;
            out.task(&store.move_task(id, done.id, None)?)
        }
        TaskCmd::Archive { id } => out.task(&store.set_task_archived(id, true)?),
        TaskCmd::Restore { id } => out.task(&store.set_task_archived(id, false)?),
    }
}
