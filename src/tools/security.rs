//! Security filtering for mcp-windows
//! Warns on dangerous commands rather than blocking (configurable)
//! Full PATH from Registry is READ-ONLY by design - never edit system PATH

use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use std::path::Path;

/// Sensitive file paths that should NEVER be read/written by AI tools.
/// These are enforced at the tool level, not advisory.
const SENSITIVE_PATH_PATTERNS: &[&str] = &[
    // SSH keys (not config, just private keys)
    ".ssh/id_rsa", ".ssh\\id_rsa",
    ".ssh/id_ed25519", ".ssh\\id_ed25519",
    ".ssh/id_ecdsa", ".ssh\\id_ecdsa",
    ".ssh/id_dsa", ".ssh\\id_dsa",
    // Windows credentials
    "microsoft\\credentials", "microsoft/credentials",
    "microsoft\\protect", "microsoft/protect",
    // Browser passwords (Chrome, Edge, Brave, Firefox)
    "login data",
    "logins.json",
    "key4.db",
    // GPG keys
    ".gnupg/private", ".gnupg\\private",
    ".gnupg/secring", ".gnupg\\secring",
    // Cloud credentials
    ".aws/credentials", ".aws\\credentials",
    ".azure/credentials", ".azure\\credentials",
    // Crypto wallets
    "wallet.dat",
    "keystore/utc", "keystore\\utc",
];

/// Check if a file path is in the sensitive deny list.
/// Returns Err with reason if blocked, Ok(()) if allowed.
pub fn check_sensitive_path(path: &str) -> Result<(), String> {
    // Normalize: lowercase, resolve canonically if possible
    let normalized = if let Ok(canonical) = Path::new(path).canonicalize() {
        canonical.to_string_lossy().to_lowercase()
    } else {
        path.to_lowercase()
    };

    // Also normalize with forward slashes for cross-platform matching
    let forward_slash = normalized.replace('\\', "/");

    for pattern in SENSITIVE_PATH_PATTERNS {
        let p = pattern.to_lowercase();
        if normalized.contains(&p) || forward_slash.contains(&p) {
            return Err(format!(
                "BLOCKED: Access to '{}' denied - matches sensitive path pattern '{}'. \
                 This file contains credentials/keys that should never be accessed by AI tools.",
                path, pattern
            ));
        }
    }

    Ok(())
}

/// Dangerous command patterns - context-aware
const DANGEROUS_PATTERNS: &[(&str, &str)] = &[
    // Catastrophic deletions (only dangerous with root/system paths)
    ("rm -rf /", "Recursive delete of root filesystem"),
    ("rm -rf /*", "Recursive delete of all root contents"),
    ("del /s /q c:\\windows", "Delete Windows system files"),
    ("del /s /q c:\\program", "Delete Program Files"),
    ("format c:", "Format system drive"),
    ("rd /s /q c:\\", "Remove entire C: drive"),
    
    // Fork bombs and resource exhaustion
    (":(){:|:&};:", "Fork bomb (bash)"),
    ("while(1){start powershell}", "Fork bomb (PowerShell)"),
    
    // Registry damage (we read but never write)
    ("reg delete hklm", "Delete system registry keys"),
    ("reg delete hkcu\\software\\microsoft", "Delete critical user registry"),
    
    // Crypto/ransomware patterns
    ("cipher /w:", "Secure wipe (often ransomware)"),
    ("-encodedcommand", "Obfuscated PowerShell (malware pattern)"),
    ("invoke-webrequest.*|iex", "Download and execute pattern"),
    
    // Boot sector / MBR
    ("bootrec /fixmbr", "Modify master boot record"),
    ("bcdedit /delete", "Delete boot configuration"),

    // Remote code execution (pipe to shell)
    ("| bash", "Remote code execution via pipe to bash"),
    ("| sh", "Remote code execution via pipe to shell"),

    // Account/persistence attacks
    ("net user ", "Creating/modifying system user account"),
    ("net localgroup admin", "Modifying administrator group"),
    ("schtasks /create", "Creating scheduled task (persistence vector)"),

    // Startup persistence via registry
    ("currentversion\\run", "Startup persistence via registry Run key"),
    ("currentversion\\runonce", "Startup persistence via registry RunOnce key"),

    // Firewall/network tampering
    ("netsh advfirewall", "Modifying Windows Firewall rules"),
    ("netsh firewall", "Modifying Windows Firewall (legacy)"),

    // Download and execute patterns
    ("bitsadmin /transfer", "Download via BITS (common malware technique)"),
    ("certutil -urlcache", "Download via certutil (common malware technique)"),
    ("mshta ", "Execute via mshta (malware technique)"),
    ("regsvr32 /s /n", "Regsvr32 bypass (Squiblydoo attack)"),

    // Credential/sensitive file access
    (".ssh\\id_rsa", "Reading SSH private key"),
    (".ssh/id_rsa", "Reading SSH private key"),
    (".ssh\\id_ed25519", "Reading SSH private key"),
    (".ssh/id_ed25519", "Reading SSH private key"),
    ("microsoft\\credentials", "Accessing Windows credential store"),
    ("microsoft\\protect", "Accessing Windows DPAPI keys"),
    (".gnupg", "Accessing GPG private keys"),
    (".aws\\credentials", "Accessing AWS credentials"),
    (".aws/credentials", "Accessing AWS credentials"),
    ("login data", "Accessing browser saved passwords"),
    ("wallet.dat", "Accessing cryptocurrency wallet"),
];

