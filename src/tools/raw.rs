//! Raw tools - Zero metadata overhead execution
//! Ports from raw-tools Python server

use super::auto_backup;
use super::log;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Tool definitions for MCP
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "run",
            "description": "Execute command, return stdout only (no JSON wrapper)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute" }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "chain",
            "description": "Execute commands in sequence, stop on first failure",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Commands to execute in order"
                    }
                },
                "required": ["commands"]
            }
        }),
        json!({
            "name": "read_file",
            "description": "Read file with smart options: search for pattern, get specific lines, or auto-truncate large files. Returns content only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Full file path" },
                    "search": { "type": "string", "description": "Optional: grep for pattern, return only matching lines with context" },
                    "lines": { "type": "string", "description": "Optional: line range like '50:100' or '1:50' (1-indexed)" },
                    "max_kb": { "type": "integer", "description": "Optional: max KB to return (default 100KB, truncates with hint if larger)" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_file",
            "description": "Write file, return confirmation only",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Full file path" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "append_file",
            "description": "Append to file, return confirmation only",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Full file path" },
                    "content": { "type": "string", "description": "Content to append" }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "list_dir",
            "description": "List directory, return tree only",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" },
                    "depth": { "type": "integer", "default": 2, "description": "Depth to traverse" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "list_process",
            "description": "List processes, raw text",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "default": "", "description": "Filter by process name" }
                }
            }
        }),
        json!({
            "name": "kill_process",
            "description": "Kill process by PID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Process ID to kill" }
                },
                "required": ["pid"]
            }
        }),
        json!({
            "name": "get_env",
            "description": "Get environment variable(s)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "default": "", "description": "Specific key (empty for common vars)" }
                }
            }
        }),
        json!({
            "name": "clipboard_read",
            "description": "Read from Windows clipboard",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "clipboard_write",
            "description": "Write to Windows clipboard",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Content to copy" }
                },
                "required": ["content"]
            }
        }),
        // ============ ADDED: POWERSHELL, ARCHIVE, SEARCH, SYSINFO ============
        json!({
            "name": "powershell",
            "description": "Execute PowerShell command. Most versatile single tool for Windows.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "PowerShell command to execute" },
                    "timeout_secs": { "type": "integer", "description": "Timeout in seconds (default: 30)", "default": 30 }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "notify",
            "description": "Show a silent Windows toast notification that remains visible even if Claude Desktop is minimized.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Notification title" },
                    "body": { "type": "string", "description": "Notification body" },
                    "icon": {
                        "type": "string",
                        "enum": ["info", "warning", "error"],
                        "description": "Severity label shown in the notification title",
                        "default": "info"
                    },
                    "duration_ms": {
                        "type": "integer",
                        "description": "Requested toast duration in milliseconds (mapped to Windows short/long display time)",
                        "default": 5000
                    }
                },
                "required": ["title", "body"]
            }
        }),
        json!({
            "name": "archive_create",
            "description": "Create archive (zip). Uses PowerShell Compress-Archive.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Files/dirs to archive" },
                    "output": { "type": "string", "description": "Output archive path (.zip)" }
                },
                "required": ["paths", "output"]
            }
        }),
        json!({
            "name": "archive_extract",
            "description": "Extract archive (zip). Uses PowerShell Expand-Archive.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive_path": { "type": "string", "description": "Path to .zip archive" },
                    "destination": { "type": "string", "description": "Extraction destination dir" }
                },
                "required": ["archive_path"]
            }
        }),
        json!({
            "name": "search_file",
            "description": "Search for files by name or content.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory to search" },
                    "pattern": { "type": "string", "description": "Search pattern (regex)" },
                    "search_type": { "type": "string", "description": "files or content", "default": "files" }
                },
                "required": ["path", "pattern"]
            }
        }),
        json!({
            "name": "system_info",
            "description": "Get system info: OS, CPU, memory, disk.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({"name": "recovery_status", "description": "Check for recoverable sessions and pending checkpoints.", "inputSchema": {"type": "object", "properties": {}}}),
        json!({"name": "recovery_resume", "description": "Resume an interrupted operation from checkpoint.", "inputSchema": {"type": "object", "properties": {"checkpoint_id": {"type": "string", "description": "Checkpoint ID to resume"}}, "required": ["checkpoint_id"]}}),
        json!({"name": "recovery_clear", "description": "Clear all recovery data.", "inputSchema": {"type": "object", "properties": {}}}),
        json!({
            "name": "port_check",
            "description": "Test TCP connectivity to a host:port. Returns whether the port is open and connection time. Use for checking if MCP servers are listening.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "host": { "type": "string", "description": "Host to connect to (default: 127.0.0.1)", "default": "127.0.0.1" },
                    "port": { "type": "integer", "description": "Port number" },
                    "timeout_ms": { "type": "integer", "description": "Connection timeout in ms (default: 2000)", "default": 2000 }
                },
                "required": ["port"]
            }
        }),
        json!({
            "name": "tail_file",
            "description": "Return last N lines of a file plus current byte offset. Pass since_bytes from a previous call to get only NEW content (delta polling without streaming). Returns {lines, byte_offset, total_bytes, new_content}.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to tail" },
                    "lines": { "type": "integer", "description": "Number of lines to return (default: 50)", "default": 50 },
                    "since_bytes": { "type": "integer", "description": "Byte offset from previous call. 0 = read from end (default). Pass previous byte_offset to get only new content.", "default": 0 }
                },
                "required": ["path"]
            }
        }),
    ]
}

