//! WSL tools - Run commands and builds in WSL with managed output
//! Background jobs stay alive because local.exe holds the child process handle

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static JOBS: Mutex<Option<HashMap<String, WslJob>>> = Mutex::new(None);

struct WslJob {
    pid: u32,
    log_file: String,
    status_file: String,
    started: Instant,
}

fn get_jobs() -> std::sync::MutexGuard<'static, Option<HashMap<String, WslJob>>> {
    let mut guard = JOBS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn gen_job_id() -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    format!("wsl_{}", ts)
}

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "wsl_run",
            "description": "Run command in WSL, output to file, return JSON summary. For scripts: write to C:\\temp, pass path. Handles CRLF automatically.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to run (e.g., 'ls -la /home/joe' or 'bash /mnt/c/temp/task.sh')" },
                    "timeout_secs": { "type": "integer", "description": "Timeout in seconds (default: 120)" }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "wsl_bg",
            "description": "Launch command in WSL background. Returns job_id immediately. Process stays alive because local.exe holds the handle. Poll with wsl_status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to run in background" },
                    "job_name": { "type": "string", "description": "Optional friendly name (default: auto-generated)" }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "wsl_status",
            "description": "Check status of a background WSL job. Returns running/done/failed + log tail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Job ID from wsl_bg (or 'all' to list all jobs)" },
                    "tail": { "type": "integer", "description": "Number of log lines to return (default: 10)" }
                },
                "required": ["job_id"]
            }
        }),
        json!({
            "name": "wsl_log",
            "description": "Get full or partial log from a WSL job.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Job ID from wsl_bg" },
                    "lines": { "type": "string", "description": "Line range like '1:50' or 'last:20' (default: last 50)" }
                },
                "required": ["job_id"]
            }
        }),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "wsl_run" => wsl_run(args),
        "wsl_bg" => wsl_bg(args),
        "wsl_status" => wsl_status(args),
        "wsl_log" => wsl_log(args),
        _ => json!({"error": format!("Unknown WSL tool: {}", name)}),
    }
}

fn wsl_run(args: &Value) -> Value {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "command required"}),
    };
    let timeout = args.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(120);
    
    let log_path = format!("C:\\temp\\wsl_run_{}.log",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    
    let start = Instant::now();
    
    // Run via wsl -- bash -c "..." with output to log file
    let result = Command::new("wsl")
        .args(["-d", "Ubuntu-24.04", "--", "bash", "-c", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    let duration = start.elapsed().as_secs();
    
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}{}", stdout, stderr);
            
            // Write full output to log
            let _ = fs::write(&log_path, &combined);
            
            let lines: Vec<&str> = combined.lines().collect();
            let line_count = lines.len();
            
            // Return last 15 lines as summary
            let tail: Vec<&str> = lines.iter().rev().take(15).rev().cloned().collect();
            
            json!({
                "status": if output.status.success() { "success" } else { "failed" },
                "exit_code": output.status.code().unwrap_or(-1),
                "duration_secs": duration,
                "total_lines": line_count,
                "log": log_path,
                "tail": tail.join("\n")
            })
        }
        Err(e) => json!({"error": format!("WSL launch failed: {}", e)}),
    }
}

