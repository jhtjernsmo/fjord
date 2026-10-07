//! Tauri shell: exposes fjord-core to the React UI as commands.

mod commands;

use std::sync::Mutex;

use fjord_core::Store;
use tauri::Manager;

pub struct AppState(pub Mutex<Store>);

fn default_actor() -> String {
    std::env::var("FJORD_ACTOR")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = Store::open(&Store::default_dir(), &default_actor())?;
            app.manage(AppState(Mutex::new(store)));
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fjord");
}