/// Execute raw tool
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "run" | "raw_run" => raw_run(args),
        "chain" | "raw_chain" => raw_chain(args),
        "read_file" | "read" | "raw_read" => raw_read(args),
        "write_file" | "write" | "raw_write" => raw_write(args),
        "append_file" | "append" | "raw_append" => raw_append(args),
        "list_dir" | "raw_list" => raw_list(args),
        "list_process" | "raw_ps" => raw_ps(args),
        "kill_process" | "raw_kill" => raw_kill(args),
        "get_env" | "raw_env" => raw_env(args),
        "clipboard_read" | "raw_clip_read" => raw_clip_read(args),
        "clipboard_write" | "raw_clip_write" => raw_clip_write(args),
        "powershell" => powershell(args),
        "notify" => notify(args),
        "archive_create" => archive_create(args),
        "archive_extract" => archive_extract(args),
        "search_file" | "search_files" => search_files(args),
        "system_info" => system_info(args),
        "recovery_status" => recovery_status(args),
        "recovery_resume" | "resume_operation" => resume_operation(args),
        "recovery_clear" | "clear_recovery" => clear_recovery(args),
        "port_check" => port_check(args),
        "tail_file" => tail_file(args),
        _ => json!({"error": format!("Unknown raw tool: {}", name)}),
    }
}

fn raw_run(args: &Value) -> Value {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");

    match Command::new("cmd")
        .args(["/C", command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let success = output.status.success();
            log::log_execution("raw_run", command, &stdout, &stderr, success);
            if success {
                json!(stdout.trim())
            } else {
                json!(format!("[ERROR] {}", stderr.trim()))
            }
        }
        Err(e) => {
            log::log_execution("raw_run", command, "", &e.to_string(), false);
            json!(format!("[ERROR] {}", e))
        }
    }
}

fn raw_chain(args: &Value) -> Value {
    let commands = match args.get("commands").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return json!("[ERROR] commands must be array"),
    };

    for (i, cmd) in commands.iter().enumerate() {
        let cmd_str = cmd.as_str().unwrap_or("");
        match Command::new("cmd")
            .args(["/C", cmd_str])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return json!(format!("[ERROR] Step {} failed: {}", i + 1, stderr.trim()));
                }
            }
            Err(e) => return json!(format!("[ERROR] Step {}: {}", i + 1, e)),
        }
    }

    json!("success")
}

