pub mod commands;
pub mod config;
pub mod uteke_adapter;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use commands::AppState;
use config::init_environment;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            commands::init_data_dir,
            // Uteke Integration
            commands::uteke_available,
            commands::uteke_list,
            commands::uteke_search,
            commands::uteke_stats,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            // Auto-initialize Codecora environment on startup (synchronous)
            // Creates ~/.codecora/ structure and opens Hub database
            match init_environment() {
                Ok((db_path, _config)) => {
                    match rusqlite::Connection::open(&db_path) {
                        Ok(conn) => {
                            if let Err(e) = init_schema(&conn) {
                                eprintln!("Failed to init schema: {e}");
                            }
                            // Store in managed state
                            let state = app.state::<Arc<Mutex<AppState>>>();
                            let state = state.clone();
                            // Use blocking lock since this runs before the event loop
                            let mut s = state.blocking_lock();
                            s.data_dir = config::hub_dir().ok();
                            s.db_path = Some(db_path);
                            s.conn = Some(conn);

                            // Open read-only connection to Uteke DB if available
                            if let Some(uteke_path) = config::detect_uteke() {
                                let uteke_db = uteke_path.join("uteke.db");
                                match rusqlite::Connection::open_with_flags(
                                    &uteke_db,
                                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                                        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                                ) {
                                    Ok(uteke_conn) => {
                                        s.uteke_db_path = Some(uteke_db);
                                        s.uteke_conn = Some(uteke_conn);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to open Uteke DB: {e}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to open database: {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to init Codecora environment: {e}");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initialize the SQLite schema for Hub database.
fn init_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '[]',
            content_type TEXT,
            importance REAL,
            namespace TEXT,
            created_at TEXT,
            updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS graph_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            edge_type TEXT DEFAULT 'related',
            weight REAL DEFAULT 1.0,
            created_at TEXT
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
        CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories(updated_at);
        CREATE INDEX IF NOT EXISTS idx_edges_source ON graph_edges(source);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON graph_edges(target);
        ",
    )?;
    Ok(())
}
