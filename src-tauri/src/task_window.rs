//! Task details in a window of their own ("pop out"), to keep beside the board or
//! full screen on another monitor. Links in it open in the main window.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

const MAIN: &str = "main";
const LABEL_PREFIX: &str = "task-";
/// The main window listens for this to open what a task window links to.
const NAV_EVENT: &str = "fjord-nav";
const SIZE: (f64, f64) = (760.0, 900.0);
const MIN_SIZE: (f64, f64) = (480.0, 400.0);

/// Something a task window asks the main window to show.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MainNav {
    Note { id: i64, project_id: Option<i64> },
    Task { project_id: i64, task_id: i64 },
    Project { id: i64 },
}

/// Opens the task in its own window, or brings forward the one it already has.
#[tauri::command]
pub fn open_task_window(app: AppHandle, task_id: i64, title: String) -> Result<(), String> {
    let label = format!("{LABEL_PREFIX}{task_id}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        return window.set_focus().map_err(|e| e.to_string());
    }
    let url = WebviewUrl::App(format!("index.html?task={task_id}").into());
    WebviewWindowBuilder::new(&app, &label, url)
        .title(format!("#{task_id} {title} · Fjord"))
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Shows `nav` in the main window and brings it forward.
#[tauri::command]
pub fn navigate_main(app: AppHandle, nav: MainNav) -> Result<(), String> {
    let main = app
        .get_webview_window(MAIN)
        .ok_or_else(|| "the main window is closed".to_string())?;
    let _ = main.unminimize();
    let _ = main.set_focus();
    app.emit_to(MAIN, NAV_EVENT, nav).map_err(|e| e.to_string())
}

/// Task windows close with the main window, so the app doesn't linger.
pub fn close_task_windows<R: Runtime>(app: &AppHandle<R>) {
    for (label, window) in app.webview_windows() {
        if label.starts_with(LABEL_PREFIX) {
            let _ = window.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_matches_what_the_ui_sends() {
        let nav: MainNav =
            serde_json::from_str(r#"{"kind":"note","id":3,"project_id":null}"#).unwrap();
        assert!(matches!(
            nav,
            MainNav::Note {
                id: 3,
                project_id: None
            }
        ));
        let nav: MainNav =
            serde_json::from_str(r#"{"kind":"task","project_id":1,"task_id":7}"#).unwrap();
        assert!(matches!(
            nav,
            MainNav::Task {
                project_id: 1,
                task_id: 7
            }
        ));
    }
}
