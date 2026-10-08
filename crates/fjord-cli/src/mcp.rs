//! Minimal MCP server (JSON-RPC 2.0 over stdio, one message per line) so AI
//! agents can work in Fjord. Agents can create, edit, move and archive, but
//! never hard-delete — everything they do is logged and reversible.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use fjord_core::{NewProject, NewTask, Note, Store, TaskPatch, parse_links};
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2025-06-18";
const DEFAULT_ACTIVITY_LIMIT: i64 = 20;

/// Serves requests until stdin closes.
pub fn serve(store: &mut Store) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(store, &line) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// Handles one JSON-RPC message; returns the reply (None for notifications).
pub fn handle_line(store: &mut Store, line: &str) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(error_reply(
                Value::Null,
                -32700,
                &format!("parse error: {e}"),
            ));
        }
    };
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let id = id?; // notifications (no id) get no reply
    let result = match method {
        "initialize" => initialize(&params),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tool_definitions() }),
        "tools/call" => call_tool(store, &params),
        _ => {
            return Some(error_reply(
                id,
                -32601,
                &format!("method not found: {method}"),
            ));
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error_reply(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "fjord", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Fjord is the user's local project manager. Projects are referenced by id or slug. \
            Prefer get_board before changing tasks. Nothing can be hard-deleted; archive instead."
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required }
    })
}

