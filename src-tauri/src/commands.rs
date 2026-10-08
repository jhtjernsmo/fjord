//! Thin command layer: lock the store, call fjord-core, map errors to strings.

use std::path::PathBuf;

use base64::Engine;
use fjord_core::{
    Activity, Attachment, LinkTarget, NewProject, NewTask, Note, Project, ProjectPatch,
    ProjectSummary, SearchHit, Status, StatusPatch, Store, Task, TaskPatch,
};
use serde::Serialize;
use tauri::State;

use crate::AppState;

const MAX_PREVIEW_BYTES: u64 = 8 * 1024 * 1024;
const IMAGE_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
];

type CmdResult<T> = Result<T, String>;

fn with_store<T>(
    state: &State<AppState>,
    f: impl FnOnce(&mut Store) -> fjord_core::Result<T>,
) -> CmdResult<T> {
    let mut store = state
        .0
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    f(&mut store).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct Board {
    project: Project,
    statuses: Vec<Status>,
    tasks: Vec<Task>,
}

#[tauri::command]
pub fn actor(state: State<AppState>) -> CmdResult<String> {
    with_store(&state, |s| Ok(s.actor().to_string()))
}

#[tauri::command]
pub fn change_counter(state: State<AppState>) -> CmdResult<i64> {
    with_store(&state, |s| s.change_counter())
}

#[tauri::command]
pub fn list_projects(
    state: State<AppState>,
    include_archived: bool,
) -> CmdResult<Vec<ProjectSummary>> {
    with_store(&state, |s| s.list_projects(include_archived))
}

#[tauri::command]
pub fn create_project(state: State<AppState>, input: NewProject) -> CmdResult<Project> {
    with_store(&state, |s| s.create_project(input))
}

#[tauri::command]
pub fn update_project(state: State<AppState>, id: i64, patch: ProjectPatch) -> CmdResult<Project> {
    with_store(&state, |s| s.update_project(id, patch))
}

#[tauri::command]
pub fn archive_project(state: State<AppState>, id: i64, archived: bool) -> CmdResult<Project> {
    with_store(&state, |s| s.set_project_archived(id, archived))
}

#[tauri::command]
pub fn get_board(state: State<AppState>, project_id: i64) -> CmdResult<Board> {
    with_store(&state, |s| {
        Ok(Board {
            project: s.get_project(project_id)?,
            statuses: s.list_statuses(project_id)?,
            tasks: s.list_tasks(project_id)?,
        })
    })
}

#[tauri::command]
pub fn create_task(state: State<AppState>, input: NewTask) -> CmdResult<Task> {
    with_store(&state, |s| s.create_task(input))
}

#[tauri::command]
pub fn update_task(state: State<AppState>, id: i64, patch: TaskPatch) -> CmdResult<Task> {
    with_store(&state, |s| s.update_task(id, patch))
}

#[tauri::command]
pub fn move_task(
    state: State<AppState>,
    id: i64,
    status_id: i64,
    before_task_id: Option<i64>,
) -> CmdResult<Task> {
    with_store(&state, |s| s.move_task(id, status_id, before_task_id))
}

#[tauri::command]
pub fn archive_task(state: State<AppState>, id: i64, archived: bool) -> CmdResult<Task> {
    with_store(&state, |s| s.set_task_archived(id, archived))
}

#[tauri::command]
pub fn list_subtasks(state: State<AppState>, task_id: i64) -> CmdResult<Vec<Task>> {
    with_store(&state, |s| s.list_subtasks(task_id))
}

#[tauri::command]
pub fn move_subtask(state: State<AppState>, id: i64, index: usize) -> CmdResult<Vec<Task>> {
    with_store(&state, |s| s.move_subtask(id, index))
}

#[tauri::command]
pub fn list_archived_tasks(state: State<AppState>, project_id: i64) -> CmdResult<Vec<Task>> {
    with_store(&state, |s| s.list_archived_tasks(project_id))
}

#[tauri::command]
pub fn create_status(
    state: State<AppState>,
    project_id: i64,
    name: String,
    color: Option<String>,
    is_done: bool,
) -> CmdResult<Status> {
    with_store(&state, |s| {
        s.create_status(project_id, &name, color.as_deref(), is_done)
    })
}

#[tauri::command]
pub fn update_status(state: State<AppState>, id: i64, patch: StatusPatch) -> CmdResult<Status> {
    with_store(&state, |s| s.update_status(id, patch))
}

#[tauri::command]
pub fn move_status(state: State<AppState>, id: i64, index: usize) -> CmdResult<Vec<Status>> {
    with_store(&state, |s| s.move_status(id, index))
}

#[tauri::command]
pub fn delete_status(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.delete_status(id))
}

#[tauri::command]
pub fn delete_task(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.delete_task(id))
}

#[tauri::command]
pub fn delete_project(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.delete_project(id))
}

#[tauri::command]
pub fn delete_note(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.delete_note(id))
}

#[tauri::command]
pub fn rename_user(
    state: State<AppState>,
    name: String,
    rewrite_history: bool,
) -> CmdResult<String> {
    with_store(&state, |s| s.rename_user(&name, rewrite_history))
}

#[tauri::command]
pub fn list_attachments(
    state: State<AppState>,
    project_id: i64,
    task_id: Option<i64>,
) -> CmdResult<Vec<Attachment>> {
    with_store(&state, |s| s.list_attachments(project_id, task_id))
}

#[derive(Serialize)]
pub struct AttachOutcome {
    path: String,
    attachment: Option<Attachment>,
    error: Option<String>,
}

