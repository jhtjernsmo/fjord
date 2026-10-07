//! `fjord` — command-line access to Fjord, for humans, scripts and AI agents.

mod mcp;
mod output;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fjord_core::{NewProject, NewTask, ProjectPatch, StatusPatch, Store, TaskPatch, default_actor};

#[derive(Parser)]
#[command(
    name = "fjord",
    version,
    about = "Local-first project management — CLI"
)]
struct Cli {
    /// Who is making changes (shown in the activity log). Default: $FJORD_ACTOR or your user name.
    #[arg(long, global = true)]
    actor: Option<String>,
    /// Data directory. Default: $FJORD_DATA_DIR or the platform data dir (~/.local/share/fjord on Linux).
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
    /// Manage board columns
    #[command(subcommand, alias = "c")]
    Column(ColumnCmd),
    /// Run as an MCP server on stdio (for AI agents; actor defaults to "claude")
    Mcp,
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
    /// List archived tasks of a project
    Archived { project: String },
}

#[derive(Subcommand)]
enum ColumnCmd {
    /// List columns of a project
    #[command(alias = "ls")]
    List { project: String },
    /// Add a column at the end
    Add {
        project: String,
        name: String,
        #[arg(long)]
        color: Option<String>,
        /// Tasks in this column count as done
        #[arg(long)]
        done: bool,
    },
    /// Rename / recolor a column (name or id)
    Edit {
        project: String,
        column: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        done: Option<bool>,
    },
    /// Move a column to a 0-based position
    #[command(alias = "mv")]
    Move {
        project: String,
        column: String,
        position: usize,
    },
    /// Remove an empty column
    #[command(alias = "rm")]
    Remove { project: String, column: String },
}

fn main() {
    if let Err(err) = run(Cli::parse()) {
        eprintln!("fjord: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let dir = cli.data_dir.clone().unwrap_or_else(Store::default_dir);
    let is_mcp = matches!(cli.command, Command::Mcp);
    let actor = cli.actor.clone().unwrap_or_else(|| {
        if is_mcp {
            "claude".into()
        } else {
            default_actor()
        }
    });
    let mut store =
        Store::open(&dir, &actor).with_context(|| format!("could not open {}", dir.display()))?;
    let out = output::Printer { json: cli.json };

    match cli.command {
        Command::Project(cmd) => project(&mut store, &out, cmd),
        Command::Task(cmd) => task(&mut store, &out, cmd),
        Command::Column(cmd) => column(&mut store, &out, cmd),
        Command::Mcp => mcp::serve(&mut store),
        Command::Attach {
            project,
            path,
            task,
        } => {
            let p = store.find_project(&project)?;
            out.attachment(&store.attach_file(p.id, task, &path)?)
        }
        Command::Files { project, task } => {
            let p = store.find_project(&project)?;
            out.attachments(&store.list_attachments(p.id, task)?)
        }
        Command::Note {
            project,
            title,
            body,
        } => {
            let p = store.find_project(&project)?;
            out.note(&store.add_note(p.id, &title, &body)?)
        }
        Command::Search { query } => out.search(&store.search(&query.join(" "))?),
        Command::Log { project, limit } => {
            let project_id = project
                .map(|p| store.find_project(&p))
                .transpose()?
                .map(|p| p.id);
            out.activity(&store.recent_activity(project_id, limit)?)
        }
    }
}

fn project(store: &mut Store, out: &output::Printer, cmd: ProjectCmd) -> Result<()> {
    match cmd {
        ProjectCmd::List { all } => out.projects(&store.list_projects(all)?),
        ProjectCmd::Add {
            name,
            desc,
            color,
            icon,
        } => {
            let locale = std::env::var("LANG").ok();
            out.project(&store.create_project(NewProject {
                name,
                locale,
                description: desc,
                color,
                icon,
            })?)
        }
        ProjectCmd::Show { project } => {
            let p = store.find_project(&project)?;
            out.board(&p, &store.list_statuses(p.id)?, &store.list_tasks(p.id)?)
        }
        ProjectCmd::Edit {
            project,
            name,
            desc,
            color,
            icon,
        } => {
            let p = store.find_project(&project)?;
            let patch = ProjectPatch {
                name,
                description: desc,
                color,
                icon,
            };
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
        TaskCmd::Add {
            project,
            title,
            body,
            priority,
            due,
            status,
        } => {
            let p = store.find_project(&project)?;
            let status_id = status
                .map(|s| store.find_status(p.id, &s))
                .transpose()?
                .map(|s| s.id);
            let new = NewTask {
                project_id: p.id,
                title,
                body_md: body,
                priority,
                due_at: due,
                status_id,
            };
            out.task(&store.create_task(new)?)
        }
        TaskCmd::Show { id } => out.task(&store.get_task(id)?),
        TaskCmd::Edit {
            id,
            title,
            body,
            priority,
            due,
            no_due,
        } => {
            let due_at = if no_due { Some(None) } else { due.map(Some) };
            out.task(&store.update_task(
                id,
                TaskPatch {
                    title,
                    body_md: body,
                    priority,
                    due_at,
                },
            )?)
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
        TaskCmd::Archived { project } => {
            let p = store.find_project(&project)?;
            out.tasks(&store.list_archived_tasks(p.id)?)
        }
    }
}

fn column(store: &mut Store, out: &output::Printer, cmd: ColumnCmd) -> Result<()> {
    match cmd {
        ColumnCmd::List { project } => {
            let p = store.find_project(&project)?;
            out.statuses(&store.list_statuses(p.id)?)
        }
        ColumnCmd::Add {
            project,
            name,
            color,
            done,
        } => {
            let p = store.find_project(&project)?;
            out.statuses(&[store.create_status(p.id, &name, color.as_deref(), done)?])
        }
        ColumnCmd::Edit {
            project,
            column,
            name,
            color,
            done,
        } => {
            let p = store.find_project(&project)?;
            let s = store.find_status(p.id, &column)?;
            out.statuses(&[store.update_status(
                s.id,
                StatusPatch {
                    name,
                    color,
                    is_done: done,
                },
            )?])
        }
        ColumnCmd::Move {
            project,
            column,
            position,
        } => {
            let p = store.find_project(&project)?;
            let s = store.find_status(p.id, &column)?;
            out.statuses(&store.move_status(s.id, position)?)
        }
        ColumnCmd::Remove { project, column } => {
            let p = store.find_project(&project)?;
            let s = store.find_status(p.id, &column)?;
            store.delete_status(s.id)?;
            out.statuses(&store.list_statuses(p.id)?)
        }
    }
}
