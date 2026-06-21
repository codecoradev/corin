pub mod commands;
pub mod uteke_adapter;

use commands::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            // Memory
            commands::remember,
            commands::recall,
            commands::search,
            commands::list,
            commands::forget,
            commands::get_memory,
            // Graph
            commands::get_graph_data,
            commands::get_neighbors,
            commands::add_edge,
            commands::remove_edge,
            // Room
            commands::list_rooms,
            commands::get_room_summary,
            commands::create_room,
            commands::get_room_document,
            // System
            commands::stats,
            commands::list_namespaces,
            commands::list_tags,
            commands::get_settings,
            commands::set_settings,
            commands::export_data,
            commands::import_data,
            commands::open_data_dir,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