/// Context-aware checks - these need path analysis
const CONTEXT_PATTERNS: &[(&str, &str)] = &[
    ("rm -rf", "Recursive delete - check target path"),
    ("del /s", "Recursive delete - check target path"),
    ("rd /s", "Remove directory recursively - check target path"),
    ("rmdir /s", "Remove directory recursively - check target path"),
];

/// Safe path prefixes - deletions here are generally OK
const SAFE_PATHS: &[&str] = &[
    "./", ".\\",
    "node_modules",
    "target/",
    "target\\",
    "dist/",
    "dist\\",
    "build/",
    "build\\",
    "__pycache__",
    ".cache",
    "temp/",
    "temp\\",
    "tmp/",
    "tmp\\",
];

/// Audit log path
const AUDIT_LOG: &str = "C:\\temp\\mcp_security_audit.log";

/// Check command for security issues
/// Returns: (is_safe, warning_message, severity)
pub fn check_command(command: &str) -> (bool, Option<String>, &'static str) {
    let cmd_lower = command.to_lowercase();
    
    // Check absolute dangerous patterns
    for (pattern, reason) in DANGEROUS_PATTERNS {
        if cmd_lower.contains(&pattern.to_lowercase()) {
            // Context-aware: exempt safe read-only uses
            if is_safe_in_context(&cmd_lower, pattern) {
                continue;
            }
            return (false, Some(format!("BLOCKED: {} - {}", pattern, reason)), "critical");
        }
    }
    
    // Check context-aware patterns
    for (pattern, reason) in CONTEXT_PATTERNS {
        if cmd_lower.contains(&pattern.to_lowercase()) {
            // Extract the path being operated on
            if !is_safe_path(&cmd_lower) {
                return (false, Some(format!("WARNING: {} - verify target is safe", reason)), "warning");
            }
        }
    }
    
    (true, None, "safe")
}

/// Context-aware exemptions for dangerous patterns.
fn is_safe_in_context(cmd_lower: &str, pattern: &str) -> bool {
    let p = pattern.to_lowercase();
    // "net user " is safe when just viewing (no /add, /delete)
    if p.starts_with("net user") && !cmd_lower.contains("/add") && !cmd_lower.contains("/delete") {
        return true;
    }
    // "netsh advfirewall" is safe when viewing (show/display)
    if p.starts_with("netsh advfirewall") && (cmd_lower.contains("show") || cmd_lower.contains("display")) {
        return true;
    }
    // "currentversion\run" is safe when querying registry, not adding
    if p.contains("currentversion") && (cmd_lower.starts_with("reg query") || cmd_lower.contains("reg query")) {
        return true;
    }
    false
}

/// Check if path is in safe list
fn is_safe_path(command: &str) -> bool {
    for safe in SAFE_PATHS {
        if command.contains(safe) {
            return true;
        }
    }
    false
}

/// Log security event to audit file
pub fn audit_log(command: &str, result: &str, severity: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let entry = format!("[{}] [{}] {} | {}\n", timestamp, severity, result, command);
    
    // Ensure directory exists
    let _ = std::fs::create_dir_all("C:\\temp");
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(AUDIT_LOG)
    {
        let _ = file.write_all(entry.as_bytes());
    }
}

/// Tool definitions for security
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "security_check_cmd",
            "description": "Check if a command is safe to execute. Returns warnings for dangerous patterns.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to check"
                    }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "security_audit_log",
            "description": "View recent security audit log entries.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "lines": {
                        "type": "integer",
                        "description": "Number of recent entries (default: 20)"
                    }
                }
            }
        }),
    ]
}

/// Execute security tools
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "security_check_cmd" | "security_check_command" => {
            let command = args["command"].as_str().unwrap_or("");
            let (is_safe, warning, severity) = check_command(command);
            
            json!({
                "safe": is_safe,
                "severity": severity,
                "warning": warning,
                "command": command
            })
        },
        "security_audit_log" => {
            let lines = args["lines"].as_u64().unwrap_or(20) as usize;
            match std::fs::read_to_string(AUDIT_LOG) {
                Ok(content) => {
                    let entries: Vec<&str> = content.lines().rev().take(lines).collect();
                    json!({
                        "entries": entries,
                        "count": entries.len(),
                        "log_path": AUDIT_LOG
                    })
                },
                Err(_) => json!({
                    "entries": [],
                    "count": 0,
                    "note": "No audit log yet"
                })
            }
        },
        _ => json!({"error": format!("Unknown security tool: {}", name)})
    }
}