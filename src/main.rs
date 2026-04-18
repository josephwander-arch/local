//! MCP-Windows: Raw tools + Windows automation for Claude Desktop
//! Replaces: raw-tools (Python) + windows-mcp (uvx)
// NAV: TOC at line 152 | 2 fn | 2 struct | 2026-04-15

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

mod dashboard_endpoint;
mod tools;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn main() {
    // Non-blocking cleanup of breadcrumb archives older than LOCAL_BREADCRUMB_RETENTION_DAYS (default 30d)
    tools::breadcrumbs_startup_cleanup();

    // Spawn HTTP dashboard endpoint (127.0.0.1:9101 by default)
    dashboard_endpoint::spawn();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                continue;
            }
        };

        // Validate JSON-RPC 2.0 version
        if request.jsonrpc != "2.0" {
            eprintln!("Invalid JSON-RPC version: {}", request.jsonrpc);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone().unwrap_or(Value::Null),
                result: None,
                error: Some(json!({
                    "code": -32600,
                    "message": format!("Invalid JSON-RPC version: expected '2.0', got '{}'", request.jsonrpc)
                })),
            };
            writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).unwrap();
            stdout.flush().unwrap();
            continue;
        }

        // Handle notifications (no id)
        if request.id.is_none() {
            continue;
        }

        let response = handle_request(&request);
        let response_str = serde_json::to_string(&response).unwrap();
        writeln!(stdout, "{}", response_str).unwrap();
        stdout.flush().unwrap();
    }
}

fn handle_request(request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "local",
                    "version": "0.1.0"
                }
            })),
            error: None,
        },

        "tools/list" => {
            let all_tools = tools::get_all_definitions();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "tools": all_tools })),
                error: None,
            }
        }

        "tools/call" => {
            let params = request.params.as_ref().unwrap_or(&Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            let tc_start = std::time::Instant::now();
            let result = tools::execute(name, &args);
            let tc_ms = tc_start.elapsed().as_millis() as u64;

            // B1: record tool call for dashboard feed
            let input_preview = {
                let s = args.to_string();
                if s.len() > 80 {
                    format!("{}…", &s[..80])
                } else {
                    s
                }
            };
            tools::record_tool_call(tools::ToolCallEntry {
                tool_name: name.to_string(),
                timestamp_utc: chrono::Utc::now().to_rfc3339(),
                input_preview,
                duration_ms: tc_ms,
            });

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    }]
                })),
                error: None,
            }
        }

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": format!("Method not found: {}", request.method)
            })),
        },
    }
}

// === FILE NAVIGATION ===
// Generated: 2026-04-15T22:03:42
// Total: 149 lines | 2 functions | 2 structs | 0 constants
//
// IMPORTS: serde, serde_json, std
//
// STRUCTS:
//   JsonRpcRequest: 12-19
//   JsonRpcResponse: 22-29
//
// FUNCTIONS:
//   main: 31-86 [med]
//   handle_request: 88-149 [med]
//
// === END FILE NAVIGATION ===
