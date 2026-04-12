//! Persistent Shell Sessions for mcp-windows (REAL persistence)
//! Maintains actual shell process with stdin/stdout pipes across calls

use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader, Write};
use std::thread;
use once_cell::sync::Lazy;
use uuid::Uuid;
use super::security;
use super::log;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// CREATE_NO_WINDOW flag for Windows
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Session storage
static SESSIONS: Lazy<Arc<Mutex<HashMap<String, PersistentSession>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

struct PersistentSession {
    name: String,
    child: Child,
    output_buffer: Arc<Mutex<Vec<String>>>,
    cwd: String,
    env: HashMap<String, String>,
    history: Vec<String>,
    created_at: String,
}

// Reader thread - continuously reads from process stdout
fn start_output_reader(
    stdout: std::process::ChildStdout, 
    buffer: Arc<Mutex<Vec<String>>>
) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let mut buf = buffer.lock().unwrap();
                    buf.push(l);
                }
                Err(_) => break,
            }
        }
    });
}

impl PersistentSession {
    fn new(name: &str, cwd: Option<&str>) -> Result<Self, String> {
        let working_dir = cwd.unwrap_or("C:\\").to_string();
        
        // Build command
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoLogo", "-NoProfile", "-Command", "-"])
            .current_dir(&working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Windows-specific: hide window
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        
        // Spawn
        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn PowerShell: {}", e))?;
        
        // Take stdout for reader thread
        let stdout = child.stdout.take()
            .ok_or("Failed to take stdout")?;
        
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        
        // Start background reader
        start_output_reader(stdout, output_buffer.clone());
        
        // Give shell a moment to initialize
        thread::sleep(std::time::Duration::from_millis(100));
        
        Ok(Self {
            name: name.to_string(),
            child,
            output_buffer,
            cwd: working_dir,
            env: HashMap::new(),
            history: Vec::new(),
            created_at: chrono::Local::now().to_rfc3339(),
        })
    }
    
    fn run_command(&mut self, command: &str, timeout_secs: u64) -> Result<Value, String> {
        // Generate unique marker for exit code detection
        let marker = format!("__EXIT_{}__", &Uuid::new_v4().to_string()[..8]);
        
        // Build command with exit code capture
        // Write-Host outputs marker followed by $LASTEXITCODE
        let full_cmd = format!(
            "{}; Write-Host '{}' $LASTEXITCODE\n",
            command, marker
        );
        
        // Write to stdin
        let stdin = self.child.stdin.as_mut()
            .ok_or("No stdin available - session may be dead")?;
        
        stdin.write_all(full_cmd.as_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        stdin.flush()
            .map_err(|e| format!("Flush failed: {}", e))?;
        
        // Wait for marker in output
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let mut collected = Vec::new();
        
        loop {
            if start.elapsed() > timeout {
                return Err(format!("Command timed out after {}s", timeout_secs));
            }
            
            // Drain buffer
            {
                let mut buf = self.output_buffer.lock().unwrap();
                collected.append(&mut *buf);
            }
            
            // Check for marker
            let full_output = collected.join("\n");
            if full_output.contains(&marker) {
                // Parse exit code and clean output
                let (clean_output, exit_code) = self.parse_output(&collected, &marker);
                
                // Record in history
                self.history.push(command.to_string());
                
                // Auto-checkpoint every 5 commands
                if self.history.len() % 5 == 0 {
                    self.auto_checkpoint();
                }
                
                // Update cwd if cd command
                self.maybe_update_cwd(command);
                
                return Ok(json!({
                    "success": exit_code == 0,
                    "session": self.name,
                    "cwd": self.cwd,
                    "output": clean_output.trim(),
                    "exit_code": exit_code
                }));
            }
            
            thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    
    fn parse_output(&self, lines: &[String], marker: &str) -> (String, i32) {
        let mut output_lines = Vec::new();
        let mut exit_code = 0;
        
        for line in lines {
            if line.contains(marker) {
                // Extract exit code: "__EXIT_abc12345__ 0"
                let parts: Vec<&str> = line.split(marker).collect();
                if parts.len() > 1 {
                    exit_code = parts[1].trim().parse().unwrap_or(0);
                }
            } else {
                output_lines.push(line.clone());
            }
        }
        
        (output_lines.join("\n"), exit_code)
    }
    
    fn maybe_update_cwd(&mut self, command: &str) {
        let cmd_lower = command.to_lowercase();
        let cmd_trimmed = cmd_lower.trim();
        
        // Handle cd, Set-Location, Push-Location
        let path = if cmd_trimmed.starts_with("cd ") {
            Some(command.trim()[3..].trim())
        } else if cmd_trimmed.starts_with("set-location ") {
            Some(command.trim()[13..].trim())
        } else if cmd_trimmed.starts_with("sl ") {
            Some(command.trim()[3..].trim())
        } else {
            None
        };
        
        if let Some(p) = path {
            // Remove quotes if present
            let clean_path = p.trim_matches(|c| c == '\'' || c == '"');
            
            let new_path = if std::path::Path::new(clean_path).is_absolute() {
                clean_path.to_string()
            } else {
                format!("{}\\{}", self.cwd, clean_path)
            };
            
            // Canonicalize and verify
            if let Ok(canonical) = std::fs::canonicalize(&new_path) {
                self.cwd = canonical.to_string_lossy().to_string();
            }
        }
    }
    
    fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false,  // Process has exited
            Ok(None) => true,      // Still running
            Err(_) => false,       // Error checking
        }
    }
    
    /// Auto-checkpoint to file (silent, for crash recovery)
    fn auto_checkpoint(&self) {
        let checkpoint_dir = "C:\\temp\\session_checkpoints";
        let _ = std::fs::create_dir_all(checkpoint_dir);
        
        let checkpoint = serde_json::json!({
            "session_name": self.name,
            "cwd": self.cwd,
            "env": self.env,
            "history": self.history,
            "saved_at": chrono::Local::now().to_rfc3339(),
            "auto": true
        });
        
        let path = format!("{}\\{}.json", checkpoint_dir, self.name);
        let _ = std::fs::write(&path, serde_json::to_string_pretty(&checkpoint).unwrap_or_default());
    }
}

impl Drop for PersistentSession {
    fn drop(&mut self) {
        // Kill child process on drop
        let _ = self.child.kill();
    }
}

/// Tool definitions
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "session_create",
            "description": "Create a persistent shell session. Env vars and cwd persist across calls.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Session name (default: 'default')"
                    },
                    "cwd": {
                        "type": "string", 
                        "description": "Initial working directory"
                    }
                }
            }
        }),
        json!({
            "name": "session_run",
            "description": "Run command in persistent session. Inherits env and cwd from session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name (default: 'default')"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "session_cd",
            "description": "Change directory in session. Persists for subsequent commands.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to change to"
                    }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "session_set_env",
            "description": "Set environment variable in session. Persists for subsequent commands.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Environment variable name"
                    },
                    "value": {
                        "type": "string",
                        "description": "Environment variable value"
                    }
                },
                "required": ["key", "value"]
            }
        }),
        json!({
            "name": "session_get_env",
            "description": "Get environment variable(s) from session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Specific key (empty for all)"
                    }
                }
            }
        }),
        json!({
            "name": "session_list",
            "description": "List all active sessions with their state.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "session_destroy",
            "description": "Destroy a session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name to destroy"
                    }
                },
                "required": ["session"]
            }
        }),
        json!({
            "name": "session_history",
            "description": "Get command history for a session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max commands to return (default: 20)"
                    }
                }
            }
        }),
        json!({
            "name": "session_checkpoint",
            "description": "Save session state to file for crash recovery.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "checkpoint_path": {
                        "type": "string",
                        "description": "Path to save checkpoint (default: C:/temp/session_{name}.checkpoint)"
                    }
                }
            }
        }),
        json!({
            "name": "session_recover",
            "description": "Recover session from checkpoint file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "checkpoint_path": {
                        "type": "string",
                        "description": "Path to checkpoint file"
                    }
                },
                "required": ["checkpoint_path"]
            }
        }),
        json!({
            "name": "session_read_output",
            "description": "Read buffered output from session without blocking. Use for long-running commands.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Max lines to return (default: all)"
                    },
                    "clear": {
                        "type": "boolean",
                        "description": "Clear buffer after reading (default: true)"
                    }
                }
            }
        }),
    ]
}

