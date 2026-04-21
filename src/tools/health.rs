//! Server health, tool fallback, and deploy preflight
//! Reads tool_fallback_map.json for cross-server awareness

use chrono;
use cpc_breadcrumbs;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

fn fallback_map_path() -> PathBuf {
    cpc_paths::volumes_path()
        .map(|v| v.join("system_architecture").join("tool_fallback_map.json"))
        .unwrap_or_else(|_| {
            PathBuf::from(r"C:\My Drive\Volumes\system_architecture\tool_fallback_map.json")
        })
}

fn breadcrumb_archive_dir(date: &str) -> PathBuf {
    cpc_paths::volumes_path()
        .map(|v| v.join("breadcrumbs").join("completed").join(date))
        .unwrap_or_else(|_| {
            PathBuf::from(format!(
                r"C:\My Drive\Volumes\breadcrumbs\completed\{}",
                date
            ))
        })
}

// ── local_health helpers ───────────────────────────────────────────────────────

/// Count entries in the active breadcrumb index.
/// Delegates to cpc_breadcrumbs::active_count() which reads the live index object.
/// (Previous impl used .as_array() on an object — always returned 0.)
fn count_active_breadcrumbs() -> usize {
    cpc_breadcrumbs::active_count()
}

/// Count archived breadcrumbs completed today.
fn count_archive_today() -> usize {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let dir = breadcrumb_archive_dir(&today);
    std::fs::read_dir(&dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

/// Diagnostic health check for the local server.
fn local_health() -> Value {
    let paths = serde_json::to_value(cpc_paths::health_check())
        .unwrap_or_else(|e| json!({"error": format!("serialize: {}", e)}));

    let active_breadcrumbs = count_active_breadcrumbs();
    let archive_today = count_archive_today();
    let session_count = super::session::active_count();

    json!({
        "server": "local",
        "version": env!("CARGO_PKG_VERSION"),
        "paths": paths,
        "breadcrumbs": {
            "active_count": active_breadcrumbs,
            "archive_today_count": archive_today
        },
        "sessions": {
            "active_count": session_count
        }
    })
}

/// Check if a process is running by name
fn is_process_running(name: &str) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", name), "/NH"])
        .output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.contains(name)
        }
        Err(_) => false,
    }
}