fn raw_read(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    // Enforce sensitive path deny list
    if let Err(msg) = super::security::check_sensitive_path(path) {
        return json!(format!("[BLOCKED] {}", msg));
    }
    let search = args.get("search").and_then(|v| v.as_str());
    let lines_param = args.get("lines").and_then(|v| v.as_str());
    let max_kb = args.get("max_kb").and_then(|v| v.as_i64()).unwrap_or(100);

    // Check file exists
    let file_path = Path::new(path);
    if !file_path.exists() {
        return json!(format!("[ERROR] File not found: {}", path));
    }

    // Get file size
    let file_size = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return json!(format!("[ERROR] {}", e)),
    };
    let file_kb = file_size / 1024;

    // SEARCH MODE: grep for pattern
    if let Some(pattern) = search {
        return raw_read_search(path, pattern);
    }

    // LINES MODE: extract specific lines
    if let Some(range) = lines_param {
        return raw_read_lines(path, range);
    }

    // FULL READ with size check
    match fs::read_to_string(path) {
        Ok(content) => {
            if file_kb > max_kb as u64 {
                // Truncate large files
                let chars_limit = (max_kb * 1024) as usize;
                let truncated: String = content.chars().take(chars_limit).collect();
                let total_lines = content.lines().count();
                let shown_lines = truncated.lines().count();
                json!(format!(
                    "{}\n\n[TRUNCATED: {}KB file, showed {}/{} lines. Use search='pattern' or lines='start:end' for specific content]",
                    truncated, file_kb, shown_lines, total_lines
                ))
            } else {
                json!(content)
            }
        }
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_read_search(path: &str, pattern: &str) -> Value {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return json!(format!("[ERROR] {}", e)),
    };

    let reader = BufReader::new(file);
    let pattern_lower = pattern.to_lowercase();
    let mut matches: Vec<String> = Vec::new();
    let mut total_lines = 0;

    for (i, line) in reader.lines().enumerate() {
        total_lines = i + 1;
        if let Ok(text) = line {
            if text.to_lowercase().contains(&pattern_lower) {
                matches.push(format!("{}:{}", i + 1, text));
            }
        }
        // Limit matches to prevent huge output
        if matches.len() >= 100 {
            matches.push("[...truncated at 100 matches]".to_string());
            break;
        }
    }

    if matches.is_empty() {
        json!(format!(
            "[NO MATCHES] '{}' not found in {} lines",
            pattern, total_lines
        ))
    } else {
        json!(format!(
            "[{} matches in {} lines]\n{}",
            matches.len(),
            total_lines,
            matches.join("\n")
        ))
    }
}

fn raw_read_lines(path: &str, range: &str) -> Value {
    // Parse range like "50:100" or "1:50"
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return json!("[ERROR] lines format: 'start:end' e.g. '50:100'");
    }

    let start: usize = parts[0].parse().unwrap_or(1);
    let end: usize = parts[1].parse().unwrap_or(50);

    if start < 1 || end < start {
        return json!("[ERROR] Invalid line range");
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return json!(format!("[ERROR] {}", e)),
    };

    let reader = BufReader::new(file);
    let mut result: Vec<String> = Vec::new();
    let mut total_lines = 0;

    for (i, line) in reader.lines().enumerate() {
        let line_num = i + 1;
        total_lines = line_num;

        if line_num >= start && line_num <= end {
            if let Ok(text) = line {
                result.push(format!("{}:{}", line_num, text));
            }
        }

        if line_num > end {
            break;
        }
    }

    json!(format!(
        "[Lines {}-{} of {}]\n{}",
        start,
        end.min(total_lines),
        total_lines,
        result.join("\n")
    ))
}

fn raw_write(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    // Enforce sensitive path deny list
    if let Err(msg) = super::security::check_sensitive_path(path) {
        return json!(format!("[BLOCKED] {}", msg));
    }
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

    // Auto-backup if file exists
    auto_backup::backup_if_exists(path);

    // Create parent directories
    if let Some(parent) = Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(path, content) {
        Ok(_) => json!(format!("written: {}", path)),
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_append(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    // Enforce sensitive path deny list
    if let Err(msg) = super::security::check_sensitive_path(path) {
        return json!(format!("[BLOCKED] {}", msg));
    }
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

    // Auto-backup if file exists
    auto_backup::backup_if_exists(path);

    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(_) => json!(format!("appended: {}", path)),
            Err(e) => json!(format!("[ERROR] {}", e)),
        },
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_list(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let depth = args.get("depth").and_then(|v| v.as_i64()).unwrap_or(2) as usize;

    let mut output = Vec::new();
    list_recursive(Path::new(path), depth, 0, &mut output);

    json!(output.join("\n"))
}

fn list_recursive(
    base: &Path,
    max_depth: usize,
    current_depth: usize,
    output: &mut Vec<String>,
) {
    if current_depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| e.file_name());

    for entry in items.iter().take(100) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let prefix = "  ".repeat(current_depth);

        if path.is_dir() {
            output.push(format!("{}{}/", prefix, name));
            if current_depth < max_depth {
                list_recursive(&path, max_depth, current_depth + 1, output);
            }
        } else {
            output.push(format!("{}{}", prefix, name));
        }
    }
}

