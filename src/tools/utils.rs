//! Utility tools for common Claude workflows
//! Auto-backup, validation, document generation
// NAV: TOC at line 175 | 4 fn | 0 struct | 2026-02-03

use serde_json::{json, Value};
use std::process::Command;

const SCRIPTS_DIR: &str = "C:\\My Drive\\scripts";

/// Run a PowerShell script and capture output
fn run_ps_script(script_path: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-File", script_path])
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(format!("Exit code {}: {}", output.status.code().unwrap_or(-1), stderr))
    }
}

/// Run md2docx.bat with input and output paths
fn run_md2docx(input: &str, output: &str) -> Result<String, String> {
    let bat_path = format!("{}\\md2docx.bat", SCRIPTS_DIR);
    
    // Check input exists
    if !std::path::Path::new(input).exists() {
        return Err(format!("Input file not found: {}", input));
    }
    
    let cmd_output = Command::new("cmd")
        .args(["/c", &bat_path, input, output])
        .env("NODE_PATH", format!("{}\\npm\\node_modules", std::env::var("APPDATA").unwrap_or_default()))
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;
    
    if cmd_output.status.success() {
        // Check output was created
        if std::path::Path::new(output).exists() {
            let metadata = std::fs::metadata(output).ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            Ok(format!("Created: {} ({}KB)", output, size / 1024))
        } else {
            Err("Conversion ran but output file not created".to_string())
        }
    } else {
        let stderr = String::from_utf8_lossy(&cmd_output.stderr);
        Err(format!("Conversion failed: {}", stderr))
    }
}

/// Tool definitions
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "config_backup",
            "description": "Backup claude_desktop_config.json before editing. Creates timestamped copy, keeps last 10.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "config_backup_operating",
            "description": "Backup all Operating_*.md files before editing. Creates timestamped folder, keeps last 5.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "config_validate",
            "description": "Validate claude_desktop_config.json after editing. Checks JSON syntax, server paths, warns on issues.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "md2docx",
            "description": "Convert Markdown file to DOCX. Supports headings, bold, italic, tables, lists, page breaks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Path to input .md file"
                    },
                    "output": {
                        "type": "string",
                        "description": "Path for output .docx file"
                    }
                },
                "required": ["input", "output"]
            }
        }),
    ]
}

/// Execute utility tools
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "config_backup" | "util_backup_config" => {
            let script = format!("{}\\backup_config.ps1", SCRIPTS_DIR);
            match run_ps_script(&script) {
                Ok(output) => json!({
                    "success": true,
                    "message": output
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e
                })
            }
        },
        "config_backup_operating" | "util_backup_operating" => {
            let script = format!("{}\\backup_operating.ps1", SCRIPTS_DIR);
            match run_ps_script(&script) {
                Ok(output) => json!({
                    "success": true,
                    "message": output
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e
                })
            }
        },
        "config_validate" | "util_validate_config" => {
            let script = format!("{}\\validate_config.ps1", SCRIPTS_DIR);
            match run_ps_script(&script) {
                Ok(output) => json!({
                    "success": true,
                    "valid": true,
                    "output": output
                }),
                Err(e) => json!({
                    "success": false,
                    "valid": false,
                    "error": e
                })
            }
        },
        "md2docx" => {
            let input = args["input"].as_str().unwrap_or("");
            let output = args["output"].as_str().unwrap_or("");
            
            if input.is_empty() || output.is_empty() {
                return json!({
                    "success": false,
                    "error": "Both input and output paths required"
                });
            }
            
            match run_md2docx(input, output) {
                Ok(msg) => json!({
                    "success": true,
                    "message": msg
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e
                })
            }
        },
        _ => json!({"error": format!("Unknown util tool: {}", name)})
    }
}

// === FILE NAVIGATION ===
// Generated: 2026-02-03T22:41:07
// Total: 172 lines | 4 functions | 0 structs | 1 constants
//
// IMPORTS: serde_json, std
//
// CONSTANTS:
//   const SCRIPTS_DIR: 7
//
// FUNCTIONS:
//   run_ps_script: 10-24
//   run_md2docx: 27-54
//   pub +get_definitions: 57-102
//   pub +execute: 105-172 [med]
//
// === END FILE NAVIGATION ===