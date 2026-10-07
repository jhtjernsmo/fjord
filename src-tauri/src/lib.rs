//! Tauri shell: exposes fjord-core to the React UI as commands.

mod commands;
mod git_commands;

use std::sync::{Arc, Mutex};

use fjord_core::{Store, default_actor};
use tauri::Manager;

/// Shared so slow work (git, network) can run on worker threads.
pub struct AppState(pub Arc<Mutex<Store>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let mut store = Store::open(&Store::default_dir(), &default_actor())?;
            // A name chosen in Settings wins over the OS user name ($FJORD_ACTOR still wins over both).
            if std::env::var_os("FJORD_ACTOR").is_none()
                && let Some(name) = store.user_name()?
            {
                store.set_actor(&name)?;
            }
            app.manage(AppState(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::actor,
            commands::change_counter,
            commands::list_projects,
            commands::create_project,
            commands::update_project,
            commands::archive_project,
            commands::get_board,
            commands::create_task,
            commands::update_task,
            commands::move_task,
            commands::archive_task,
            commands::list_archived_tasks,
            commands::create_status,
            commands::update_status,
            commands::move_status,
            commands::delete_status,
            commands::list_attachments,
            commands::attach_files,
            commands::detach_file,
            commands::open_attachment,
            commands::preview_attachment,
            commands::list_notes,
            commands::add_note,
            commands::update_note,
            commands::search,
            commands::recent_activity,
            commands::load_keymap,
            commands::data_paths,
            commands::delete_task,
            commands::delete_project,
            commands::delete_note,
            commands::rename_user,
            git_commands::get_project_repo,
            git_commands::link_repo,
            git_commands::unlink_repo,
            git_commands::set_repo_auto_move,
            git_commands::git_overview,
            git_commands::start_branch,
            git_commands::sync_pull_requests,
            git_commands::open_pull_request,
            git_commands::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fjord");
}