fn raw_ps(args: &Value) -> Value {
    let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("");

    let ps_cmd = if filter.is_empty() {
        "Get-Process | Select-Object ProcessName, Id | Format-Table -Auto".to_string()
    } else {
        format!("Get-Process | Where-Object {{ $_.ProcessName -like '*{}*' }} | Select-Object ProcessName, Id | Format-Table -Auto", filter)
    };

    match Command::new("powershell")
        .args(["-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            json!(stdout.trim())
        }
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_kill(args: &Value) -> Value {
    let pid = args.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);

    match Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                json!(format!("killed: {}", pid))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                json!(format!("[ERROR] {}", stderr.trim()))
            }
        }
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_env(args: &Value) -> Value {
    let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");

    if !key.is_empty() {
        match std::env::var(key) {
            Ok(val) => json!(val),
            Err(_) => json!(format!("[ERROR] {} not set", key)),
        }
    } else {
        let useful = ["PATH", "USERPROFILE", "APPDATA", "TEMP", "COMPUTERNAME"];
        let values: Vec<String> = useful
            .iter()
            .map(|k| format!("{}={}", k, std::env::var(k).unwrap_or_default()))
            .collect();
        json!(values.join("\n"))
    }
}

fn raw_clip_read(_args: &Value) -> Value {
    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
        Ok(text) => json!(text),
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

fn raw_clip_write(args: &Value) -> Value {
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(content.to_string())) {
        Ok(()) => json!(format!("copied: {} chars", content.len())),
        Err(e) => json!(format!("[ERROR] {}", e)),
    }
}

// ============ ADDED: POWERSHELL, ARCHIVE, SEARCH, SYSINFO ============

fn powershell(args: &Value) -> Value {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let _timeout = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            log::log_execution(
                "powershell",
                command,
                &stdout,
                &stderr,
                output.status.success(),
            );
            json!({
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": stdout.trim(),
                "stderr": stderr.trim(),
                "success": output.status.success()
            })
        }
        Err(e) => {
            log::log_execution("powershell", command, "", &e.to_string(), false);
            json!({"error": format!("{}", e)})
        }
    }
}

fn notify(args: &Value) -> Value {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let icon = args.get("icon").and_then(|v| v.as_str()).unwrap_or("info");
    let duration_ms = args
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(5000)
        .max(1);

    if title.is_empty() || body.is_empty() {
        return json!({"error": "Both title and body are required"});
    }
    if !matches!(icon, "info" | "warning" | "error") {
        return json!({"error": "icon must be one of: info, warning, error"});
    }

    let display_title = match icon {
        "warning" => format!("[Warning] {title}"),
        "error" => format!("[Error] {title}"),
        _ => format!("[Info] {title}"),
    };
    let toast_duration = if duration_ms > 7_000 { "long" } else { "short" };
    // Try BurntToast first when available, otherwise fall back to native WinRT toast APIs.
    let script = r#"
$ErrorActionPreference = 'Stop'
$toastDuration = if ([int]$env:MCP_NOTIFY_DURATION_MS -gt 7000) { 'long' } else { 'short' }
if (Get-Command New-BurntToastNotification -ErrorAction SilentlyContinue) {
    New-BurntToastNotification -Text $env:MCP_NOTIFY_TITLE, $env:MCP_NOTIFY_BODY -Silent | Out-Null
    Write-Output 'burnttoast'
    return
}
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null
$titleEscaped = [System.Security.SecurityElement]::Escape($env:MCP_NOTIFY_TITLE)
$bodyEscaped = [System.Security.SecurityElement]::Escape($env:MCP_NOTIFY_BODY)
$xml = @"
<toast duration="$toastDuration">
  <visual>
    <binding template="ToastGeneric">
      <text>$titleEscaped</text>
      <text>$bodyEscaped</text>
    </binding>
  </visual>
  <audio silent="true"/>
</toast>
"@
$doc = [Windows.Data.Xml.Dom.XmlDocument]::new()
$doc.LoadXml($xml)
$toast = [Windows.UI.Notifications.ToastNotification]::new($doc)
$appId = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
try {
    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($appId).Show($toast)
} catch {
    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier().Show($toast)
}
Write-Output 'winrt'
"#;

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .env("MCP_NOTIFY_TITLE", &display_title)
        .env("MCP_NOTIFY_BODY", body)
        .env("MCP_NOTIFY_DURATION_MS", duration_ms.to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let backend = stdout.lines().last().unwrap_or("powershell").trim();
            let success = output.status.success();
            let command_summary = format!("icon={icon}; duration_ms={duration_ms}; title={title}");
            log::log_execution("notify", &command_summary, &stdout, &stderr, success);

            if success {
                json!({
                    "success": true,
                    "backend": backend,
                    "title": display_title,
                    "body": body,
                    "icon": icon,
                    "duration_ms": duration_ms,
                    "toast_duration": toast_duration,
                    "silent": true
                })
            } else {
                json!({
                    "error": stderr.trim(),
                    "stdout": stdout.trim()
                })
            }
        }
        Err(e) => {
            let command_summary = format!("icon={icon}; duration_ms={duration_ms}; title={title}");
            log::log_execution("notify", &command_summary, "", &e.to_string(), false);
            json!({"error": format!("{}", e)})
        }
    }
}

