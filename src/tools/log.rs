//! Terminal execution logging
//! Logs all command executions to C:\My Drive\Volumes\logs\terminal_log.jsonl

use serde_json::{json, Value};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;
use chrono::Local;

const LOG_PATH: &str = r"C:\My Drive\Volumes\logs\terminal_log.jsonl";

/// Log a command execution
pub fn log_execution(tool: &str, command: &str, stdout: &str, stderr: &str, success: bool) {
    // Ensure log directory exists
    if let Some(parent) = Path::new(LOG_PATH).parent() {
        let _ = create_dir_all(parent);
    }
    
    let entry = json!({
        "timestamp": Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        "tool": tool,
        "command": truncate_for_log(command, 500),
        "stdout": truncate_for_log(stdout, 2000),
        "stderr": truncate_for_log(stderr, 1000),
        "success": success
    });
    
    // Append to log file
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        let _ = writeln!(file, "{}", entry.to_string());
    }
}

/// Truncate long strings for logging
fn truncate_for_log(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

/// Get recent log entries
#[allow(dead_code)] // Utility for future log browsing tool
pub fn get_recent_logs(count: usize) -> Value {
    match std::fs::read_to_string(LOG_PATH) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().rev().take(count).collect();
            let entries: Vec<Value> = lines.iter()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            json!(entries)
        }
        Err(_) => json!([])
    }
}
