//! Command shortcuts - compound commands that save tokens
//! Supports both hardcoded defaults and custom shortcuts from JSON config

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;

/// Shortcut definitions - name, commands, description
const SHORTCUTS: &[(&str, &[&str], &str)] = &[
    // Build shortcuts
    ("cargo_check_build", &["cargo check", "cargo build --release"], "Check then build Rust project"),
    ("npm_install_build", &["npm install", "npm run build"], "Install deps then build"),
    ("npm_clean_install", &["rd /s /q node_modules", "npm install"], "Clean reinstall node_modules"),
    ("pip_upgrade_all", &["pip list --outdated --format=freeze | %{$_.split('==')[0]} | %{pip install -U $_}"], "Upgrade all pip packages"),

    // Cleanup shortcuts
    ("clean_temp", &["rd /s /q %TEMP%\\*", "mkdir %TEMP%"], "Clear Windows temp folder"),
    ("clean_rust_target", &["cargo clean"], "Clean Rust build artifacts"),
    ("clean_npm_cache", &["npm cache clean --force"], "Clear npm cache"),

    // Development shortcuts
    ("dev_server_next", &["npm run dev"], "Start Next.js dev server"),
    ("dev_server_expo", &["npx expo start"], "Start Expo dev server"),
    ("dev_server_python", &["python -m http.server 8000"], "Start Python HTTP server"),

    // System info shortcuts
    ("system_info", &["systeminfo | findstr /B /C:\"OS\" /C:\"Total Physical\""], "Quick system info"),
    ("disk_usage", &["wmic logicaldisk get size,freespace,caption"], "Disk space summary"),
    ("process_top", &["Get-Process | Sort-Object CPU -Descending | Select-Object -First 10 Name, CPU, WorkingSet"], "Top 10 CPU processes"),

    // Network shortcuts
    ("network_info", &["ipconfig | findstr /i \"IPv4 Default\""], "Quick IP info"),
    ("ports_listening", &["netstat -an | findstr LISTENING"], "List listening ports"),
    ("flush_dns", &["ipconfig /flushdns"], "Flush DNS cache"),
];

/// Custom shortcut from JSON config
#[derive(Debug, Deserialize, Serialize, Clone)]
struct CustomShortcut {
    name: String,
    description: String,
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CustomShortcutsConfig {
    shortcuts: Vec<CustomShortcut>,
}

/// Path to custom shortcuts JSON
const CUSTOM_SHORTCUTS_PATH: &str = "C:\\My Drive\\Volumes\\config\\custom_shortcuts.json";

/// Load custom shortcuts from JSON file
fn load_custom_shortcuts() -> Vec<CustomShortcut> {
    match fs::read_to_string(CUSTOM_SHORTCUTS_PATH) {
        Ok(content) => match serde_json::from_str::<CustomShortcutsConfig>(&content) {
            Ok(config) => config.shortcuts,
            Err(e) => {
                eprintln!("Failed to parse custom_shortcuts.json: {}", e);
                Vec::new()
            }
        },
        Err(_) => Vec::new(), // File doesn't exist, that's fine
    }
}

/// Tool definitions
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "shortcut_list",
            "description": "List all available command shortcuts.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "shortcut_run",
            "description": "Run a predefined shortcut (compound command).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Shortcut name (use shortcut_list to see available)"
                    },
                    "session": {
                        "type": "string",
                        "description": "Session to run in (default: 'default')"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "shortcut_chain",
            "description": "Run multiple commands in sequence with checkpointing. Stops on first error.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Commands to run in sequence"
                    },
                    "session": {
                        "type": "string",
                        "description": "Session to run in (default: 'default')"
                    },
                    "checkpoint": {
                        "type": "boolean",
                        "description": "Save checkpoint after each command (default: true for 3+ commands)"
                    },
                    "stop_on_error": {
                        "type": "boolean",
                        "description": "Stop chain on first error (default: true)"
                    }
                },
                "required": ["commands"]
            }
        }),
    ]
}