fn archive_create(args: &Value) -> Value {
    let paths: Vec<&str> = args
        .get("paths")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let output = args
        .get("output")
        .and_then(|v| v.as_str())
        .unwrap_or("archive.zip");

    if paths.is_empty() {
        return json!({"error": "No paths provided"});
    }

    // Build PowerShell Compress-Archive command
    let paths_str = paths
        .iter()
        .map(|p| format!("'{}'", p))
        .collect::<Vec<_>>()
        .join(",");
    let cmd = format!(
        "Compress-Archive -Path {} -DestinationPath '{}' -Force",
        paths_str, output
    );

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", &cmd])
        .output()
    {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                json!({"success": true, "output": output, "paths_archived": paths.len()})
            } else {
                json!({"error": stderr.trim()})
            }
        }
        Err(e) => json!({"error": format!("{}", e)}),
    }
}

fn archive_extract(args: &Value) -> Value {
    let archive = args
        .get("archive_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let dest = args
        .get("destination")
        .and_then(|v| v.as_str())
        .unwrap_or(".");

    if archive.is_empty() {
        return json!({"error": "No archive_path provided"});
    }

    let cmd = format!(
        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
        archive, dest
    );

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", &cmd])
        .output()
    {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                json!({"success": true, "archive": archive, "destination": dest})
            } else {
                json!({"error": stderr.trim()})
            }
        }
        Err(e) => json!({"error": format!("{}", e)}),
    }
}

fn search_files(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let search_type = args
        .get("search_type")
        .and_then(|v| v.as_str())
        .unwrap_or("files");

    let cmd = if search_type == "content" {
        format!("Get-ChildItem -Path '{}' -Recurse -File | Select-String -Pattern '{}' | Select-Object -First 50 Path,LineNumber,Line | Format-Table -AutoSize", path, pattern)
    } else {
        format!("Get-ChildItem -Path '{}' -Recurse -Filter '*{}*' | Select-Object -First 50 FullName,Length,LastWriteTime | Format-Table -AutoSize", path, pattern)
    };

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", &cmd])
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            json!({"results": stdout.trim()})
        }
        Err(e) => json!({"error": format!("{}", e)}),
    }
}

fn system_info(_args: &Value) -> Value {
    let cmd = r#"
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$mem = $os
$disk = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3" | Select-Object DeviceID,@{N='SizeGB';E={[math]::Round($_.Size/1GB,1)}},@{N='FreeGB';E={[math]::Round($_.FreeSpace/1GB,1)}}
[PSCustomObject]@{
    OS = "$($os.Caption) $($os.Version)"
    CPU = $cpu.Name
    Cores = $cpu.NumberOfCores
    TotalMemoryGB = [math]::Round($mem.TotalVisibleMemorySize/1MB,1)
    FreeMemoryGB = [math]::Round($mem.FreePhysicalMemory/1MB,1)
    Disks = ($disk | ForEach-Object { "$($_.DeviceID) $($_.FreeGB)/$($_.SizeGB)GB" }) -join "; "
} | ConvertTo-Json
"#;

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", cmd])
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            serde_json::from_str(stdout.trim()).unwrap_or(json!({"raw": stdout.trim()}))
        }
        Err(e) => json!({"error": format!("{}", e)}),
    }
}

const RECOVERY_FILE: &str = "C:\\temp\\mcp_recovery.json";

fn load_recovery() -> Value {
    match std::fs::read_to_string(RECOVERY_FILE) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(json!({"sessions": [], "checkpoints": []}))
        }
        Err(_) => json!({"sessions": [], "checkpoints": []}),
    }
}

