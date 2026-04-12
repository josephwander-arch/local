//! Server health, tool fallback, and deploy preflight
//! Reads tool_fallback_map.json for cross-server awareness

use serde_json::{json, Value};
use std::process::Command;

const FALLBACK_MAP_PATH: &str = "C:\\My Drive\\Volumes\\system_architecture\\tool_fallback_map.json";

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
    let content = std::fs::read_to_string(FALLBACK_MAP_PATH)
        .map_err(|e| format!("Cannot read fallback map: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in fallback map: {}", e))
}

pub fn get_definitions() -> Vec<Value> {
    vec![
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
        "server_health" => {
            let map = match load_fallback_map() {
                Ok(m) => m,
                Err(e) => return json!({"error": e}),
            };

            let filter: Option<Vec<String>> = args.get("servers")
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

                let process = config.get("process")
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown");

                let alive = is_process_running(process);
                if alive { alive_count += 1; } else { dead_count += 1; }

                let mirror = config.get("mirror")
                    .and_then(|m| m.as_str())
                    .unwrap_or("none");

                let critical = config.get("critical")
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);

                results.insert(name.clone(), json!({
                    "alive": alive,
                    "process": process,
                    "mirror": mirror,
                    "critical": critical
                }));
            }

            json!({
                "servers": results,
                "summary": {
                    "alive": alive_count,
                    "dead": dead_count,
                    "total": alive_count + dead_count
                }
            })
        },

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
            if let Some(equiv) = map.get("equivalents").and_then(|e| e.get(tool)).and_then(|v| v.as_str()) {
                return json!({
                    "tool": tool,
                    "fallback": equiv,
                    "type": "equivalent",
                    "note": "Direct mirror tool available"
                });
            }

            // Check fallback chains
            if let Some(chain) = map.get("fallback_chains").and_then(|c| c.get(tool)).and_then(|v| v.as_array()) {
                let fallbacks: Vec<&str> = chain.iter()
                    .filter_map(|v| v.as_str())
                    .collect();
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
                    if key.starts_with('_') { continue; }
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
        },

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
            let deploy_seq = map.get("deploy_sequence")
                .and_then(|d| d.get(target));

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
                let mirror_process = map.get("servers")
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
                warnings.push(format!("BLOCK: Mirror '{}' is DOWN. Cannot safely kill critical server '{}'.", 
                    mirror_name.unwrap_or("unknown"), target));
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
        },

        _ => json!({"error": format!("Unknown health tool: {}", name)}),
    }
}