/// Execute session tools
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "session_create" => session_create(args),
        "session_run" => session_run(args),
        "session_cd" => session_cd(args),
        "session_set_env" => session_setenv(args),
        "session_get_env" => session_getenv(args),
        "session_list" => session_list(args),
        "session_destroy" => session_destroy(args),
        "session_history" => session_history(args),
        "session_checkpoint" => session_checkpoint(args),
        "session_recover" => session_recover(args),
        "session_read_output" => session_read_output(args),
        _ => json!({"error": format!("Unknown tool: {}", name)}),
    }
}

fn session_create(args: &Value) -> Value {
    let name = args["name"].as_str().unwrap_or("default");
    let cwd = args["cwd"].as_str();
    
    let mut sessions = SESSIONS.lock().unwrap();
    
    if sessions.contains_key(name) {
        // Session exists - return helpful response suggesting reuse
        let session = sessions.get(name).unwrap();
        return json!({
            "exists": true,
            "session": name,
            "cwd": session.cwd.clone(),
            "history_count": session.history.len(),
            "message": format!("Session '{}' already exists - use session_run to reuse it", name),
            "hint": "To recreate: session_destroy first, then session_create"
        });
    }
    
    match PersistentSession::new(name, cwd) {
        Ok(session) => {
            let cwd_used = session.cwd.clone();
            sessions.insert(name.to_string(), session);
            
            json!({
                "success": true,
                "session": name,
                "cwd": cwd_used,
                "persistent": true,
                "message": format!("Persistent session '{}' created. PowerShell process spawned.", name)
            })
        }
        Err(e) => json!({
            "success": false,
            "error": e
        })
    }
}

