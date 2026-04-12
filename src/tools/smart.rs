//! Smart execution routing - Auto-picks best tool for the job
//! Now with auto-retry on known error patterns
//! Reads/writes to Volumes/logs/error_fallbacks.json
// NAV: TOC at line 323 | 11 fn |  struct | 2026-03-04

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use super::{raw, session, transforms};

const FALLBACKS_PATH: &str = r"C:\My Drive\Volumes\logs\error_fallbacks.json";
const ERROR_LOG_PATH: &str = r"C:\My Drive\Volumes\logs\error_patterns.jsonl";

#[derive(Serialize, Deserialize, Clone, Default)]
struct ErrorPattern {
    trigger: String,
    symptom: String,
    fallback: String,
    success_rate: f64,
    occurrences: u32,
}

fn load_fallbacks() -> HashMap<String, ErrorPattern> {
    if let Ok(content) = fs::read_to_string(FALLBACKS_PATH) {
        if let Ok(patterns) = serde_json::from_str(&content) {
            return patterns;
        }
    }
    // Default patterns
    let mut patterns = HashMap::new();
    patterns.insert("path_spaces_cmd".into(), ErrorPattern {
        trigger: "raw_run".into(),
        symptom: "syntax is incorrect".into(),
        fallback: "powershell".into(),
        success_rate: 0.95,
        occurrences: 0,
    });
    patterns.insert("timeout_raw".into(), ErrorPattern {
        trigger: "raw_run".into(),
        symptom: "timeout".into(),
        fallback: "powershell".into(),
        success_rate: 0.80,
        occurrences: 0,
    });
    patterns.insert("cargo_pipe_fail".into(), ErrorPattern {
        trigger: "powershell".into(),
        symptom: "cargo".into(),
        fallback: "raw_run_bat".into(),
        success_rate: 1.0,
        occurrences: 0,
    });
    patterns
}

fn save_fallbacks(patterns: &HashMap<String, ErrorPattern>) {
    if let Ok(content) = serde_json::to_string_pretty(&patterns) {
        let _ = fs::write(FALLBACKS_PATH, content);
    }
}

fn log_error_attempt(tool: &str, error: &str, fallback: Option<&str>, success: Option<bool>) {
    let entry = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "tool": tool,
        "error_message": error,
        "fallback_tried": fallback,
        "fallback_success": success
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(ERROR_LOG_PATH) {
        let _ = writeln!(file, "{}", entry);
    }
}

fn find_fallback(route: &str, error_msg: &str, patterns: &HashMap<String, ErrorPattern>) -> Option<(String, String)> {
    let error_lower = error_msg.to_lowercase();
    for (id, pattern) in patterns {
        if pattern.trigger.to_lowercase() == route.to_lowercase() 
            && error_lower.contains(&pattern.symptom.to_lowercase()) 
        {
            return Some((id.clone(), pattern.fallback.clone()));
        }
    }
    None
}

fn is_error_result(result: &Value) -> Option<String> {
    // Check various error indicators
    if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        return Some(err.to_string());
    }
    if result.get("success") == Some(&json!(false)) {
        if let Some(stderr) = result.get("stderr").and_then(|v| v.as_str()) {
            if !stderr.is_empty() {
                return Some(stderr.to_string());
            }
        }
    }
    // Check for [ERROR] prefix in string result
    if let Some(s) = result.as_str() {
        if s.starts_with("[ERROR]") {
            return Some(s.to_string());
        }
    }
    None
}

fn update_pattern_stats(patterns: &mut HashMap<String, ErrorPattern>, pattern_id: &str, success: bool) {
    if let Some(pattern) = patterns.get_mut(pattern_id) {
        pattern.occurrences += 1;
        let new_rate = if success { 1.0 } else { 0.0 };
        pattern.success_rate = (pattern.success_rate * (pattern.occurrences - 1) as f64 + new_rate) 
            / pattern.occurrences as f64;
    }
}

/// Tool definitions
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "smart_exec",
            "description": "Auto-routing command execution. Analyzes command and routes to raw_run (simple), session_run (needs env/cwd), or powershell (PS syntax). Returns which route was used.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute" },
                    "cwd": { "type": "string", "description": "Working directory (triggers session mode)" },
                    "needs_env": { "type": "boolean", "default": false, "description": "If true, uses persistent session" }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "smart_read",
            "description": "Auto-routing file read. For Volumes Operating_*.md files: returns TOC index by default, or targeted ±3 line section reads via 'section' param. For other files: routes to raw_read, transform_grep, transform_extract_lines, or transform_diff_files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to read" },
                    "section": { "type": "string", "description": "Section name or keyword to read from Operating file TOC (e.g. 'Liberation Protocol', 'trading')" },
                    "find": { "type": "string", "description": "Search for pattern (uses grep)" },
                    "lines": { "type": "string", "description": "Line range like '50:100'" },
                    "compare_to": { "type": "string", "description": "Compare with another file (returns diff)" }
                },
                "required": ["path"]
            }
        })
    ]
}

/// Execute smart tool
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "smart_exec" => smart_exec(args),
        "smart_read" => smart_read(args),
        _ => json!({"error": format!("Unknown smart tool: {}", name)})
    }
}

