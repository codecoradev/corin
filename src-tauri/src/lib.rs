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

/// Check if the device has enough resources to run uteke-serve.
///
/// uteke-serve loads an ONNX embedding model into memory (~200-400MB)
/// and keeps the SQLite + vector index warm. On low-spec machines
/// (e.g. <4GB RAM or <2 cores), we skip auto-start and let the user
/// decide when to run it manually.
///
/// Returns `true` if the device can comfortably run the server.
fn device_can_run_server() -> bool {
    // Check CPU cores.
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    if cores < 2 {
        eprintln!(
            "CorIn: device has {cores} CPU core(s) — skipping uteke-serve auto-start (need ≥2)"
        );
        return false;
    }

    // Check available memory (macOS: sysctl hw.memsize; Linux: /proc/meminfo).
    let total_mem_gb = get_total_memory_gb();
    if total_mem_gb > 0.0 && total_mem_gb < 4.0 {
        eprintln!(
            "CorIn: device has {total_mem_gb:.1}GB RAM — skipping uteke-serve auto-start (need ≥4GB)"
        );
        return false;
    }

    true
}

/// Get total system memory in GB. Returns 0 if unknown.
fn get_total_memory_gb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output();
        if let Ok(o) = out
            && let Ok(s) = String::from_utf8(o.stdout)
            && let Ok(bytes) = s.trim().parse::<u64>()
        {
            return bytes as f64 / 1_073_741_824.0;
        }
        0.0
    }
    #[cfg(target_os = "linux")]
    {
        let Ok(contents) = std::fs::read_to_string("/proc/meminfo") else {
            return 0.0;
        };
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
                return kb as f64 / 1_048_576.0;
            }
        }
        0.0
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        0.0
    }
}

/// Ensure uteke is installed and the server is running.
///
/// Flow:
/// 1. If server already reachable at detected URL → done.
/// 2. Check if uteke-serve binary exists (PATH or ~/.local/bin).
///    If not → run install script.
/// 3. Check device specs (≥2 cores, ≥4GB RAM).
///    If too weak → skip server start (user runs manually).
/// 4. Start uteke-serve as detached process.
///
/// Returns the server URL.
fn ensure_uteke_server() -> String {
    let server_url = config::detect_uteke_serve_url();

    let (host, port) = match parse_host_port(&server_url) {
        Some(hp) => hp,
        None => return server_url,
    };

    // 1. Already running?
    if is_port_open(&host, port) {
        eprintln!("CorIn: uteke-serve already running at {server_url}");
        return server_url;
    }

    // 2. Check if uteke-serve is installed.
    let uteke_serve = match find_uteke_serve() {
        Some(path) => {
            eprintln!("CorIn: uteke-serve found at {}", path.display());
            path
        }
        None => {
            eprintln!("CorIn: uteke-serve not found — installing...");
            if !install_uteke() {
                eprintln!(
                    "CorIn: install failed — semantic search unavailable. \
                     Install manually: curl -fsSL https://raw.githubusercontent.com/codecoradev/uteke/main/install.sh | sh"
                );
                return server_url;
            }
            match find_uteke_serve() {
                Some(path) => path,
                None => {
                    eprintln!("CorIn: uteke-serve still not found after install");
                    return server_url;
                }
            }
        }
    };

    // 3. Check device specs before starting server.
    if !device_can_run_server() {
        eprintln!(
            "CorIn: skipping uteke-serve auto-start (device too low-spec). \
             Start manually: uteke-serve"
        );
        return server_url;
    }

    // 4. Start the server.
    eprintln!("CorIn: starting uteke-serve...");
    let result = std::process::Command::new(&uteke_serve)
        .arg("--host")
        .arg(&host)
        .arg("--port")
        .arg(port.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match result {
        Ok(child) => {
            eprintln!("CorIn: uteke-serve started (PID {})", child.id());
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

    server_url
}

/// Find uteke-serve binary in PATH or ~/.local/bin.
fn find_uteke_serve() -> Option<std::path::PathBuf> {
    // Try PATH first.
    if let Some(p) = find_in_path("uteke-serve") {
        return Some(p);
    }
    // Try ~/.local/bin/uteke-serve (install script default).
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".local/bin/uteke-serve");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Run the official uteke install script.
///
/// Downloads `install.sh` from GitHub and pipes it to `sh`.
/// Installs both `uteke` and `uteke-serve` into `~/.local/bin/`.
///
/// Returns `true` if the install succeeded (exit code 0).
fn install_uteke() -> bool {
    eprintln!("CorIn: installing uteke via official script...");

    let curl = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "https://raw.githubusercontent.com/codecoradev/uteke/main/install.sh",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn();

    let mut curl_child = match curl {
        Ok(c) => c,
        Err(e) => {
            eprintln!("CorIn: failed to run curl for install: {e}");
            return false;
        }
    };

    let stdout = curl_child.stdout.take();

    let mut sh_cmd = std::process::Command::new("sh");
    sh_cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if let Some(stdout) = stdout {
        sh_cmd.stdin(stdout);
    } else {
        sh_cmd.stdin(std::process::Stdio::null());
    }

    let sh_status = match sh_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("CorIn: failed to run sh for install: {e}");
            let _ = curl_child.kill();
            return false;
        }
    };

    // Ensure curl is reaped.
    let _ = curl_child.wait();

    // Ensure ~/.local/bin is in PATH for subsequent lookups.
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin");
        if let Some(path) = std::env::var_os("PATH") {
            let mut paths: Vec<std::path::PathBuf> = std::env::split_paths(&path).collect();
            if !paths.contains(&local_bin) {
                paths.insert(0, local_bin);
                let new_path = std::env::join_paths(paths).unwrap_or(path);
                // SAFETY: setting PATH is safe in single-threaded setup context.
                unsafe {
                    std::env::set_var("PATH", new_path);
                }
            }
        }
    }

    let ok = sh_status.success();
    if ok {
        eprintln!("CorIn: uteke installed successfully");
    } else {
        eprintln!("CorIn: uteke install script exited with non-zero status");
    }
    ok
}

/// Extract (host, port) from a URL like `http://127.0.0.1:8767`.
fn parse_host_port(url: &str) -> Option<(String, u16)> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
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
            // AI Agent Integration (#55)
            commands::detect_agents,
            commands::generate_agent_md,
            commands::run_dream_cycle,
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