/// Attaches dropped files; directories and unreadable paths are reported per file, not fatal.
#[tauri::command]
pub fn attach_files(
    state: State<AppState>,
    project_id: i64,
    task_id: Option<i64>,
    paths: Vec<PathBuf>,
) -> CmdResult<Vec<AttachOutcome>> {
    with_store(&state, |s| {
        Ok(paths
            .iter()
            .map(|p| match s.attach_file(project_id, task_id, p) {
                Ok(a) => AttachOutcome {
                    path: p.display().to_string(),
                    attachment: Some(a),
                    error: None,
                },
                Err(e) => AttachOutcome {
                    path: p.display().to_string(),
                    attachment: None,
                    error: Some(e.to_string()),
                },
            })
            .collect())
    })
}

#[tauri::command]
pub fn detach_file(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.detach_file(id))
}

/// Blobs have no extension, so copy to a temp file with the original name
/// and let the desktop pick the right app.
#[tauri::command]
pub fn open_attachment(state: State<AppState>, id: i64) -> CmdResult<()> {
    let (blob, attachment) = with_store(&state, |s| {
        Ok((s.attachment_path(id)?, s.get_attachment(id)?))
    })?;
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("fjord-open");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe_name: String = attachment
        .original_name
        .chars()
        // Characters not allowed in file names on Windows (and '/' everywhere).
        .map(|c| {
            if r#"/\:*?"<>|"#.contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let target = dir.join(format!("{}-{safe_name}", attachment.id));
    std::fs::copy(&blob, &target).map_err(|e| e.to_string())?;
    tauri_plugin_opener::open_path(&target, None::<&str>).map_err(|e| e.to_string())
}

/// Data URL for small raster images, `None` for everything else.
#[tauri::command]
pub fn preview_attachment(state: State<AppState>, id: i64) -> CmdResult<Option<String>> {
    let (blob, attachment) = with_store(&state, |s| {
        Ok((s.attachment_path(id)?, s.get_attachment(id)?))
    })?;
    let ext = attachment
        .original_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    let Some((_, mime)) = IMAGE_TYPES.iter().find(|(e, _)| *e == ext) else {
        return Ok(None);
    };
    if attachment.size as u64 > MAX_PREVIEW_BYTES {
        return Ok(None);
    }
    let bytes = std::fs::read(blob).map_err(|e| e.to_string())?;
    Ok(Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}

#[tauri::command]
pub fn list_notes(state: State<AppState>, project_id: i64) -> CmdResult<Vec<Note>> {
    with_store(&state, |s| s.list_notes(project_id))
}

#[tauri::command]
pub fn add_note(
    state: State<AppState>,
    project_id: i64,
    title: String,
    body_md: String,
) -> CmdResult<Note> {
    with_store(&state, |s| s.add_note(project_id, &title, &body_md))
}

#[tauri::command]
pub fn update_note(
    state: State<AppState>,
    id: i64,
    title: String,
    body_md: String,
) -> CmdResult<Note> {
    with_store(&state, |s| s.update_note(id, &title, &body_md))
}

#[tauri::command]
pub fn list_all_notes(state: State<AppState>, free_only: bool) -> CmdResult<Vec<Note>> {
    with_store(&state, |s| s.list_all_notes(free_only))
}

#[tauri::command]
pub fn create_note(
    state: State<AppState>,
    project_id: Option<i64>,
    title: String,
    body_md: String,
) -> CmdResult<Note> {
    with_store(&state, |s| s.create_note(project_id, &title, &body_md))
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: i64) -> CmdResult<Note> {
    with_store(&state, |s| s.get_note(id))
}

#[tauri::command]
pub fn move_note(state: State<AppState>, id: i64, project_id: Option<i64>) -> CmdResult<Note> {
    with_store(&state, |s| s.move_note(id, project_id))
}

#[tauri::command]
pub fn set_note_folder(state: State<AppState>, id: i64, folder: String) -> CmdResult<Note> {
    with_store(&state, |s| s.set_note_folder(id, &folder))
}

#[tauri::command]
pub fn set_note_pinned(state: State<AppState>, id: i64, pinned: bool) -> CmdResult<Note> {
    with_store(&state, |s| s.set_note_pinned(id, pinned))
}

/// Resolves many `[[link]]` texts at once (for rendering a note).
#[tauri::command]
pub fn resolve_links(
    state: State<AppState>,
    texts: Vec<String>,
) -> CmdResult<Vec<Option<LinkTarget>>> {
    with_store(&state, |s| {
        texts.iter().map(|t| s.resolve_link(t)).collect()
    })
}

#[tauri::command]
pub fn backlinks(state: State<AppState>, kind: String, target_id: i64) -> CmdResult<Vec<Note>> {
    with_store(&state, |s| s.backlinks(&kind, target_id))
}

#[tauri::command]
pub fn link_suggestions(state: State<AppState>, query: String) -> CmdResult<Vec<LinkTarget>> {
    with_store(&state, |s| s.link_suggestions(&query))
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> CmdResult<Vec<SearchHit>> {
    with_store(&state, |s| s.search(&query))
}

#[tauri::command]
pub fn recent_activity(
    state: State<AppState>,
    project_id: Option<i64>,
    limit: i64,
) -> CmdResult<Vec<Activity>> {
    with_store(&state, |s| s.recent_activity(project_id, limit))
}

/// User overrides from <config dir>/keymap.json (raw JSON; merged in the UI).
#[tauri::command]
pub fn load_keymap() -> CmdResult<Option<serde_json::Value>> {
    let path = fjord_core::config_dir().join("keymap.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Serialize)]
pub struct DataPaths {
    data: String,
    keymap: String,
}

/// Where Fjord keeps its data and keymap on this machine (shown in Settings).
#[tauri::command]
pub fn data_paths() -> DataPaths {
    DataPaths {
        data: Store::default_dir().display().to_string(),
        keymap: fjord_core::config_dir()
            .join("keymap.json")
            .display()
            .to_string(),
    }
}
