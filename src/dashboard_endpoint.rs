//! HTTP dashboard endpoint for the local server.
//!
//! GET  /api/status          → JSON: health, breadcrumbs, sessions
//! POST /api/action/clean_old → delete *.exe.old in install_path
//!
//! Port: CPC_DASHBOARD_PORT_LOCAL env var, default 9101.
//! Binds 127.0.0.1 only. Falls back through +5 ports if primary is taken.
//! Graceful: if all ports fail, logs a warning and returns — MCP continues normally.

use serde_json::{json, Value};
use std::thread;

const DEFAULT_PORT: u16 = 9101;
const ENV_PORT: &str = "CPC_DASHBOARD_PORT_LOCAL";

/// Spawn the dashboard HTTP server on an isolated thread.
pub fn spawn() {
    thread::Builder::new()
        .name("local-dashboard".into())
        .spawn(move || {
            let base_port: u16 = std::env::var(ENV_PORT)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_PORT);

            let server = match try_bind(base_port) {
                Some(s) => s,
                None => {
                    eprintln!(
                        "[local/dashboard] Could not bind on ports {}–{}. \
                         MCP continues without dashboard endpoint.",
                        base_port,
                        base_port + 5
                    );
                    return;
                }
            };

            let port = server
                .server_addr()
                .to_ip()
                .map(|a| a.port())
                .unwrap_or(base_port);
            eprintln!(
                "[local/dashboard] Listening on http://127.0.0.1:{}/api/status",
                port
            );

            for request in server.incoming_requests() {
                handle_request(request);
            }
        })
        .ok();
}

fn try_bind(base_port: u16) -> Option<tiny_http::Server> {
    for port in base_port..base_port + 6 {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(s) = tiny_http::Server::http(&addr) {
            return Some(s);
        }
    }
    None
}

fn cors_headers() -> Vec<tiny_http::Header> {
    vec![
        "Access-Control-Allow-Origin: *".parse().unwrap(),
        "Access-Control-Allow-Methods: GET, POST, OPTIONS"
            .parse()
            .unwrap(),
        "Access-Control-Allow-Headers: Content-Type"
            .parse()
            .unwrap(),
        "Content-Type: application/json".parse().unwrap(),
    ]
}

fn respond(request: tiny_http::Request, status: u16, body: Value) {
    let body_str = serde_json::to_string(&body).unwrap_or_default();
    let mut response = tiny_http::Response::from_string(body_str).with_status_code(status);
    for h in cors_headers() {
        response = response.with_header(h);
    }
    let _ = request.respond(response);
}

fn handle_request(request: tiny_http::Request) {
    let method = request.method().as_str().to_uppercase();
    let url = request.url().split('?').next().unwrap_or("").to_string();

    match (method.as_str(), url.as_str()) {
        ("GET", "/api/status") => respond(request, 200, build_status()),
        ("POST", "/api/action/clean_old") => respond(request, 200, clean_old_binaries()),
        ("OPTIONS", _) => respond(request, 204, json!({})),
        _ => respond(request, 404, json!({"error": "Not found"})),
    }
}

// ── Status builder ─────────────────────────────────────────────────────────────

fn build_status() -> Value {
    let paths = serde_json::to_value(cpc_paths::health_check())
        .unwrap_or_else(|_| json!({"error": "serialize failed"}));

    let install_path = resolve_install_path(&paths);
    let old_binaries = list_old_binaries(&install_path);

    // Active breadcrumbs with full detail
    let all_bcs = cpc_breadcrumbs::list_active();
    let active: Vec<Value> = all_bcs
        .iter()
        .filter(|bc| !bc.aborted)
        .map(|bc| {
            json!({
                "id": bc.id,
                "name": bc.name,
                "project_id": bc.project_id,
                "current_step": bc.current_step,
                "total_steps": bc.total_steps,
                "owner": bc.owner,
                "stale": bc.is_stale(),
                "last_activity_at": bc.last_activity_at,
                "files_changed": bc.files_changed.len()
            })
        })
        .collect();
    let active_count = active.len();
    let archive_today_count = count_archive_today();

    // Sessions
    let sessions_list = crate::tools::session::list_active_sessions();
    let session_count = sessions_list.len();

    json!({
        "server": "local",
        "version": "1.2.10",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "health": {
            "paths": paths,
            "old_binaries": old_binaries
        },
        "breadcrumbs": {
            "active": active,
            "active_count": active_count,
            "archive_today_count": archive_today_count
        },
        "sessions": {
            "active_count": session_count,
            "active": sessions_list
        }
    })
}

fn resolve_install_path(paths: &Value) -> String {
    // cpc_paths::health_check() serializes to a struct with an "install" field.
    if let Some(s) = paths.get("install").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    // Some versions nest under "paths"
    if let Some(s) = paths.pointer("/paths/install").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    r"C:\CPC\servers".to_string()
}

fn list_old_binaries(dir: &str) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".exe.old") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn count_archive_today() -> usize {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let dir = format!(r"C:\My Drive\Volumes\breadcrumbs\completed\{}", today);
    std::fs::read_dir(&dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

fn clean_old_binaries() -> Value {
    let paths = serde_json::to_value(cpc_paths::health_check()).unwrap_or_default();
    let install_path = resolve_install_path(&paths);
    let old_files = list_old_binaries(&install_path);
    let mut deleted: Vec<String> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();

    for name in &old_files {
        let path = std::path::Path::new(&install_path).join(name);
        match std::fs::remove_file(&path) {
            Ok(_) => deleted.push(name.clone()),
            Err(e) => errors.push(json!({"file": name, "error": e.to_string()})),
        }
    }

    json!({
        "deleted": deleted,
        "count": deleted.len(),
        "errors": errors
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_has_required_fields() {
        let status = build_status();
        assert_eq!(status["server"], "local");
        assert_eq!(status["version"], "1.2.10");
        assert!(
            status["timestamp"].is_string(),
            "timestamp must be a string"
        );
        assert!(status["health"].is_object(), "health must be an object");
        assert!(status["health"]["paths"].is_object() || status["health"]["paths"].is_null());
        assert!(
            status["breadcrumbs"].is_object(),
            "breadcrumbs must be an object"
        );
        assert!(
            status["breadcrumbs"]["active"].is_array(),
            "active must be an array"
        );
        assert!(status["breadcrumbs"]["active_count"].is_number());
        assert!(status["sessions"].is_object(), "sessions must be an object");
        assert!(status["sessions"]["active_count"].is_number());
    }

    #[test]
    fn test_port_fallback_range() {
        // The fallback range must cover 6 consecutive ports.
        let base = DEFAULT_PORT;
        let range: Vec<u16> = (base..base + 6).collect();
        assert_eq!(range.len(), 6);
        assert_eq!(range[0], 9101);
        assert_eq!(range[5], 9106);
    }

    #[test]
    fn test_list_old_binaries_nonexistent_dir() {
        let result = list_old_binaries(r"C:\nonexistent\path\that\does\not\exist");
        assert!(result.is_empty(), "should return empty vec for missing dir");
    }

    #[test]
    fn test_resolve_install_path_fallback() {
        let paths = serde_json::json!({});
        let p = resolve_install_path(&paths);
        assert_eq!(p, r"C:\CPC\servers");
    }
}