fn session_run(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default");
    let command = match args["command"].as_str() {
        Some(c) => c,
        None => return json!({"error": "command required"}),
    };
    
    // Security pre-check - warn on dangerous commands
    let (is_safe, warning, severity) = security::check_command(command);
    if !is_safe {
        // Log to audit
        security::audit_log(command, "blocked_pre_run", severity);
        return json!({
            "blocked": true,
            "command": command,
            "severity": severity,
            "warning": warning,
            "hint": "If intentional, use raw_run with explicit acknowledgment"
        });
    }
    
    let mut sessions = SESSIONS.lock().unwrap();
    
    // Auto-create default session if needed
    if !sessions.contains_key(session_name) && session_name == "default" {
        match PersistentSession::new("default", None) {
            Ok(s) => { sessions.insert("default".to_string(), s); }
            Err(e) => return json!({"error": format!("Failed to create default session: {}", e)})
        }
    }
    
    let session = match sessions.get_mut(session_name) {
        Some(s) => s,
        None => return json!({
            "error": format!("Session '{}' not found", session_name),
            "hint": "Create with session_create first"
        }),
    };
    
    // Check if session is still alive
    if !session.is_alive() {
        return json!({
            "error": "Session process has died",
            "hint": "Destroy and recreate the session"
        });
    }
    
    // Execute with 30 second timeout
    match session.run_command(command, 30) {
        Ok(result) => {
            let success = result["success"].as_bool().unwrap_or(false);
            let output = result["output"].as_str().unwrap_or("");
            log::log_execution("session_run", command, output, "", success);
            result
        }
        Err(e) => {
            log::log_execution("session_run", command, "", &e, false);
            json!({
                "error": e,
                "session": session_name
            })
        }
    }
}

fn session_cd(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default");
    let path = match args["path"].as_str() {
        Some(p) => p,
        None => return json!({"error": "path required"}),
    };
    
    // Execute cd command through session_run
    let run_args = json!({
        "session": session_name,
        "command": format!("cd '{}'", path)
    });
    
    session_run(&run_args)
}

fn session_setenv(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default");
    let key = match args["key"].as_str() {
        Some(k) => k,
        None => return json!({"error": "key required"}),
    };
    let value = match args["value"].as_str() {
        Some(v) => v,
        None => return json!({"error": "value required"}),
    };
    
    // Set via PowerShell $env: syntax
    let run_args = json!({
        "session": session_name,
        "command": format!("$env:{}='{}'", key, value)
    });
    
    let result = session_run(&run_args);
    
    // Also track in our env map
    let mut sessions = SESSIONS.lock().unwrap();
    if let Some(session) = sessions.get_mut(session_name) {
        session.env.insert(key.to_string(), value.to_string());
    }
    
    result
}

fn session_getenv(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default");
    let key = args["key"].as_str();
    
    match key {
        Some(k) if !k.is_empty() => {
            // Get specific env var via PowerShell
            let run_args = json!({
                "session": session_name,
                "command": format!("$env:{}", k)
            });
            session_run(&run_args)
        }
        _ => {
            // Return our tracked env vars
            let sessions = SESSIONS.lock().unwrap();
            match sessions.get(session_name) {
                Some(s) => json!(s.env),
                None => json!({"error": format!("Session '{}' not found", session_name)})
            }
        }
    }
}

fn session_list(_args: &Value) -> Value {
    let mut sessions = SESSIONS.lock().unwrap();
    
    let list: Vec<Value> = sessions.iter_mut().map(|(_, s)| {
        json!({
            "name": s.name,
            "cwd": s.cwd,
            "env_count": s.env.len(),
            "history_count": s.history.len(),
            "created_at": s.created_at,
            "alive": s.is_alive(),
            "persistent": true
        })
    }).collect();
    
    json!({
        "sessions": list,
        "count": list.len()
    })
}

fn session_destroy(args: &Value) -> Value {
    let session_name = match args["session"].as_str() {
        Some(s) => s,
        None => return json!({"error": "session name required"}),
    };
    
    let mut sessions = SESSIONS.lock().unwrap();
    
    match sessions.remove(session_name) {
        Some(_) => json!({
            "success": true,
            "destroyed": session_name,
            "message": "Session and PowerShell process terminated"
        }),
        None => json!({
            "success": false,
            "error": format!("Session '{}' not found", session_name)
        }),
    }
}