fn execute_route(route: &str, command: &str, cwd: Option<&str>) -> Value {
    match route {
        "session_run" => {
            if let Some(dir) = cwd {
                let _ = session::execute("session_create", &json!({"name": "default", "cwd": dir}));
            } else {
                let _ = session::execute("session_create", &json!({"name": "default"}));
            }
            session::execute("session_run", &json!({"session": "default", "command": command}))
        },
        "powershell" => raw::execute("powershell", &json!({"command": command})),
        "raw_run" | _ => raw::execute("raw_run", &json!({"command": command})),
    }
}

fn smart_exec(args: &Value) -> Value {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let cwd = args.get("cwd").and_then(|v| v.as_str());
    let needs_env = args.get("needs_env").and_then(|v| v.as_bool()).unwrap_or(false);
    
    // Detect PowerShell syntax
    let needs_powershell = command.contains("$") 
        || command.contains("Get-")
        || command.contains("Set-")
        || command.contains("New-Item")
        || command.contains("Remove-Item")
        || command.contains("Where-Object")
        || command.contains("-ErrorAction")
        || command.contains("Select-Object")
        || command.contains("Format-Table");
    
    // Detect session needs
    let needs_session = needs_env
        || cwd.is_some()
        || command.contains("cargo ")
        || command.contains("npm ")
        || command.contains("pip ")
        || command.starts_with("cd ")
        || command.contains(" && cd ")
        || command.starts_with("set ")
        || command.starts_with("export ");
    
    // Select initial route
    let route = if needs_session {
        "session_run"
    } else if needs_powershell {
        "powershell"
    } else {
        "raw_run"
    };
    
    // First attempt
    let result = execute_route(route, command, cwd);
    
    // Check for error and try fallback
    if let Some(error_msg) = is_error_result(&result) {
        let mut patterns = load_fallbacks();
        
        if let Some((pattern_id, fallback_route)) = find_fallback(route, &error_msg, &patterns) {
            // Log first attempt failure
            log_error_attempt(route, &error_msg, Some(&fallback_route), None);
            
            // Try fallback
            let fallback_result = execute_route(&fallback_route, command, cwd);
            
            let fallback_success = is_error_result(&fallback_result).is_none();
            
            // Update stats
            update_pattern_stats(&mut patterns, &pattern_id, fallback_success);
            save_fallbacks(&patterns);
            
            // Log fallback result
            log_error_attempt(&fallback_route, 
                &is_error_result(&fallback_result).unwrap_or_default(), 
                None, 
                Some(fallback_success));
            
            return json!({
                "routed_to": route,
                "fallback_used": fallback_route,
                "fallback_reason": error_msg,
                "result": fallback_result
            });
        }
        
        // No fallback available, log and return original error
        log_error_attempt(route, &error_msg, None, None);
    }
    
    json!({
        "routed_to": route,
        "result": result
    })
}

fn smart_read(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let section = args.get("section").and_then(|v| v.as_str());
    let find = args.get("find").and_then(|v| v.as_str());
    let lines = args.get("lines").and_then(|v| v.as_str());
    let compare_to = args.get("compare_to").and_then(|v| v.as_str());
    
    // TOC-aware routing for Operating files
    if super::toc::is_operating_file(path) {
        // Section param → targeted read
        if section.is_some() {
            return super::toc::toc_read(path, section);
        }
        // No specific params → return TOC index instead of full file
        if find.is_none() && lines.is_none() && compare_to.is_none() {
            let toc_result = super::toc::toc_read(path, None);
            // If TOC available, return it; otherwise fall through to raw_read
            if toc_result.get("toc_available") != Some(&json!(false)) {
                return toc_result;
            }
        }
    }
    
    let route: &str;
    let result: Value;
    
    if let Some(pattern) = find {
        route = "transform_grep";
        result = transforms::execute("transform_grep", &json!({
            "path": path,
            "pattern": pattern,
            "context": 2
        }));
    } else if let Some(range) = lines {
        route = "transform_extract_lines";
        let parts: Vec<&str> = range.split(':').collect();
        if parts.len() == 2 {
            let start: i64 = parts[0].parse().unwrap_or(1);
            let end: i64 = parts[1].parse().unwrap_or(-1);
            result = transforms::execute("transform_extract_lines", &json!({
                "path": path,
                "start": start,
                "end": end
            }));
        } else {
            return json!({"error": "lines format: 'start:end' e.g. '50:100'"});
        }
    } else if let Some(other) = compare_to {
        route = "transform_diff_files";
        result = transforms::execute("transform_diff_files", &json!({
            "file_a": path,
            "file_b": other
        }));
    } else {
        route = "raw_read";
        result = raw::execute("raw_read", &json!({
            "path": path,
            "max_kb": 50
        }));
    }
    
    json!({
        "routed_to": route,
        "result": result
    })
}

// === FILE NAVIGATION ===
// Generated: 2026-03-04T17:12:34
// Total: 320 lines | 11 functions |  structs | 2 constants
//
// IMPORTS: serde, serde_json, std, super
//
// CONSTANTS:
//   const FALLBACKS_PATH: 12
//   const ERROR_LOG_PATH: 13
//
// FUNCTIONS:
//   load_fallbacks: 24-54
//   save_fallbacks: 56-60
//   log_error_attempt: 62-73
//   find_fallback: 75-85
//   is_error_result: 87-106
//   update_pattern_stats: 108-115
//   pub +get_definitions: 118-149
//   pub +execute: 152-158
//   execute_route: 160-173
//   smart_exec: 175-253 [med]
//   smart_read: 255-320 [med]
//
// === END FILE NAVIGATION ===