fn tool_definitions() -> Vec<Value> {
    let project = json!({ "type": "string", "description": "Project id or slug" });
    vec![
        tool(
            "list_projects",
            "List projects with progress counts.",
            json!({ "include_archived": { "type": "boolean" } }),
            &[],
        ),
        tool(
            "get_board",
            "Columns and active tasks of a project.",
            json!({ "project": project }),
            &["project"],
        ),
        tool(
            "create_project",
            "Create a project with default columns.",
            json!({ "name": { "type": "string" }, "description": { "type": "string" },
                    "icon": { "type": "string", "description": "Short glyph, e.g. >_" },
                    "color": { "type": "string", "description": "#rrggbb" } }),
            &["name"],
        ),
        tool(
            "create_task",
            "Add a task to a project. Pass parent (a task id in the same project) to create a subtask; subtasks can't have subtasks.",
            json!({ "project": project, "title": { "type": "string" }, "body": { "type": "string", "description": "Markdown" },
                    "parent": { "type": "integer", "description": "Parent task id, to create a subtask" },
                    "priority": { "type": "integer", "minimum": 0, "maximum": 3 },
                    "due": { "type": "string", "description": "YYYY-MM-DD" },
                    "status": { "type": "string", "description": "Column name or id (default: first)" } }),
            &["project", "title"],
        ),
        tool(
            "update_task",
            "Edit a task. Pass due: null to clear the due date.",
            json!({ "id": { "type": "integer" }, "title": { "type": "string" }, "body": { "type": "string" },
                    "priority": { "type": "integer", "minimum": 0, "maximum": 3 },
                    "due": { "type": ["string", "null"] } }),
            &["id"],
        ),
        tool(
            "move_task",
            "Move a task to another column.",
            json!({ "id": { "type": "integer" }, "status": { "type": "string" } }),
            &["id", "status"],
        ),
        tool(
            "complete_task",
            "Move a task to the project's done column.",
            json!({ "id": { "type": "integer" } }),
            &["id"],
        ),
        tool(
            "archive_task",
            "Archive a task (reversible).",
            json!({ "id": { "type": "integer" } }),
            &["id"],
        ),
        tool(
            "add_note",
            "Add a markdown note — to a project, or to the global notespace if project is omitted. Link with [[Project]], [[#12]] (task) or [[Note title]].",
            json!({ "project": project, "title": { "type": "string" }, "body": { "type": "string" },
                    "folder": { "type": "string", "description": "e.g. product/ideas" }, "pinned": { "type": "boolean" } }),
            &["title"],
        ),
        tool(
            "list_notes",
            "List notes (without bodies; use get_note). project: only that project's notes; free_only: only notespace notes without a project; otherwise all.",
            json!({ "project": project, "free_only": { "type": "boolean" } }),
            &[],
        ),
        tool(
            "get_note",
            "Read a note: markdown body plus its [[links]] resolved to projects/tasks/notes.",
            json!({ "id": { "type": "integer" } }),
            &["id"],
        ),
        tool(
            "update_note",
            "Edit a note. Only given fields change. append adds text to the end of the body. project: id/slug to move into a project, null to move to the notespace.",
            json!({ "id": { "type": "integer" }, "title": { "type": "string" }, "body": { "type": "string", "description": "Replaces the whole body" },
                    "append": { "type": "string" }, "folder": { "type": "string", "description": "e.g. product/ideas; empty = none" },
                    "pinned": { "type": "boolean" }, "project": { "type": ["string", "null"] } }),
            &["id"],
        ),
        tool(
            "list_new_imports",
            "Tasks imported from Azure Boards that no agent has analyzed yet, with their task and link. Analyze each one (update_task to append a short summary, acceptance criteria and suggested subtasks to the body; create_task for subtasks if useful), then call mark_analyzed.",
            json!({}),
            &[],
        ),
        tool(
            "mark_analyzed",
            "Mark an imported task as analyzed so it no longer shows up in list_new_imports.",
            json!({ "task": { "type": "integer" } }),
            &["task"],
        ),
        tool(
            "import_azure",
            "Fetch work items assigned to the user in Azure Boards and create or refresh tasks in the mapped Fjord projects (mappings are set in Fjord's Settings).",
            json!({}),
            &[],
        ),
        tool(
            "backlinks",
            "Notes that link to a project, task or note.",
            json!({ "kind": { "type": "string", "enum": ["project", "task", "note"] }, "id": { "type": "integer" } }),
            &["kind", "id"],
        ),
        tool(
            "attach_file",
            "Copy a local file into Fjord and attach it to a project or task.",
            json!({ "project": project, "path": { "type": "string" }, "task": { "type": "integer" } }),
            &["project", "path"],
        ),
        tool(
            "search",
            "Full-text search across tasks, notes and file names.",
            json!({ "query": { "type": "string" } }),
            &["query"],
        ),
        tool(
            "link_repo",
            "Link a project to the git repository at a local path (GitHub or Azure DevOps is detected from origin).",
            json!({ "project": project, "path": { "type": "string" } }),
            &["project", "path"],
        ),
        tool(
            "git_status",
            "Current branch, branches and recent commits of a project's repository.",
            json!({ "project": project }),
            &["project"],
        ),
        tool(
            "start_branch",
            "Check out (or create) the git branch for a task, named <type>/<id>-<title>, e.g. feat/12-add-login. type is feat, fix, chore, docs, refactor, test, perf, ci or hotfix (guessed from the task if omitted). Moves the task to in progress when auto-move is on.",
            json!({ "id": { "type": "integer" }, "type": { "type": "string", "enum": ["feat", "fix", "chore", "docs", "refactor", "test", "perf", "ci", "hotfix"] } }),
            &["id"],
        ),
        tool(
            "link_branch",
            "Link a task to a branch that already exists in the linked repository (or unlink with branch: null), so its pull requests show on the task. Doesn't check anything out.",
            json!({ "id": { "type": "integer" }, "branch": { "type": ["string", "null"] } }),
            &["id", "branch"],
        ),
        tool(
            "list_pull_requests",
            "GitHub pull requests of a project with CI state and linked tasks; moves tasks of merged PRs to done.",
            json!({ "project": project }),
            &["project"],
        ),
        tool(
            "open_pull_request",
            "Push a task's branch and open a GitHub pull request using the task title and description.",
            json!({ "id": { "type": "integer" }, "draft": { "type": "boolean" } }),
            &["id"],
        ),
        tool(
            "recent_activity",
            "Recent changes, optionally for one project.",
            json!({ "project": project, "limit": { "type": "integer" } }),
            &[],
        ),
    ]
}

fn call_tool(store: &mut Store, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match run_tool(store, name, &args) {
        Ok(value) => json!({
            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }],
            "structuredContent": { "result": value },
        }),
        Err(err) => {
            json!({ "content": [{ "type": "text", "text": format!("Error: {err:#}") }], "isError": true })
        }
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing string argument «{key}»"))
}

fn int_arg(args: &Value, key: &str) -> Result<i64> {
    args.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("missing integer argument «{key}»"))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn note_summary(n: &Note) -> Value {
    json!({ "id": n.id, "title": n.title, "project_id": n.project_id, "folder": n.folder,
            "pinned": n.pinned, "updated_at": n.updated_at })
}

fn update_note(store: &mut Store, args: &Value) -> Result<Note> {
    let id = int_arg(args, "id")?;
    let mut note = store.get_note(id)?;
    if args.get("title").is_some() || args.get("body").is_some() || args.get("append").is_some() {
        let title = opt_str(args, "title").unwrap_or(note.title);
        let mut body = opt_str(args, "body").unwrap_or(note.body_md);
        if let Some(extra) = opt_str(args, "append") {
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&extra);
        }
        note = store.update_note(id, &title, &body)?;
    }
    if let Some(folder) = opt_str(args, "folder") {
        note = store.set_note_folder(id, &folder)?;
    }
    if let Some(pinned) = args.get("pinned").and_then(Value::as_bool) {
        note = store.set_note_pinned(id, pinned)?;
    }
    match args.get("project") {
        None => {}
        Some(Value::Null) => note = store.move_note(id, None)?,
        Some(v) => {
            let p = v.as_str().context("project must be a string or null")?;
            let pid = store.find_project(p)?.id;
            note = store.move_note(id, Some(pid))?;
        }
    }
    Ok(note)
}