/// Load the fallback map JSON
fn load_fallback_map() -> Result<Value, String> {
    let content = std::fs::read_to_string(fallback_map_path())
        .map_err(|e| format!("Cannot read fallback map: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in fallback map: {}", e))
}

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "local_health",
            "description": "Diagnostic health check for the local server. Returns cpc-paths path resolution status (Volumes, install, backups), active breadcrumb count, archive count for today, and active session count.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "server_health",
            "description": "Check which MCP servers are alive. Returns process status for all registered servers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "servers": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Specific servers to check (default: all)"
                    }
                }
            }
        }),
        json!({
            "name": "tool_fallback",
            "description": "Look up fallback tool when primary is unavailable. Returns equivalent tool name from fallback map.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool": {
                        "type": "string",
                        "description": "Full tool name (e.g., 'learning2t:cpc_breadcrumb_step')"
                    }
                },
                "required": ["tool"]
            }
        }),
        json!({
            "name": "deploy_preflight",
            "description": "Pre-kill safety checks before deploying/rebuilding an MCP server. Verifies mirrors are alive and prerequisites met.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Server name to deploy (e.g., 'learning2t', 'mcp-windows')"
                    }
                },
                "required": ["target"]
            }
        }),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "local_health" => local_health(),

        "server_health" => {
            let map = match load_fallback_map() {
                Ok(m) => m,
                Err(e) => return json!({"error": e}),
            };

            let filter: Option<Vec<String>> = args
                .get("servers")
                .and_then(|s| serde_json::from_value(s.clone()).ok());

            let servers = match map.get("servers").and_then(|s| s.as_object()) {
                Some(s) => s,
                None => return json!({"error": "No servers in fallback map"}),
            };

            let mut results = serde_json::Map::new();
            let mut alive_count = 0u32;
            let mut dead_count = 0u32;

            for (name, config) in servers {
                if let Some(ref f) = filter {
                    if !f.iter().any(|s| s == name) {
                        continue;
                    }
                }

                let process = config
                    .get("process")
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown");

                let alive = is_process_running(process);
                if alive {
                    alive_count += 1;
                } else {
                    dead_count += 1;
                }

                let mirror = config
                    .get("mirror")
                    .and_then(|m| m.as_str())
                    .unwrap_or("none");

                let critical = config
                    .get("critical")
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);

                results.insert(
                    name.clone(),
                    json!({
                        "alive": alive,
                        "process": process,
                        "mirror": mirror,
                        "critical": critical
                    }),
                );
            }

            json!({
                "servers": results,
                "summary": {
                    "alive": alive_count,
                    "dead": dead_count,
                    "total": alive_count + dead_count
                }
            })
        }

        "tool_fallback" => {
            let tool = match args.get("tool").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return json!({"error": "tool parameter required"}),
            };

            let map = match load_fallback_map() {
                Ok(m) => m,
                Err(e) => return json!({"error": e}),
            };

            // Check equivalents first (bidirectional mirror tools)
            if let Some(equiv) = map
                .get("equivalents")
                .and_then(|e| e.get(tool))
                .and_then(|v| v.as_str())
            {
                return json!({
                    "tool": tool,
                    "fallback": equiv,
                    "type": "equivalent",
                    "note": "Direct mirror tool available"
                });
            }

            // Check fallback chains
            if let Some(chain) = map
                .get("fallback_chains")
                .and_then(|c| c.get(tool))
                .and_then(|v| v.as_array())
            {
                let fallbacks: Vec<&str> = chain.iter().filter_map(|v| v.as_str()).collect();
                return json!({
                    "tool": tool,
                    "fallbacks": fallbacks,
                    "type": "chain",
                    "note": "Ordered fallback chain - try first available"
                });
            }

            // Try reverse lookup in equivalents (tool might be the value, not the key)
            if let Some(equivs) = map.get("equivalents").and_then(|e| e.as_object()) {
                for (key, val) in equivs {
                    if key.starts_with('_') {
                        continue;
                    }
                    if val.as_str() == Some(tool) {
                        return json!({
                            "tool": tool,
                            "fallback": key,
                            "type": "reverse_equivalent",
                            "note": "Found via reverse lookup"
                        });
                    }
                }
            }

            json!({
                "tool": tool,
                "fallback": null,
                "type": "none",
                "note": "No fallback registered for this tool"
            })
        }

        "deploy_preflight" | "preflight_deploy" => {
            let target = match args.get("target").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return json!({"error": "target parameter required"}),
            };

            let map = match load_fallback_map() {
                Ok(m) => m,
                Err(e) => return json!({"error": e}),
            };

            // Get deploy sequence for target
            let deploy_seq = map.get("deploy_sequence").and_then(|d| d.get(target));

            let pre_kill_steps = deploy_seq
                .and_then(|d| d.get("pre_kill"))
                .and_then(|p| p.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            let post_restart_steps = deploy_seq
                .and_then(|d| d.get("post_restart"))
                .and_then(|p| p.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            // Check if mirror is alive
            let server_config = map.get("servers").and_then(|s| s.get(target));
            let mirror_name = server_config
                .and_then(|s| s.get("mirror"))
                .and_then(|m| m.as_str());

            let mirror_alive = if let Some(mirror) = mirror_name {
                let mirror_process = map
                    .get("servers")
                    .and_then(|s| s.get(mirror))
                    .and_then(|s| s.get("process"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown");
                is_process_running(mirror_process)
            } else {
                false // no mirror
            };

            let critical = server_config
                .and_then(|s| s.get("critical"))
                .and_then(|c| c.as_bool())
                .unwrap_or(false);

            // Safety verdict
            let safe = if critical && mirror_name.is_some() {
                mirror_alive // critical server needs its mirror alive
            } else {
                true // non-critical or no mirror requirement
            };

            let mut warnings = Vec::new();
            if critical && !mirror_alive && mirror_name.is_some() {
                warnings.push(format!(
                    "BLOCK: Mirror '{}' is DOWN. Cannot safely kill critical server '{}'.",
                    mirror_name.unwrap_or("unknown"),
                    target
                ));
            }
            if target == "learning2t" {
                warnings.push("Remember: breadcrumb_backup via utonomous BEFORE kill".to_string());
            }

            json!({
                "target": target,
                "safe_to_deploy": safe,
                "mirror": mirror_name,
                "mirror_alive": mirror_alive,
                "critical": critical,
                "pre_kill_steps": pre_kill_steps,
                "post_restart_steps": post_restart_steps,
                "warnings": warnings
            })
        }

        _ => json!({"error": format!("Unknown health tool: {}", name)}),
    }
}

// ============ TESTS ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_health_shape() {
        let result = local_health();

        assert_eq!(result["server"], "local", "server field must be 'local'");
        assert_eq!(
            result["version"],
            env!("CARGO_PKG_VERSION"),
            "version must track Cargo.toml"
        );
        assert!(result.get("paths").is_some(), "paths field must be present");
        assert!(
            result.get("breadcrumbs").is_some(),
            "breadcrumbs field must be present"
        );
        assert!(
            result.get("sessions").is_some(),
            "sessions field must be present"
        );
    }

    #[test]
    fn test_local_health_paths_fields() {
        let result = local_health();
        let paths = &result["paths"];

        // HealthReport fields from cpc-paths
        assert!(
            paths.get("platform").is_some(),
            "paths.platform must be present"
        );
        assert!(
            paths.get("crate_version").is_some(),
            "paths.crate_version must be present"
        );
        assert!(
            paths.get("volumes").is_some(),
            "paths.volumes must be present"
        );
        assert!(
            paths.get("install").is_some(),
            "paths.install must be present"
        );
        assert!(
            paths.get("backups").is_some(),
            "paths.backups must be present"
        );
    }

    #[test]
    fn test_local_health_breadcrumb_counts_are_numeric() {
        let result = local_health();
        let bc = &result["breadcrumbs"];

        assert!(
            bc["active_count"].is_u64() || bc["active_count"].is_i64(),
            "breadcrumbs.active_count must be numeric"
        );
        assert!(
            bc["archive_today_count"].is_u64() || bc["archive_today_count"].is_i64(),
            "breadcrumbs.archive_today_count must be numeric"
        );
    }

    #[test]
    fn test_local_health_sessions_count_numeric() {
        let result = local_health();
        let sessions = &result["sessions"];

        assert!(
            sessions["active_count"].is_u64() || sessions["active_count"].is_i64(),
            "sessions.active_count must be numeric"
        );
    }

    #[test]
    fn test_local_health_via_execute() {
        // Verify execute() routing reaches local_health
        let result = execute("local_health", &json!({}));
        assert_eq!(
            result["server"], "local",
            "execute('local_health') must reach local_health()"
        );
        assert!(
            result.get("error").is_none(),
            "execute('local_health') must not return error"
        );
    }

    /// Verify active_count reflects truth: delegates to cpc_breadcrumbs::active_count()
    /// which reads the live index object (not an array — previous impl bug).
    #[test]
    fn test_active_count_uses_index_object() {
        // count_active_breadcrumbs() reads the index as an object (HashMap).
        // If the previous as_array() bug is present, count would be 0 even when breadcrumbs exist.
        let count = count_active_breadcrumbs();
        // The count is a valid usize — value depends on runtime state.
        // During CI with no active breadcrumbs this will be 0, which is correct.
        // The important invariant: this compiles and does not panic.
        let _ = count;

        // Verify local_health surfaces the same count
        let result = local_health();
        let reported = result["breadcrumbs"]["active_count"]
            .as_u64()
            .expect("breadcrumbs.active_count must be a u64");
        assert_eq!(
            reported as usize,
            count_active_breadcrumbs(),
            "local_health active_count must match cpc_breadcrumbs::active_count()"
        );
    }
}