fn session_history(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default");
    let limit = args["limit"].as_u64().unwrap_or(20) as usize;
    
    let sessions = SESSIONS.lock().unwrap();
    
    let session = match sessions.get(session_name) {
        Some(s) => s,
        None => return json!({"error": format!("Session '{}' not found", session_name)}),
    };
    
    let history: Vec<&String> = session.history.iter().rev().take(limit).collect();
    
    json!({
        "session": session_name,
        "history": history,
        "total_commands": session.history.len()
    })
}


/// Save session state to checkpoint file for crash recovery
fn session_checkpoint(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default").to_string();
    let default_path = format!("C:/temp/session_{}.checkpoint", session_name);
    let checkpoint_path = args["checkpoint_path"].as_str().unwrap_or(&default_path);
    
    let sessions = SESSIONS.lock().unwrap();
    let session = match sessions.get(&session_name) {
        Some(s) => s,
        None => return json!({"error": format!("Session '{}' not found", session_name)}),
    };
    
    // Build checkpoint data
    let checkpoint = json!({
        "session_name": session_name,
        "cwd": session.cwd,
        "env": session.env,
        "history": session.history,
        "saved_at": chrono::Utc::now().to_rfc3339(),
    });
    
    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(checkpoint_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    match std::fs::write(checkpoint_path, serde_json::to_string_pretty(&checkpoint).unwrap()) {
        Ok(_) => json!({
            "success": true,
            "checkpoint_path": checkpoint_path,
            "session": session_name,
            "commands_saved": session.history.len()
        }),
        Err(e) => json!({"error": format!("Failed to write checkpoint: {}", e)}),
    }
}

/// Recover session from checkpoint file
fn session_recover(args: &Value) -> Value {
    let checkpoint_path = match args["checkpoint_path"].as_str() {
        Some(p) => p,
        None => return json!({"error": "checkpoint_path required"}),
    };
    
    // Read checkpoint file
    let checkpoint_data = match std::fs::read_to_string(checkpoint_path) {
        Ok(data) => data,
        Err(e) => return json!({"error": format!("Failed to read checkpoint: {}", e)}),
    };
    
    let checkpoint: Value = match serde_json::from_str(&checkpoint_data) {
        Ok(v) => v,
        Err(e) => return json!({"error": format!("Invalid checkpoint format: {}", e)}),
    };
    
    let session_name = checkpoint["session_name"].as_str().unwrap_or("recovered").to_string();
    let cwd = checkpoint["cwd"].as_str().unwrap_or("C:\\").to_string();
    
    // Restore environment
    let mut env: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(saved_env) = checkpoint["env"].as_object() {
        for (k, v) in saved_env {
            if let Some(val) = v.as_str() {
                env.insert(k.clone(), val.to_string());
            }
        }
    }
    
    // Restore history
    let mut history: Vec<String> = Vec::new();
    if let Some(saved_history) = checkpoint["history"].as_array() {
        for item in saved_history {
            if let Some(cmd) = item.as_str() {
                history.push(cmd.to_string());
            }
        }
    }
    
    // Create new session with restored state
    // First spawn a new PowerShell process
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args(&["-NoProfile", "-NoExit", "-Command", "-"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.current_dir(&cwd);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Failed to spawn PowerShell: {}", e)}),
    };
    
    let session = PersistentSession {
        name: session_name.clone(),
        child,
        cwd,
        env,
        history,
        created_at: chrono::Utc::now().to_rfc3339(),
        output_buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
    };
    
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.insert(session_name.clone(), session);
    
    json!({
        "success": true,
        "session": session_name,
        "recovered_from": checkpoint_path,
        "saved_at": checkpoint["saved_at"],
        "commands_restored": checkpoint["history"].as_array().map(|a| a.len()).unwrap_or(0)
    })
}

/// Read buffered output without blocking - for long-running commands
fn session_read_output(args: &Value) -> Value {
    let session_name = args["session"].as_str().unwrap_or("default").to_string();
    let max_lines = args["lines"].as_u64().map(|n| n as usize);
    let clear = args["clear"].as_bool().unwrap_or(true);
    
    let sessions = SESSIONS.lock().unwrap();
    let session = match sessions.get(&session_name) {
        Some(s) => s,
        None => return json!({"error": format!("Session '{}' not found", session_name)}),
    };
    
    let mut buffer = session.output_buffer.lock().unwrap();
    
    let output: Vec<String> = if let Some(limit) = max_lines {
        buffer.iter().rev().take(limit).rev().cloned().collect()
    } else {
        buffer.clone()
    };
    
    let line_count = output.len();
    
    if clear {
        buffer.clear();
    }
    
    json!({
        "session": session_name,
        "output": output.join("\n"),
        "lines": line_count,
        "cleared": clear
    })
}
