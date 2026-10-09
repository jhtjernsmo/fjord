//! Tauri shell: exposes fjord-core to the React UI as commands.

mod commands;
mod git_commands;
mod window_fit;

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let mut store = Store::open(&Store::default_dir(), &default_actor())?;
            // A name chosen in Settings wins over the OS user name ($FJORD_ACTOR still wins over both).
            if std::env::var_os("FJORD_ACTOR").is_none()
                && let Some(name) = store.user_name()?
            {
                store.set_actor(&name)?;
            }
            app.manage(AppState(Arc::new(Mutex::new(store))));
            window_fit::fit_main_window(app);
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
            commands::list_subtasks,
            commands::move_subtask,
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
            commands::list_all_notes,
            commands::create_note,
            commands::get_note,
            commands::move_note,
            commands::set_note_folder,
            commands::set_note_pinned,
            commands::resolve_links,
            commands::backlinks,
            commands::link_suggestions,
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
            git_commands::github_repo_owners,
            git_commands::create_github_repo,
            git_commands::publish_to_github,
            git_commands::set_repo_auto_move,
            git_commands::git_overview,
            git_commands::start_branch,
            git_commands::sync_pull_requests,
            git_commands::open_pull_request,
            git_commands::open_url,
            git_commands::github_account,
            git_commands::connect_github,
            git_commands::disconnect_github,
            git_commands::azure_account,
            git_commands::connect_azure,
            git_commands::disconnect_azure,
            git_commands::get_import_settings,
            git_commands::set_import_settings,
            git_commands::run_azure_import,
            git_commands::azure_mentions,
            git_commands::add_work_item_task,
            git_commands::azure_discussion,
            git_commands::task_external_link,
            git_commands::list_branches,
            git_commands::suggest_branch,
            git_commands::open_in_editor,
            git_commands::checkout_default,
            git_commands::link_task_branch,
            git_commands::start_github_login,
            git_commands::poll_github_login,
            git_commands::connect_azure_cli,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fjord");
}
