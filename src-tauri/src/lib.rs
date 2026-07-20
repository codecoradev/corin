pub mod commands;
pub mod config;
pub mod connections;
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
pub(crate) fn ensure_uteke_server() -> String {
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
///
/// Windows-aware: tries `uteke-serve.exe` (and other `PATHEXT` exts) in
/// both PATH and `~/.local/bin`. Fixes #171.
fn find_uteke_serve() -> Option<std::path::PathBuf> {
    // Try PATH first.
    if let Some(p) = find_in_path("uteke-serve") {
        return Some(p);
    }
    // Try ~/.local/bin/uteke-serve[-.exe] (install script default).
    if let Some(home) = dirs::home_dir() {
        for n in candidate_filenames("uteke-serve") {
            let candidate = home.join(".local/bin").join(n);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Minimum uteke version required for the global Documents feature.
/// v0.7.0 introduced global docs (#614), but document delete/move landed in
/// v0.7.1 — require it so the full doc feature set is available.
pub(crate) const MIN_UTEKE_FOR_DOCS: &str = "0.7.1";

/// Find the `uteke` CLI binary in PATH or ~/.local/bin.
///
/// On Windows the binary is `uteke.exe` (not `uteke`), so both the PATH
/// scan and the `~/.local/bin` fallback try every `PATHEXT` extension —
/// see [`candidate_filenames`]. Fixes #171.
pub(crate) fn find_uteke_cli() -> Option<std::path::PathBuf> {
    if let Some(p) = find_in_path("uteke") {
        return Some(p);
    }
    if let Some(home) = dirs::home_dir() {
        for n in candidate_filenames("uteke") {
            let candidate = home.join(".local/bin").join(n);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Detect the installed uteke CLI version by shelling `uteke --version`.
///
/// Returns the parsed `X.Y.Z` string (e.g. `"0.7.0"`), or `None` if the
/// binary is missing or the output can't be parsed. The HTTP server exposes
/// no version endpoint, so the CLI is the only source of truth.
pub(crate) fn detect_uteke_version() -> Option<String> {
    let uteke = find_uteke_cli()?;
    let out = std::process::Command::new(uteke)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // Output looks like "uteke 0.7.0". Take the last whitespace-separated token
    // that contains a '.' so stray suffixes don't leak in.
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .last()
        .filter(|s| s.contains('.'))
        .map(|s| s.to_string())
}

/// Compare two `X.Y.Z` version strings. Returns `true` if `current >= min`.
///
/// If either side fails to parse, returns `false` — conservative, so an
/// unknown/unparseable version prompts an upgrade rather than passing.
pub(crate) fn version_meets(current: &str, min: &str) -> bool {
    fn parse(v: &str) -> Option<(u32, u32, u32)> {
        let mut parts = v.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        Some((major, minor, patch))
    }
    match (parse(current), parse(min)) {
        (Some(c), Some(m)) => c >= m,
        _ => false,
    }
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

/// Expand a bare executable `name` against a `PATHEXT`-style list.
///
/// `pathext` is a `;`-separated list of extensions, each including the
/// leading dot (e.g. `".EXE;.COM;.BAT;.CMD"` — the shape of Windows'
/// `PATHEXT` env var). Returns `name` with each extension appended.
/// Empty entries are skipped.
///
/// Pure and platform-independent so the Windows path-resolution logic can
/// be unit-tested on any OS. Fixes #171.
#[cfg(any(windows, test))]
fn expand_with_pathext(name: &str, pathext: &str) -> Vec<String> {
    pathext
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|ext| format!("{name}{ext}"))
        .collect()
}

/// Filenames to probe when resolving `name` on the current platform.
///
/// - **Windows**: the bare `name` is never the on-disk binary — Windows
///   executables carry an extension (`.exe`, `.cmd`, …). The shell searches
///   `PATHEXT` to pick one; we mirror that, falling back to a sensible
///   default (`.EXE;.COM;.BAT;.CMD;.VBS;.JS;.WS;.MSC`) when `PATHEXT` is
///   unset. If `PATHEXT` is explicitly empty, the Windows default list is
///   still used (a bare name is never executable on Windows).
/// - **Unix**: executables have no extension, so only `name` itself is tried.
fn candidate_filenames(name: &str) -> Vec<std::path::PathBuf> {
    #[cfg(windows)]
    {
        let default = ".EXE;.COM;.BAT;.CMD;.VBS;.JS;.WS;.MSC";
        let pathext = std::env::var("PATHEXT")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string());
        expand_with_pathext(name, &pathext)
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![std::path::PathBuf::from(name)]
    }
}

/// Find an executable in PATH.
///
/// On Windows, also probes every `PATHEXT` extension (e.g. `name.exe`) per
/// PATH entry — see [`candidate_filenames`]. Fixes #171.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    let names = candidate_filenames(name);
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            names.iter().find_map(|n| {
                let candidate = dir.join(n);
                if candidate.is_file() {
                    Some(candidate)
                } else {
                    None
                }
            })
        })
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
            commands::delete_room,
            // System
            commands::stats,
            commands::list_namespaces,
            commands::list_tags,
            commands::get_settings,
            commands::set_settings,
            commands::export_data,
            commands::import_preview,
            commands::import_data,
            commands::init_data_dir,
            // AI Agent Integration (#55)
            commands::detect_agents,
            commands::generate_agent_md,
            commands::run_dream_cycle,
            commands::get_dream_history,
            // Uteke Integration
            commands::uteke_available,
            commands::uteke_get,
            commands::uteke_graph,
            commands::uteke_list,
            commands::uteke_recent,
            commands::uteke_neighbors,
            commands::uteke_namespaces,
            commands::uteke_namespaces_with_counts,
            commands::uteke_rooms,
            commands::uteke_room_recall,
            commands::uteke_room_memories,
            commands::uteke_room_stats,
            commands::uteke_search,
            commands::uteke_stats,
            // Uteke Server Integration (HTTP)
            commands::uteke_server_status,
            commands::uteke_recall,
            commands::uteke_remember,
            commands::uteke_forget,
            commands::uteke_server_graph,
            commands::uteke_server_stats,
            // Connection manager (#37)
            commands::list_connections,
            commands::add_connection,
            commands::update_connection,
            commands::delete_connection,
            commands::test_connection,
            commands::set_primary_connection,
            commands::reconnect_connection,
            commands::disconnect_connection,
            // Document Engine (#137)
            commands::uteke_version_status,
            commands::uteke_self_update,
            commands::doc_list,
            commands::doc_get,
            commands::doc_create,
            commands::doc_update,
            commands::doc_search,
            commands::doc_delete,
            commands::doc_move,
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

                            // Security: restrict db file perms to owner-only (0600).
                            // The db holds auth tokens for remote connections.
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let _ = std::fs::set_permissions(
                                    &db_path,
                                    std::fs::Permissions::from_mode(0o600),
                                );
                            }
                            s.db_path = Some(db_path);
                            s.conn = Some(conn);

                            // All Uteke access via HTTP API (no direct SQLite).
                            // Resolve server URL: DB primary → env UTEKE_SERVER_URL → TOML → default.
                            let db_conn = s.conn.as_ref();
                            let (server_url, auth_token) = config::resolve_uteke_server(db_conn);

                            // Only auto-start local uteke-serve if URL is local.
                            if !config::is_remote_url(&server_url) {
                                let resolved = ensure_uteke_server();
                                // Use resolved URL (may differ if auto-start changed port).
                                let client = UtekeClient::with_auth(&resolved, auth_token);
                                s.uteke_client = Some(client);

                                // Seed default local uteke connection on first boot.
                                if let Ok(true) =
                                    connections::store::is_empty(s.conn.as_ref().unwrap())
                                {
                                    match connections::store::seed_default(
                                        s.conn.as_mut().unwrap(),
                                        &resolved,
                                    ) {
                                        Ok(id) => eprintln!(
                                            "CorIn: seeded default connection {id} at {resolved}"
                                        ),
                                        Err(e) => eprintln!(
                                            "CorIn: failed to seed default connection: {e}"
                                        ),
                                    }
                                }
                            } else {
                                // Remote URL — no local auto-start.
                                eprintln!("CorIn: using remote uteke-serve at {server_url}");
                                let client = UtekeClient::with_auth(&server_url, auth_token);
                                s.uteke_client = Some(client);
                            }

                            // Cache whether we're talking to a user-managed remote
                            // server (vs. a locally spawned uteke-serve).
                            s.uteke_remote = config::is_remote_url(&server_url);

                            // Cache the installed uteke version for feature
                            // gating (e.g. Documents requires >= 0.7.1).
                            // For remote servers the authoritative version is
                            // probed live from /health in the gating commands;
                            // the local CLI version is only a fallback there.
                            s.uteke_version = detect_uteke_version();
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
        CREATE TABLE IF NOT EXISTS connections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            product_type TEXT NOT NULL,
            url TEXT NOT NULL,
            auth_type TEXT,
            auth_token TEXT,
            metadata TEXT DEFAULT '{}',
            status TEXT DEFAULT 'unknown',
            is_primary INTEGER DEFAULT 0,
            created_at TEXT,
            last_tested_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_connections_type ON connections(product_type);
        CREATE INDEX IF NOT EXISTS idx_connections_primary ON connections(is_primary);
        CREATE TABLE IF NOT EXISTS dream_run_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ran_at TEXT NOT NULL,
            success INTEGER NOT NULL DEFAULT 1,
            total_changes INTEGER NOT NULL DEFAULT 0,
            total_warnings INTEGER NOT NULL DEFAULT 0,
            total_errors INTEGER NOT NULL DEFAULT 0,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            phases_json TEXT DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_dream_history_ran_at ON dream_run_history(ran_at);
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_meets_equal_or_higher() {
        assert!(version_meets("0.7.0", "0.7.0"));
        assert!(version_meets("0.7.1", "0.7.0"));
        assert!(version_meets("0.8.0", "0.7.0"));
        assert!(version_meets("1.0.0", "0.7.0"));
    }

    #[test]
    fn docs_gate_requires_0_7_1() {
        // Document delete/move landed in 0.7.1; 0.7.0 must be rejected so the
        // UI prompts for an upgrade instead of hitting a missing /doc/delete.
        assert!(!version_meets("0.7.0", MIN_UTEKE_FOR_DOCS));
        assert!(version_meets("0.7.1", MIN_UTEKE_FOR_DOCS));
        assert!(version_meets("0.8.0", MIN_UTEKE_FOR_DOCS));
    }

    #[test]
    fn version_meets_lower_rejected() {
        assert!(!version_meets("0.6.7", "0.7.0"));
        assert!(!version_meets("0.6.0", "0.7.0"));
    }

    #[test]
    fn version_meets_unparseable_is_conservative() {
        // A fully unparseable current must NOT pass the gate.
        // (Pre-release suffixes like "0.7.0-rc" parse to 0.7.0 and are accepted —
        // acceptable, since uteke ships clean semver tags in practice.)
        assert!(!version_meets("unknown", "0.7.0"));
    }

    // ── Windows PATHEXT resolution (#171) ───────────────────────────────
    //
    // `expand_with_pathext` is pure, so these guard the Windows binary-
    // discovery logic on every OS — even though CI currently runs only on
    // Linux.

    #[test]
    fn pathext_expands_each_extension() {
        let out = expand_with_pathext("uteke", ".EXE;.COM;.BAT;.CMD");
        assert_eq!(
            out,
            vec!["uteke.EXE", "uteke.COM", "uteke.BAT", "uteke.CMD"]
        );
    }

    #[test]
    fn pathext_skips_empty_entries() {
        // Windows PATHEXT can carry stray/double semicolons.
        let out = expand_with_pathext("uteke-serve", ".EXE;;.BAT;");
        assert_eq!(out, vec!["uteke-serve.EXE", "uteke-serve.BAT"]);
    }

    #[test]
    fn pathext_preserves_case_as_given() {
        // We append PATHEXT entries verbatim. Windows is case-insensitive at
        // the FS layer, so `.EXE` and `.exe` both match `uteke.exe` on disk.
        let out = expand_with_pathext("uteke", ".exe");
        assert_eq!(out, vec!["uteke.exe"]);
    }
}
