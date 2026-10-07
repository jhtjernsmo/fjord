//! Minimal MCP server (JSON-RPC 2.0 over stdio, one message per line) so AI
//! agents can work in Fjord. Agents can create, edit, move and archive, but
//! never hard-delete — everything they do is logged and reversible.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use fjord_core::{NewProject, NewTask, Store, TaskPatch};
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
            "Add a task to a project.",
            json!({ "project": project, "title": { "type": "string" }, "body": { "type": "string", "description": "Markdown" },
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
            "Add a markdown note to a project.",
            json!({ "project": project, "title": { "type": "string" }, "body": { "type": "string" } }),
            &["project", "title"],
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
            let p = store.find_project(str_arg(args, "project")?)?;
            serde_json::to_value(store.add_note(
                p.id,
                str_arg(args, "title")?,
                &opt_str(args, "body").unwrap_or_default(),
            )?)?
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
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 12);
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
}