fn save_recovery(data: &Value) {
    let _ = std::fs::create_dir_all("C:\\temp");
    let _ = std::fs::write(
        RECOVERY_FILE,
        serde_json::to_string_pretty(data).unwrap_or_default(),
    );
}

fn recovery_status(_args: &Value) -> Value {
    let data = load_recovery();
    json!({
        "recoverable_sessions": data["sessions"].as_array().map(|a| a.len()).unwrap_or(0),
        "pending_checkpoints": data["checkpoints"].as_array().map(|a| a.len()).unwrap_or(0),
        "data": data
    })
}

fn resume_operation(args: &Value) -> Value {
    let checkpoint_id = args["checkpoint_id"].as_str().unwrap_or("");
    let data = load_recovery();

    if let Some(checkpoints) = data["checkpoints"].as_array() {
        for cp in checkpoints {
            if cp["checkpoint_id"].as_str() == Some(checkpoint_id) {
                return json!({"success": true, "checkpoint": cp.clone()});
            }
        }
    }
    json!({"success": false, "error": format!("Checkpoint {} not found", checkpoint_id)})
}

fn clear_recovery(_args: &Value) -> Value {
    save_recovery(&json!({"sessions": [], "checkpoints": []}));
    json!({"success": true, "message": "Recovery data cleared"})
}

fn port_check(args: &Value) -> Value {
    let host = args
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1");
    let port = match args.get("port").and_then(|v| v.as_u64()) {
        Some(p) => p as u16,
        None => return json!({"error": "port required"}),
    };
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(2000);

    let addr = format!("{}:{}", host, port);
    let socket_addr: std::net::SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => return json!({"error": format!("Invalid address {}: {}", addr, e)}),
    };

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    match std::net::TcpStream::connect_timeout(&socket_addr, timeout) {
        Ok(_) => {
            let elapsed_ms = start.elapsed().as_millis();
            json!({
                "open": true,
                "host": host,
                "port": port,
                "connect_time_ms": elapsed_ms,
            })
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis();
            json!({
                "open": false,
                "host": host,
                "port": port,
                "error": e.to_string(),
                "elapsed_ms": elapsed_ms,
            })
        }
    }
}

fn tail_file(args: &Value) -> Value {
    use std::io::{Read, Seek, SeekFrom};

    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' parameter"}),
    };
    let max_lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let since_bytes = args
        .get("since_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return json!({"error": format!("Cannot open file: {}", e)}),
    };

    let total_bytes = match file.metadata() {
        Ok(m) => m.len(),
        Err(e) => return json!({"error": format!("Cannot read metadata: {}", e)}),
    };

    // If since_bytes > 0, read only new content from that offset
    if since_bytes > 0 {
        if since_bytes >= total_bytes {
            return json!({
                "lines": [],
                "byte_offset": total_bytes,
                "total_bytes": total_bytes,
                "new_content": false
            });
        }
        if let Err(e) = file.seek(SeekFrom::Start(since_bytes)) {
            return json!({"error": format!("Seek failed: {}", e)});
        }
        let mut new_data = String::new();
        if let Err(e) = file.read_to_string(&mut new_data) {
            return json!({"error": format!("Read failed: {}", e)});
        }
        let lines: Vec<&str> = new_data.lines().collect();
        let tail: Vec<&str> = if lines.len() > max_lines {
            lines[lines.len() - max_lines..].to_vec()
        } else {
            lines
        };
        return json!({
            "lines": tail,
            "byte_offset": total_bytes,
            "total_bytes": total_bytes,
            "new_content": true
        });
    }

    // since_bytes == 0: read last N lines from end
    // Read up to 64KB from end to find enough lines
    let read_size: u64 = (64 * 1024).min(total_bytes);
    let start_pos = total_bytes.saturating_sub(read_size);
    if let Err(e) = file.seek(SeekFrom::Start(start_pos)) {
        return json!({"error": format!("Seek failed: {}", e)});
    }
    let mut buf = String::new();
    if let Err(e) = file.read_to_string(&mut buf) {
        return json!({"error": format!("Read failed: {}", e)});
    }
    let lines: Vec<&str> = buf.lines().collect();
    let tail: Vec<&str> = if lines.len() > max_lines {
        lines[lines.len() - max_lines..].to_vec()
    } else {
        lines
    };
    json!({
        "lines": tail,
        "byte_offset": total_bytes,
        "total_bytes": total_bytes,
        "new_content": true
    })
}
