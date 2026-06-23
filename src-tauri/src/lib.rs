pub mod commands;
pub mod config;
pub mod uteke_client;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use commands::AppState;
use config::init_environment;
use uteke_client::UtekeClient;

/// Check if a TCP port is listening (server already running).
fn is_port_open(host: &str, port: u16) -> bool {
    std::net::TcpStream::connect((host, port)).is_ok()
}

/// Ensure uteke-serve is running.
///
/// If the server is already reachable at the detected URL, do nothing.
/// Otherwise, try to start `uteke-serve` as a detached background process
/// (if the binary is in PATH) and wait briefly for it to come up.
///
/// Returns the server URL on success.
fn ensure_uteke_server() -> String {
    let server_url = config::detect_uteke_serve_url();

    // Parse host:port from the URL (e.g. "http://127.0.0.1:8767").
    let (host, port) = match parse_host_port(&server_url) {
        Some(hp) => hp,
        None => return server_url,
    };

    // Already running? Nothing to do.
    if is_port_open(&host, port) {
        return server_url;
    }

    // Try to start uteke-serve from PATH.
    if let Some(uteke_serve) = find_in_path("uteke-serve") {
        eprintln!("CorIn: starting uteke-serve ({})...", uteke_serve.display());

        let result = std::process::Command::new(&uteke_serve)
            .arg("--host").arg(&host)
            .arg("--port").arg(port.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                eprintln!("CorIn: uteke-serve started (PID {})", child.id());
                // Wait up to 5 seconds for the server to become available.
                for _ in 0..50 {
                    if is_port_open(&host, port) {
                        eprintln!("CorIn: uteke-serve is ready at {server_url}");
                        return server_url;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                eprintln!("CorIn: uteke-serve did not become ready within 5s");
            }
            Err(e) => {
                eprintln!("CorIn: failed to start uteke-serve: {e}");
            }
        }
    } else {
        eprintln!(
            "CorIn: uteke-serve not found in PATH — semantic search will be unavailable. \
             Install uteke or start uteke-serve manually."
        );
    }

    server_url
}

/// Extract (host, port) from a URL like `http://127.0.0.1:8767`.
fn parse_host_port(url: &str) -> Option<(String, u16)> {
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
    let (host, port_str) = rest.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    Some((host.to_string(), port))
}

/// Find an executable in PATH.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}

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
            commands::uteke_get,
            commands::uteke_graph,
            commands::uteke_list,
            commands::uteke_neighbors,
            commands::uteke_namespaces,
            commands::uteke_rooms,
            commands::uteke_room_recall,
            commands::uteke_search,
            commands::uteke_stats,
            // Uteke Server Integration (HTTP)
            commands::uteke_server_status,
            commands::uteke_recall,
            commands::uteke_remember,
            commands::uteke_forget,
            commands::uteke_server_graph,
            commands::uteke_server_stats,
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
                            s.data_dir = config::corin_dir().ok();
                            s.db_path = Some(db_path);
                            s.conn = Some(conn);

                            // All Uteke access via HTTP API (no direct SQLite).
                            // Ensure uteke-serve is running — auto-start from
                            // PATH if needed, then create the HTTP client.
                            let server_url = ensure_uteke_server();
                            let client = UtekeClient::new(&server_url);
                            s.uteke_client = Some(client);
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

/// Initialize the SQLite schema for CorIn database.
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
