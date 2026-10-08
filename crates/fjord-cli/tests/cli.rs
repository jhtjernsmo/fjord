//! End-to-end tests: run the real `fjord` binary against a temp data dir.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

fn fjord(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fjord"))
        .arg("--data-dir")
        .arg(dir)
        .args(args)
        .env("LANG", "en_US.UTF-8")
        .env("FJORD_ACTOR", "tester")
        .output()
        .expect("failed to run fjord")
}

fn ok_json(dir: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = fjord(dir, &all);
    assert!(
        out.status.success(),
        "fjord {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

#[test]
fn project_task_column_and_archive_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    let project = ok_json(d, &["project", "add", "Bokost", "--icon", "$"]);
    assert_eq!(project["slug"], "bokost");

    let task = ok_json(
        d,
        &[
            "task",
            "add",
            "bokost",
            "Fix push",
            "-p",
            "3",
            "--due",
            "2026-10-10",
        ],
    );
    let id = task["id"].as_i64().unwrap().to_string();
    assert_eq!(task["created_by"], "tester");

    ok_json(
        d,
        &["column", "add", "bokost", "Review", "--color", "#ff00aa"],
    );
    let moved = ok_json(d, &["task", "move", &id, "review"]);
    let columns = ok_json(d, &["column", "ls", "bokost"]);
    let review = columns
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Review")
        .unwrap();
    assert_eq!(moved["status_id"], review["id"]);

    ok_json(d, &["task", "done", &id]);
    let board = ok_json(d, &["project", "show", "bokost"]);
    let done_col = board["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["status"]["is_done"] == true)
        .unwrap();
    assert_eq!(done_col["tasks"][0]["title"], "Fix push");

    ok_json(d, &["task", "archive", &id]);
    assert_eq!(
        ok_json(d, &["task", "archived", "bokost"])
            .as_array()
            .unwrap()
            .len(),
        1
    );
    ok_json(d, &["task", "restore", &id]);
    assert!(
        ok_json(d, &["task", "archived", "bokost"])
            .as_array()
            .unwrap()
            .is_empty()
    );

    let hits = ok_json(d, &["search", "push"]);
    assert_eq!(hits[0]["kind"], "task");
}

#[test]
fn files_are_attached_and_listed() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    ok_json(d, &["project", "add", "P"]);
    let file = d.join("notes.txt");
    std::fs::write(&file, "hello").unwrap();
    let a = ok_json(d, &["attach", "p", file.to_str().unwrap()]);
    assert_eq!(
        (a["original_name"].as_str(), a["size"].as_i64()),
        (Some("notes.txt"), Some(5))
    );
    assert_eq!(ok_json(d, &["files", "p"]).as_array().unwrap().len(), 1);
}

#[test]
fn bad_input_fails_with_message_and_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    ok_json(d, &["project", "add", "P"]);
    let out = fjord(d, &["task", "add", "p", "x", "--due", "10.10.2026"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("YYYY-MM-DD"));
    let missing = fjord(d, &["project", "show", "nope"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("not found"));
}

#[test]
fn mcp_server_speaks_json_rpc_over_stdio() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_fjord"))
        .arg("--data-dir")
        .arg(dir.path())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn fjord mcp");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut request = |msg: Value| -> Option<Value> {
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
        msg.get("id")?;
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        Some(serde_json::from_str(&line).unwrap())
    };

    let init = request(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "fjord");
    assert!(request(json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());

    let call = |name: &str, args: Value| json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":args}});
    request(call("create_project", json!({"name": "Agent work"})));
    let task = request(call(
        "create_task",
        json!({"project": "agent-work", "title": "Write tests"}),
    ))
    .unwrap();
    assert_eq!(
        task["result"]["structuredContent"]["result"]["created_by"],
        "claude"
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn git_link_branch_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("data");
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "T"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(repo.join("a.txt"), "a").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "init"]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/jhtjernsmo/demo.git",
    ]);

    ok_json(&d, &["project", "add", "Demo"]);
    let task = ok_json(&d, &["task", "add", "demo", "Fix push"]);
    let id = task["id"].as_i64().unwrap().to_string();

    let linked = ok_json(&d, &["git", "link", "demo", repo.to_str().unwrap()]);
    assert_eq!(
        (
            linked["github_owner"].as_str(),
            linked["github_repo"].as_str()
        ),
        (Some("jhtjernsmo"), Some("demo"))
    );

    // "Fix push" reads like a bug, so the conventional type is guessed as fix.
    let started = ok_json(&d, &["git", "branch", &id]);
    assert_eq!(started["branch"], format!("fix/{id}-fix-push"));
    assert_eq!(started["created"], true);

    let status = ok_json(&d, &["git", "status", "demo"]);
    assert_eq!(status["current_branch"], format!("fix/{id}-fix-push"));
    assert_eq!(status["commits"][0]["subject"], "init");

    ok_json(&d, &["git", "auto-move", "demo", "off"]);
    ok_json(&d, &["git", "unlink", "demo"]);
    let unlinked = fjord(&d, &["git", "status", "demo"]);
    assert!(!unlinked.status.success());
    assert!(String::from_utf8_lossy(&unlinked.stderr).contains("not linked"));
}

#[test]
fn delete_requires_confirmation_or_yes() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    ok_json(d, &["project", "add", "P"]);
    let t = ok_json(d, &["task", "add", "p", "Gone soon"]);
    let id = t["id"].as_i64().unwrap().to_string();
    // stdin isn't a terminal in tests, so without --yes it must refuse.
    let refused = fjord(d, &["task", "delete", &id]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));
    ok_json(d, &["task", "delete", &id, "--yes"]);
    assert!(!fjord(d, &["task", "show", &id]).status.success());
    ok_json(d, &["project", "delete", "p", "--yes"]);
    assert!(
        ok_json(d, &["project", "ls", "--all"])
            .as_array()
            .unwrap()
            .is_empty()
    );
}