/// Execute shortcut tools
pub fn execute(name: &str, args: &Value, session_executor: fn(&str, &Value) -> Value) -> Value {
    match name {
        "shortcut_list" => {
            // Built-in shortcuts
            let mut shortcuts: Vec<Value> = SHORTCUTS
                .iter()
                .map(|(name, cmds, desc)| {
                    json!({
                        "name": name,
                        "commands": cmds,
                        "description": desc,
                        "source": "builtin"
                    })
                })
                .collect();

            // Custom shortcuts from JSON
            let custom = load_custom_shortcuts();
            for cs in &custom {
                shortcuts.push(json!({
                    "name": cs.name,
                    "commands": cs.commands,
                    "description": cs.description,
                    "source": "custom"
                }));
            }

            json!({
                "shortcuts": shortcuts,
                "count": shortcuts.len(),
                "builtin_count": SHORTCUTS.len(),
                "custom_count": custom.len(),
                "custom_config": CUSTOM_SHORTCUTS_PATH
            })
        }

        "shortcut_run" => {
            let shortcut_name = match args["name"].as_str() {
                Some(n) => n,
                None => return json!({"error": "shortcut name required"}),
            };
            let session = args["session"].as_str().unwrap_or("default");

            // First check built-in shortcuts
            let builtin = SHORTCUTS.iter().find(|(n, _, _)| *n == shortcut_name);

            if let Some((_, commands, desc)) = builtin {
                return run_shortcut_commands(
                    shortcut_name,
                    commands.to_vec(),
                    desc,
                    "builtin",
                    session,
                    session_executor,
                );
            }

            // Then check custom shortcuts
            let custom = load_custom_shortcuts();
            if let Some(cs) = custom.iter().find(|s| s.name == shortcut_name) {
                let cmds: Vec<&str> = cs.commands.iter().map(|s| s.as_str()).collect();
                return run_shortcut_commands(
                    shortcut_name,
                    cmds,
                    &cs.description,
                    "custom",
                    session,
                    session_executor,
                );
            }

            json!({
                "error": format!("Unknown shortcut: {}", shortcut_name),
                "hint": "Use shortcut_list to see available shortcuts"
            })
        }

        "shortcut_chain" => {
            let commands: Vec<&str> = match args["commands"].as_array() {
                Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
                None => return json!({"error": "commands array required"}),
            };

            if commands.is_empty() {
                return json!({"error": "commands array is empty"});
            }

            let session = args["session"].as_str().unwrap_or("default");
            let checkpoint = args["checkpoint"].as_bool().unwrap_or(commands.len() >= 3);
            let stop_on_error = args["stop_on_error"].as_bool().unwrap_or(true);

            let mut results = Vec::new();
            let mut all_success = true;
            let checkpoint_dir = "C:\\temp\\chain_checkpoints";
            let _ = std::fs::create_dir_all(checkpoint_dir);

            for (i, cmd) in commands.iter().enumerate() {
                let run_args = json!({
                    "session": session,
                    "command": cmd
                });
                let result = session_executor("session_run", &run_args);

                let success = result["success"].as_bool().unwrap_or(false);

                // Checkpoint after each command if enabled
                if checkpoint {
                    let checkpoint_file = format!("{}\\chain_step_{}.json", checkpoint_dir, i);
                    let checkpoint_data = json!({
                        "step": i,
                        "command": cmd,
                        "success": success,
                        "remaining": commands.len() - i - 1,
                        "remaining_commands": &commands[i+1..],
                        "timestamp": chrono::Local::now().to_rfc3339()
                    });
                    let _ = std::fs::write(
                        &checkpoint_file,
                        serde_json::to_string_pretty(&checkpoint_data).unwrap(),
                    );
                }

                results.push(json!({
                    "step": i + 1,
                    "command": cmd,
                    "success": success,
                    "output": result["output"]
                }));

                if !success {
                    all_success = false;
                    if stop_on_error {
                        break;
                    }
                }
            }

            json!({
                "success": all_success,
                "steps_completed": results.len(),
                "total_steps": commands.len(),
                "checkpoint_enabled": checkpoint,
                "results": results,
                "checkpoint_dir": if checkpoint { Some(checkpoint_dir) } else { None }
            })
        }

        _ => json!({"error": format!("Unknown shortcut tool: {}", name)}),
    }
}

/// Helper to run a shortcut's commands
fn run_shortcut_commands(
    shortcut_name: &str,
    commands: Vec<&str>,
    desc: &str,
    source: &str,
    session: &str,
    session_executor: fn(&str, &Value) -> Value,
) -> Value {
    let mut results = Vec::new();
    let mut all_success = true;

    for cmd in commands {
        let run_args = json!({
            "session": session,
            "command": cmd
        });
        let result = session_executor("session_run", &run_args);

        let success = result["success"].as_bool().unwrap_or(false);
        if !success {
            all_success = false;
        }

        results.push(json!({
            "command": cmd,
            "result": result
        }));

        if !success {
            break;
        }
    }

    json!({
        "shortcut": shortcut_name,
        "description": desc,
        "source": source,
        "success": all_success,
        "steps": results,
        "commands_run": results.len()
    })
}