fn run_tool(store: &mut Store, name: &str, args: &Value) -> Result<Value> {
    let value = match name {
        "list_projects" => {
            let all = args
                .get("include_archived")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            serde_json::to_value(store.list_projects(all)?)?
        }
        "get_board" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            json!({ "project": p, "statuses": store.list_statuses(p.id)?, "tasks": store.list_tasks(p.id)? })
        }
        "create_project" => serde_json::to_value(store.create_project(NewProject {
            name: str_arg(args, "name")?.to_string(),
            description: opt_str(args, "description").unwrap_or_default(),
            icon: opt_str(args, "icon"),
            color: opt_str(args, "color"),
            locale: None,
        })?)?,
        "create_task" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            let status_id = match opt_str(args, "status") {
                Some(s) => Some(store.find_status(p.id, &s)?.id),
                None => None,
            };
            serde_json::to_value(store.create_task(NewTask {
                project_id: p.id,
                title: str_arg(args, "title")?.to_string(),
                body_md: opt_str(args, "body").unwrap_or_default(),
                priority: args.get("priority").and_then(Value::as_i64).unwrap_or(0),
                due_at: opt_str(args, "due"),
                status_id,
                parent_id: args.get("parent").and_then(Value::as_i64),
            })?)?
        }
        "update_task" => {
            let due_at = match args.get("due") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(v) => Some(Some(
                    v.as_str()
                        .context("due must be a string or null")?
                        .to_string(),
                )),
            };
            let patch = TaskPatch {
                title: opt_str(args, "title"),
                body_md: opt_str(args, "body"),
                priority: args.get("priority").and_then(Value::as_i64),
                due_at,
            };
            serde_json::to_value(store.update_task(int_arg(args, "id")?, patch)?)?
        }
        "move_task" => {
            let task = store.get_task(int_arg(args, "id")?)?;
            let status = store.find_status(task.project_id, str_arg(args, "status")?)?;
            serde_json::to_value(store.move_task(task.id, status.id, None)?)?
        }
        "complete_task" => {
            let task = store.get_task(int_arg(args, "id")?)?;
            let done = store
                .list_statuses(task.project_id)?
                .into_iter()
                .find(|s| s.is_done)
                .context("project has no done column")?;
            serde_json::to_value(store.move_task(task.id, done.id, None)?)?
        }
        "archive_task" => {
            serde_json::to_value(store.set_task_archived(int_arg(args, "id")?, true)?)?
        }
        "add_note" => {
            let project_id = match opt_str(args, "project") {
                Some(p) => Some(store.find_project(&p)?.id),
                None => None,
            };
            let mut note = store.create_note(
                project_id,
                str_arg(args, "title")?,
                &opt_str(args, "body").unwrap_or_default(),
            )?;
            if let Some(folder) = opt_str(args, "folder") {
                note = store.set_note_folder(note.id, &folder)?;
            }
            if let Some(true) = args.get("pinned").and_then(Value::as_bool) {
                note = store.set_note_pinned(note.id, true)?;
            }
            serde_json::to_value(note)?
        }
        "list_notes" => {
            let notes = match opt_str(args, "project") {
                Some(p) => store.list_notes(store.find_project(&p)?.id)?,
                None => store.list_all_notes(
                    args.get("free_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?,
            };
            Value::Array(notes.iter().map(note_summary).collect())
        }
        "get_note" => {
            let note = store.get_note(int_arg(args, "id")?)?;
            let links = parse_links(&note.body_md)
                .into_iter()
                .map(|text| Ok(json!({ "text": text, "target": store.resolve_link(&text)? })))
                .collect::<Result<Vec<_>>>()?;
            json!({ "note": note, "links": links })
        }
        "update_note" => serde_json::to_value(update_note(store, args)?)?,
        "list_new_imports" => serde_json::to_value(store.unanalyzed_imports()?)?,
        "mark_analyzed" => {
            serde_json::to_value(store.mark_import_analyzed(int_arg(args, "task")?)?)?
        }
        "import_azure" => {
            let settings = store.import_settings()?;
            if settings.mappings.is_empty() {
                bail!("no Azure Boards mappings yet: add them in Fjord's Settings → Azure DevOps");
            }
            let known: Vec<String> = store
                .external_ids("azure")?
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            let fetch = fjord_vcs::fetch_import(&settings, &known)?;
            serde_json::to_value(fjord_vcs::apply_import(store, &settings, &fetch)?)?
        }
        "backlinks" => {
            serde_json::to_value(store.backlinks(str_arg(args, "kind")?, int_arg(args, "id")?)?)?
        }
        "attach_file" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            let task = args.get("task").and_then(Value::as_i64);
            serde_json::to_value(store.attach_file(
                p.id,
                task,
                &PathBuf::from(str_arg(args, "path")?),
            )?)?
        }
        "search" => serde_json::to_value(store.search(str_arg(args, "query")?)?)?,
        "recent_activity" => {
            let project_id = match opt_str(args, "project") {
                Some(p) => Some(store.find_project(&p)?.id),
                None => None,
            };
            let limit = args
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(DEFAULT_ACTIVITY_LIMIT);
            serde_json::to_value(store.recent_activity(project_id, limit)?)?
        }
        "link_repo" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            serde_json::to_value(fjord_vcs::link_repo(
                store,
                p.id,
                &PathBuf::from(str_arg(args, "path")?),
            )?)?
        }
        "git_status" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            serde_json::to_value(fjord_vcs::overview(store, p.id)?)?
        }
        "start_branch" => serde_json::to_value(fjord_vcs::start_branch(
            store,
            int_arg(args, "id")?,
            args.get("type").and_then(Value::as_str),
        )?)?,
        "link_branch" => serde_json::to_value(fjord_vcs::link_branch(
            store,
            int_arg(args, "id")?,
            args.get("branch").and_then(Value::as_str),
        )?)?,
        "list_pull_requests" => {
            let p = store.find_project(str_arg(args, "project")?)?;
            serde_json::to_value(fjord_vcs::sync(store, p.id)?)?
        }
        "open_pull_request" => {
            let draft = args.get("draft").and_then(Value::as_bool).unwrap_or(false);
            serde_json::to_value(fjord_vcs::open_pull_request(
                store,
                int_arg(args, "id")?,
                draft,
            )?)?
        }
        other => bail!("unknown tool «{other}»"),
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (Store::open(dir.path(), "claude").unwrap(), dir)
    }

    fn call(store: &mut Store, id: i64, name: &str, args: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": args } });
        handle_line(store, &line.to_string()).unwrap()["result"].clone()
    }

    #[test]
    fn handshake_lists_tools_and_ignores_notifications() {
        let (mut s, _d) = store();
        let init = handle_line(&mut s, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert!(
            handle_line(
                &mut s,
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
            )
            .is_none()
        );
        let tools =
            handle_line(&mut s, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 25);
        let unknown = handle_line(&mut s, r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#).unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
        let bad = handle_line(&mut s, "{not json").unwrap();
        assert_eq!(bad["error"]["code"], -32700);
    }

    #[test]
    fn agent_workflow_create_move_complete_search() {
        let (mut s, _d) = store();
        call(&mut s, 1, "create_project", json!({ "name": "Bokost" }));
        let task = call(
            &mut s,
            2,
            "create_task",
            json!({ "project": "bokost", "title": "Fix push", "priority": 3 }),
        );
        let id = task["structuredContent"]["result"]["id"].as_i64().unwrap();
        call(
            &mut s,
            3,
            "move_task",
            json!({ "id": id, "status": "in progress" }),
        );
        let done = call(&mut s, 4, "complete_task", json!({ "id": id }));
        assert_eq!(done["isError"], Value::Null);
        let hits = call(&mut s, 5, "search", json!({ "query": "push" }));
        assert_eq!(hits["structuredContent"]["result"][0]["ref_id"], id);
        let log = call(&mut s, 6, "recent_activity", json!({ "project": "bokost" }));
        assert!(
            log["structuredContent"]["result"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["actor"] == "claude")
        );
    }

    #[test]
    fn tool_errors_are_reported_not_fatal() {
        let (mut s, _d) = store();
        let missing = call(&mut s, 1, "get_board", json!({ "project": "nope" }));
        assert_eq!(missing["isError"], true);
        let bad_args = call(&mut s, 2, "create_task", json!({ "project": 1 }));
        assert_eq!(bad_args["isError"], true);
        let unknown = call(&mut s, 3, "delete_everything", json!({}));
        assert_eq!(unknown["isError"], true);
    }

    #[test]
    fn update_task_can_clear_due_date() {
        let (mut s, _d) = store();
        call(&mut s, 1, "create_project", json!({ "name": "P" }));
        let t = call(
            &mut s,
            2,
            "create_task",
            json!({ "project": "p", "title": "T", "due": "2026-12-01" }),
        );
        let id = t["structuredContent"]["result"]["id"].as_i64().unwrap();
        let cleared = call(&mut s, 3, "update_task", json!({ "id": id, "due": null }));
        assert_eq!(
            cleared["structuredContent"]["result"]["due_at"],
            Value::Null
        );
    }

    #[test]
    fn agent_reads_and_edits_notespace() {
        let (mut s, _d) = store();
        call(&mut s, 1, "create_project", json!({ "name": "Bokost" }));
        let t = call(
            &mut s,
            2,
            "create_task",
            json!({ "project": "bokost", "title": "Fix push" }),
        );
        let task_id = t["structuredContent"]["result"]["id"].as_i64().unwrap();
        let n = call(
            &mut s,
            3,
            "add_note",
            json!({ "title": "Plan", "body": "Work on [[Bokost]]", "folder": "inbox" }),
        );
        assert_eq!(n["structuredContent"]["result"]["folder"], "inbox");
        assert_eq!(n["structuredContent"]["result"]["project_id"], Value::Null);
        let id = n["structuredContent"]["result"]["id"].as_i64().unwrap();

        let free = call(&mut s, 4, "list_notes", json!({ "free_only": true }));
        assert_eq!(free["structuredContent"]["result"][0]["title"], "Plan");
        assert!(
            free["structuredContent"]["result"][0]
                .get("body_md")
                .is_none()
        );

        let edited = call(
            &mut s,
            5,
            "update_note",
            json!({ "id": id, "append": format!("- [[#{task_id}]]"), "folder": "work", "pinned": true }),
        );
        let note = &edited["structuredContent"]["result"];
        assert_eq!(
            note["body_md"],
            format!("Work on [[Bokost]]\n- [[#{task_id}]]")
        );
        assert_eq!(
            (note["folder"].as_str(), note["pinned"].as_bool()),
            (Some("work"), Some(true))
        );

        let read = call(&mut s, 6, "get_note", json!({ "id": id }));
        let links = &read["structuredContent"]["result"]["links"];
        assert_eq!(links[0]["target"]["kind"], "project");
        assert_eq!(links[1]["target"]["id"], task_id);
        let back = call(
            &mut s,
            7,
            "backlinks",
            json!({ "kind": "task", "id": task_id }),
        );
        assert_eq!(back["structuredContent"]["result"][0]["id"], id);

        call(
            &mut s,
            8,
            "update_note",
            json!({ "id": id, "project": "bokost" }),
        );
        let in_project = call(&mut s, 9, "list_notes", json!({ "project": "bokost" }));
        assert_eq!(in_project["structuredContent"]["result"][0]["id"], id);
        let moved_back = call(
            &mut s,
            10,
            "update_note",
            json!({ "id": id, "project": null }),
        );
        assert_eq!(
            moved_back["structuredContent"]["result"]["project_id"],
            Value::Null
        );
    }

    #[test]
    fn agent_triages_imported_tasks() {
        let (mut s, _d) = store();
        call(&mut s, 1, "create_project", json!({ "name": "Bokost" }));
        let p = s.find_project("bokost").unwrap().id;
        let item = fjord_core::ExternalItem {
            source: "azure".into(),
            external_id: "contoso/7".into(),
            url: "https://dev.azure.com/contoso/App/_workitems/edit/7".into(),
            rev: 1,
            title: "Crash on login".into(),
            body_md: "App crashes".into(),
            priority: 3,
            due_at: None,
        };
        let (task, _) = s.upsert_imported_task(p, &item).unwrap();

        let pending = call(&mut s, 2, "list_new_imports", json!({}));
        assert_eq!(
            pending["structuredContent"]["result"][0]["task"]["id"],
            task.id
        );
        assert_eq!(
            pending["structuredContent"]["result"][0]["link"]["external_id"],
            "contoso/7"
        );

        call(
            &mut s,
            3,
            "update_task",
            json!({ "id": task.id, "body": "App crashes\n\n## Analysis\n- Likely token refresh" }),
        );
        let marked = call(&mut s, 4, "mark_analyzed", json!({ "task": task.id }));
        assert!(marked["structuredContent"]["result"]["analyzed_at"].is_string());
        let empty = call(&mut s, 5, "list_new_imports", json!({}));
        assert_eq!(empty["structuredContent"]["result"], json!([]));

        let no_mappings = call(&mut s, 6, "import_azure", json!({}));
        assert_eq!(no_mappings["isError"], true);
    }
}