fn wsl_bg(args: &Value) -> Value {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "command required"}),
    };
    
    let job_id = args.get("job_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(gen_job_id);
    
    let log_file = format!("C:\\temp\\wsl_bg_{}.log", &job_id);
    let status_file = format!("C:\\temp\\wsl_bg_{}.status", &job_id);
    
    // Write initial status
    let _ = fs::write(&status_file, r#"{"status":"running"}"#);
    
    // Open log file for stdout/stderr redirect
    let log_handle = match fs::File::create(&log_file) {
        Ok(f) => f,
        Err(e) => return json!({"error": format!("Can't create log: {}", e)}),
    };
    let err_handle = match log_handle.try_clone() {
        Ok(f) => f,
        Err(e) => return json!({"error": format!("Can't clone handle: {}", e)}),
    };
    
    // Spawn WSL process - local.exe holds the child handle, keeping it alive
    let child = Command::new("wsl")
        .args(["-d", "Ubuntu-24.04", "--", "bash", "-c", command])
        .stdout(log_handle)
        .stderr(err_handle)
        .spawn();
    
    match child {
        Ok(child) => {
            let pid = child.id();
            
            // Store job for tracking
            let mut jobs = get_jobs();
            if let Some(ref mut map) = *jobs {
                map.insert(job_id.clone(), WslJob {
                    pid,
                    log_file: log_file.clone(),
                    status_file: status_file.clone(),
                    started: Instant::now(),
                });
            }
            
            // Spawn a thread to wait for completion and update status
            let sf = status_file.clone();
            let jid = job_id.clone();
            std::thread::spawn(move || {
                let mut child = child;
                let result = child.wait();
                let exit = match result {
                    Ok(status) => status.code().unwrap_or(-1),
                    Err(_) => -1,
                };
                let status = if exit == 0 { "done" } else { "failed" };
                let _ = fs::write(&sf, format!(r#"{{"status":"{}","exit_code":{}}}"#, status, exit));
                
                // Clean up job tracking
                if let Ok(mut jobs) = JOBS.lock() {
                    if let Some(ref mut map) = *jobs {
                        map.remove(&jid);
                    }
                }
            });
            
            json!({
                "job_id": job_id,
                "pid": pid,
                "log": log_file,
                "status_file": status_file,
                "poll": format!("wsl_status(job_id='{}')", job_id)
            })
        }
        Err(e) => {
            let _ = fs::write(&status_file, format!(r#"{{"status":"failed","error":"{}"}}"#, e));
            json!({"error": format!("WSL spawn failed: {}", e)})
        }
    }
}

fn wsl_status(args: &Value) -> Value {
    let job_id = match args.get("job_id").and_then(|v| v.as_str()) {
        Some(j) => j,
        None => return json!({"error": "job_id required"}),
    };
    let tail_n = args.get("tail").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    
    // List all jobs
    if job_id == "all" {
        let jobs = get_jobs();
        let mut result = Vec::new();
        if let Some(ref map) = *jobs {
            for (id, job) in map.iter() {
                let status = fs::read_to_string(&job.status_file).unwrap_or_default();
                result.push(json!({
                    "job_id": id,
                    "pid": job.pid,
                    "elapsed_secs": job.started.elapsed().as_secs(),
                    "status": status,
                }));
            }
        }
        // Also check for completed jobs via status files
        if let Ok(entries) = fs::read_dir("C:\\temp") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("wsl_bg_") && name.ends_with(".status") {
                    let jid = name.trim_start_matches("wsl_bg_").trim_end_matches(".status");
                    if !result.iter().any(|r| r.get("job_id").and_then(|v| v.as_str()) == Some(jid)) {
                        let status = fs::read_to_string(entry.path()).unwrap_or_default();
                        result.push(json!({
                            "job_id": jid,
                            "status": status,
                            "completed": true,
                        }));
                    }
                }
            }
        }
        return json!({"jobs": result});
    }
    
    // Single job
    let status_file = format!("C:\\temp\\wsl_bg_{}.status", job_id);
    let log_file = format!("C:\\temp\\wsl_bg_{}.log", job_id);
    
    let status = fs::read_to_string(&status_file).unwrap_or_else(|_| r#"{"error":"job not found"}"#.to_string());
    
    // Tail the log
    let tail = if Path::new(&log_file).exists() {
        let content = fs::read_to_string(&log_file).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        let start = if lines.len() > tail_n { lines.len() - tail_n } else { 0 };
        lines[start..].join("\n")
    } else {
        String::new()
    };
    
    let total_lines = if Path::new(&log_file).exists() {
        fs::read_to_string(&log_file).unwrap_or_default().lines().count()
    } else {
        0
    };
    
    json!({
        "job_id": job_id,
        "status": serde_json::from_str::<Value>(&status).unwrap_or(json!(status)),
        "total_lines": total_lines,
        "tail": tail,
    })
}

fn wsl_log(args: &Value) -> Value {
    let job_id = match args.get("job_id").and_then(|v| v.as_str()) {
        Some(j) => j,
        None => return json!({"error": "job_id required"}),
    };
    
    let log_file = format!("C:\\temp\\wsl_bg_{}.log", job_id);
    if !Path::new(&log_file).exists() {
        return json!({"error": format!("Log not found: {}", log_file)});
    }
    
    let content = match fs::read_to_string(&log_file) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Read failed: {}", e)}),
    };
    
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    
    let range = args.get("lines").and_then(|v| v.as_str()).unwrap_or("last:50");
    
    let (start, end) = if range.starts_with("last:") {
        let n: usize = range.trim_start_matches("last:").parse().unwrap_or(50);
        let s = if total > n { total - n } else { 0 };
        (s, total)
    } else if let Some((a, b)) = range.split_once(':') {
        let s: usize = a.parse::<usize>().unwrap_or(1).saturating_sub(1);
        let e: usize = b.parse().unwrap_or(total);
        (s, e.min(total))
    } else {
        (0, total.min(50))
    };
    
    json!({
        "job_id": job_id,
        "total_lines": total,
        "range": format!("{}:{}", start + 1, end),
        "content": lines[start..end].join("\n"),
    })
}
