//! Transform utilities - data conversion without external scripts
//! Saves tokens by providing direct transforms instead of PowerShell

use super::auto_backup;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Tool definitions for transforms
pub fn get_definitions() -> Vec<Value> {
    vec![
        // === DATA FORMAT TRANSFORMS ===
        json!({
            "name": "transform_json_format",
            "description": "Pretty-print JSON with proper indentation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_string": { "type": "string", "description": "JSON to format" },
                    "indent": { "type": "integer", "description": "Spaces (default: 2)" }
                },
                "required": ["json_string"]
            }
        }),
        json!({
            "name": "transform_json_minify",
            "description": "Minify JSON by removing whitespace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_string": { "type": "string", "description": "JSON to minify" }
                },
                "required": ["json_string"]
            }
        }),
        json!({
            "name": "transform_base64_encode",
            "description": "Encode string to base64.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to encode" }
                },
                "required": ["text"]
            }
        }),
        json!({
            "name": "transform_base64_decode",
            "description": "Decode base64 to string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "encoded": { "type": "string", "description": "Base64 to decode" }
                },
                "required": ["encoded"]
            }
        }),
        json!({
            "name": "transform_csv_to_json",
            "description": "Convert CSV to JSON array. First row = headers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "csv_string": { "type": "string", "description": "CSV data" },
                    "delimiter": { "type": "string", "description": "Delimiter (default: comma)" }
                },
                "required": ["csv_string"]
            }
        }),
        json!({
            "name": "transform_json_to_csv",
            "description": "Convert JSON array to CSV.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_array": { "type": "string", "description": "JSON array" },
                    "delimiter": { "type": "string", "description": "Delimiter (default: comma)" }
                },
                "required": ["json_array"]
            }
        }),
        // === FILE OPERATIONS (token savers) ===
        json!({
            "name": "transform_diff_file",
            "description": "Compare two files, return unified diff. Saves loading both files into chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_a": { "type": "string", "description": "First file path" },
                    "file_b": { "type": "string", "description": "Second file path" },
                    "context_lines": { "type": "integer", "description": "Context lines (default: 3)" }
                },
                "required": ["file_a", "file_b"]
            }
        }),
        json!({
            "name": "transform_bulk_rename",
            "description": "Rename multiple files with pattern. Returns preview unless execute=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory": { "type": "string", "description": "Directory to scan" },
                    "pattern": { "type": "string", "description": "Regex pattern to match" },
                    "replacement": { "type": "string", "description": "Replacement string ($1, $2 for groups)" },
                    "execute": { "type": "boolean", "description": "Actually rename (default: false = preview)" }
                },
                "required": ["directory", "pattern", "replacement"]
            }
        }),
        json!({
            "name": "transform_find_replace",
            "description": "Find/replace in file(s). Saves reading entire file into chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File or directory path" },
                    "find": { "type": "string", "description": "Text or regex to find" },
                    "replace": { "type": "string", "description": "Replacement text" },
                    "regex": { "type": "boolean", "description": "Use regex (default: false)" },
                    "recursive": { "type": "boolean", "description": "Search subdirs (default: false)" }
                },
                "required": ["path", "find", "replace"]
            }
        }),
        json!({
            "name": "transform_hash_file",
            "description": "Compute file checksum (MD5, SHA256).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "algorithm": { "type": "string", "description": "md5 or sha256 (default: sha256)" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "transform_file_stats",
            "description": "Get file/directory stats without reading content.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to analyze" },
                    "recursive": { "type": "boolean", "description": "Include subdirs (default: false)" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "transform_extract_lines",
            "description": "Extract specific line range from file. Saves reading entire file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "start": { "type": "integer", "description": "Start line (1-indexed)" },
                    "end": { "type": "integer", "description": "End line (inclusive, -1 for EOF)" }
                },
                "required": ["path", "start"]
            }
        }),
        json!({
            "name": "transform_grep",
            "description": "Search files for pattern, return matching lines with context.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File or directory" },
                    "pattern": { "type": "string", "description": "Search pattern (regex)" },
                    "context": { "type": "integer", "description": "Lines of context (default: 0)" },
                    "recursive": { "type": "boolean", "description": "Search subdirs (default: false)" }
                },
                "required": ["path", "pattern"]
            }
        }),
        // === SCAFFOLDING ===
        json!({
            "name": "transform_scaffold",
            "description": "Generate project scaffolding. Creates boilerplate structure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "template": { "type": "string", "description": "Template: rust-mcp, nextjs, expo, python-mcp, fastapi" },
                    "name": { "type": "string", "description": "Project name" },
                    "output_dir": { "type": "string", "description": "Output directory (default: current)" }
                },
                "required": ["template", "name"]
            }
        }),
    ]
}

/// Execute transform tools
pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "transform_json_format" => json_format(args),
        "transform_json_minify" => json_minify(args),
        "transform_base64_encode" => base64_encode(args),
        "transform_base64_decode" => base64_decode(args),
        "transform_csv_to_json" => csv_to_json(args),
        "transform_json_to_csv" => json_to_csv(args),
        "transform_diff_file" | "transform_diff_files" => diff_files(args),
        "transform_bulk_rename" => bulk_rename(args),
        "transform_find_replace" => find_replace(args),
        "transform_hash_file" => hash_file(args),
        "transform_file_stats" => file_stats(args),
        "transform_extract_lines" => extract_lines(args),
        "transform_grep" => grep(args),
        "transform_scaffold" => scaffold(args),
        _ => json!({"error": format!("Unknown transform: {}", name)}),
    }
}

// === IMPLEMENTATIONS ===

fn json_format(args: &Value) -> Value {
    let json_string = match args["json_string"].as_str() {
        Some(s) => s,
        None => return json!({"error": "json_string required"}),
    };
    match serde_json::from_str::<Value>(json_string) {
        Ok(parsed) => json!({"formatted": serde_json::to_string_pretty(&parsed).unwrap()}),
        Err(e) => json!({"error": format!("Invalid JSON: {}", e)}),
    }
}

fn json_minify(args: &Value) -> Value {
    let json_string = match args["json_string"].as_str() {
        Some(s) => s,
        None => return json!({"error": "json_string required"}),
    };
    match serde_json::from_str::<Value>(json_string) {
        Ok(parsed) => json!({"minified": serde_json::to_string(&parsed).unwrap()}),
        Err(e) => json!({"error": format!("Invalid JSON: {}", e)}),
    }
}

fn base64_encode(args: &Value) -> Value {
    let text = match args["text"].as_str() {
        Some(s) => s,
        None => return json!({"error": "text required"}),
    };
    json!({"encoded": BASE64.encode(text.as_bytes())})
}

fn base64_decode(args: &Value) -> Value {
    let encoded = match args["encoded"].as_str() {
        Some(s) => s,
        None => return json!({"error": "encoded required"}),
    };
    match BASE64.decode(encoded) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => json!({"decoded": decoded}),
            Err(_) => json!({"error": "Not valid UTF-8"}),
        },
        Err(e) => json!({"error": format!("Invalid base64: {}", e)}),
    }
}

fn csv_to_json(args: &Value) -> Value {
    let csv = match args["csv_string"].as_str() {
        Some(s) => s,
        None => return json!({"error": "csv_string required"}),
    };
    let delim = args["delimiter"]
        .as_str()
        .unwrap_or(",")
        .chars()
        .next()
        .unwrap_or(',');

    let lines: Vec<&str> = csv.lines().collect();
    if lines.is_empty() {
        return json!({"error": "Empty CSV"});
    }

    let headers: Vec<&str> = lines[0].split(delim).map(|s| s.trim()).collect();
    let records: Vec<Value> = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let vals: Vec<&str> = line.split(delim).map(|s| s.trim()).collect();
            let mut map = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                let v = vals.get(i).unwrap_or(&"");
                map.insert(h.to_string(), json!(v));
            }
            Value::Object(map)
        })
        .collect();

    json!({"records": records, "count": records.len()})
}

fn json_to_csv(args: &Value) -> Value {
    let json_str = match args["json_array"].as_str() {
        Some(s) => s,
        None => return json!({"error": "json_array required"}),
    };
    let delim = args["delimiter"].as_str().unwrap_or(",");

    let array: Vec<Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(e) => return json!({"error": format!("Invalid JSON: {}", e)}),
    };

    if array.is_empty() {
        return json!({"csv": "", "rows": 0});
    }

    let headers: Vec<String> = match &array[0] {
        Value::Object(obj) => obj.keys().cloned().collect(),
        _ => return json!({"error": "Array must contain objects"}),
    };

    let mut lines = vec![headers.join(delim)];
    for item in &array {
        if let Value::Object(obj) = item {
            let vals: Vec<String> = headers
                .iter()
                .map(|h| obj.get(h).map(|v| v.to_string()).unwrap_or_default())
                .collect();
            lines.push(vals.join(delim));
        }
    }
    json!({"csv": lines.join("\n"), "rows": array.len()})
}

fn diff_files(args: &Value) -> Value {
    let file_a = match args["file_a"].as_str() {
        Some(s) => s,
        None => return json!({"error": "file_a required"}),
    };
    let file_b = match args["file_b"].as_str() {
        Some(s) => s,
        None => return json!({"error": "file_b required"}),
    };

    let content_a = match std::fs::read_to_string(file_a) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Can't read {}: {}", file_a, e)}),
    };
    let content_b = match std::fs::read_to_string(file_b) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Can't read {}: {}", file_b, e)}),
    };

    let lines_a: Vec<&str> = content_a.lines().collect();
    let lines_b: Vec<&str> = content_b.lines().collect();

    // Simple line-by-line diff
    let mut diff_lines: Vec<String> = Vec::new();
    let max_len = lines_a.len().max(lines_b.len());
    let mut changes = 0;

    for i in 0..max_len {
        let a = lines_a.get(i);
        let b = lines_b.get(i);
        match (a, b) {
            (Some(la), Some(lb)) if la != lb => {
                diff_lines.push(format!("{}:- {}", i + 1, la));
                diff_lines.push(format!("{}:+ {}", i + 1, lb));
                changes += 1;
            }
            (Some(la), None) => {
                diff_lines.push(format!("{}:- {}", i + 1, la));
                changes += 1;
            }
            (None, Some(lb)) => {
                diff_lines.push(format!("{}:+ {}", i + 1, lb));
                changes += 1;
            }
            _ => {}
        }
    }

    json!({
        "diff": diff_lines.join("\n"),
        "changes": changes,
        "lines_a": lines_a.len(),
        "lines_b": lines_b.len()
    })
}

fn bulk_rename(args: &Value) -> Value {
    let dir = match args["directory"].as_str() {
        Some(s) => s,
        None => return json!({"error": "directory required"}),
    };
    let pattern = match args["pattern"].as_str() {
        Some(s) => s,
        None => return json!({"error": "pattern required"}),
    };
    let replacement = match args["replacement"].as_str() {
        Some(s) => s,
        None => return json!({"error": "replacement required"}),
    };
    let execute = args["execute"].as_bool().unwrap_or(false);

    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("Invalid regex: {}", e)}),
    };

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return json!({"error": format!("Can't read dir: {}", e)}),
    };

    let mut renames: Vec<Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if re.is_match(&name) {
            let new_name = re.replace(&name, replacement).to_string();
            if new_name != name {
                let old_path = entry.path();
                let new_path = old_path.parent().unwrap().join(&new_name);

                if execute {
                    if let Err(e) = std::fs::rename(&old_path, &new_path) {
                        errors.push(format!("{} -> {}: {}", name, new_name, e));
                    } else {
                        renames.push(json!({"from": name, "to": new_name, "done": true}));
                    }
                } else {
                    renames.push(json!({"from": name, "to": new_name, "preview": true}));
                }
            }
        }
    }

    json!({
        "renames": renames,
        "count": renames.len(),
        "executed": execute,
        "errors": errors
    })
}

fn find_replace(args: &Value) -> Value {
    let path = match args["path"].as_str() {
        Some(s) => s,
        None => return json!({"error": "path required"}),
    };
    let find = match args["find"].as_str() {
        Some(s) => s,
        None => return json!({"error": "find required"}),
    };
    let replace = match args["replace"].as_str() {
        Some(s) => s,
        None => return json!({"error": "replace required"}),
    };
    let use_regex = args["regex"].as_bool().unwrap_or(false);

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Can't read: {}", e)}),
    };

    let (new_content, count) = if use_regex {
        match regex::Regex::new(find) {
            Ok(re) => {
                let matches = re.find_iter(&content).count();
                (re.replace_all(&content, replace).to_string(), matches)
            }
            Err(e) => return json!({"error": format!("Invalid regex: {}", e)}),
        }
    } else {
        let count = content.matches(find).count();
        (content.replace(find, replace), count)
    };

    if count > 0 {
        // Auto-backup before write
        auto_backup::backup_if_exists(path);
        if let Err(e) = std::fs::write(path, &new_content) {
            return json!({"error": format!("Can't write: {}", e)});
        }
    }

    json!({"path": path, "replacements": count})
}

fn hash_file(args: &Value) -> Value {
    let path = match args["path"].as_str() {
        Some(s) => s,
        None => return json!({"error": "path required"}),
    };
    let algorithm = args["algorithm"].as_str().unwrap_or("sha256");

    let content = match std::fs::read(path) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Can't read: {}", e)}),
    };

    // Use PowerShell for hashing since we don't have crypto deps
    let algo_upper = algorithm.to_uppercase();
    let result = std::process::Command::new("powershell.exe")
        .args(&[
            "-Command",
            &format!(
                "(Get-FileHash -Path '{}' -Algorithm {}).Hash",
                path, algo_upper
            ),
        ])
        .output();

    match result {
        Ok(output) => {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            json!({
                "path": path,
                "algorithm": algorithm,
                "hash": hash,
                "size": content.len()
            })
        }
        Err(e) => json!({"error": format!("Hash failed: {}", e)}),
    }
}

fn file_stats(args: &Value) -> Value {
    let path = match args["path"].as_str() {
        Some(s) => s,
        None => return json!({"error": "path required"}),
    };
    let recursive = args["recursive"].as_bool().unwrap_or(false);

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return json!({"error": format!("Can't stat: {}", e)}),
    };

    if meta.is_file() {
        json!({
            "type": "file",
            "path": path,
            "size": meta.len(),
            "size_human": format_size(meta.len())
        })
    } else {
        let mut total_size: u64 = 0;
        let mut file_count: u64 = 0;
        let mut dir_count: u64 = 0;

        fn walk(p: &Path, recursive: bool, total: &mut u64, files: &mut u64, dirs: &mut u64) {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    if let Ok(m) = entry.metadata() {
                        if m.is_file() {
                            *total += m.len();
                            *files += 1;
                        } else if m.is_dir() {
                            *dirs += 1;
                            if recursive {
                                walk(&entry.path(), recursive, total, files, dirs);
                            }
                        }
                    }
                }
            }
        }

        walk(
            Path::new(path),
            recursive,
            &mut total_size,
            &mut file_count,
            &mut dir_count,
        );

        json!({
            "type": "directory",
            "path": path,
            "files": file_count,
            "directories": dir_count,
            "total_size": total_size,
            "total_size_human": format_size(total_size),
            "recursive": recursive
        })
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn extract_lines(args: &Value) -> Value {
    let path = match args["path"].as_str() {
        Some(s) => s,
        None => return json!({"error": "path required"}),
    };
    let start = args["start"].as_i64().unwrap_or(1) as usize;
    let end = args["end"].as_i64().unwrap_or(-1);

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return json!({"error": format!("Can't open: {}", e)}),
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line_num = i + 1;
            let in_range = line_num >= start && (end < 0 || line_num <= end as usize);
            if in_range {
                line.ok()
            } else {
                None
            }
        })
        .collect();

    json!({
        "path": path,
        "start": start,
        "end": if end < 0 { "EOF".to_string() } else { end.to_string() },
        "lines": lines,
        "count": lines.len()
    })
}

fn grep(args: &Value) -> Value {
    let path = match args["path"].as_str() {
        Some(s) => s,
        None => return json!({"error": "path required"}),
    };
    let pattern = match args["pattern"].as_str() {
        Some(s) => s,
        None => return json!({"error": "pattern required"}),
    };
    let context = args["context"].as_u64().unwrap_or(0) as usize;

    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("Invalid regex: {}", e)}),
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Can't read: {}", e)}),
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut matches: Vec<Value> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if re.is_match(line) {
            let start = i.saturating_sub(context);
            let end = (i + context + 1).min(lines.len());
            let context_lines: Vec<String> = lines[start..end]
                .iter()
                .enumerate()
                .map(|(j, l)| format!("{}: {}", start + j + 1, l))
                .collect();

            matches.push(json!({
                "line": i + 1,
                "match": line,
                "context": context_lines
            }));
        }
    }

    json!({"path": path, "pattern": pattern, "matches": matches, "count": matches.len()})
}

fn scaffold(args: &Value) -> Value {
    let template = match args["template"].as_str() {
        Some(s) => s,
        None => return json!({"error": "template required"}),
    };
    let name = match args["name"].as_str() {
        Some(s) => s,
        None => return json!({"error": "name required"}),
    };
    let output = args["output_dir"].as_str().unwrap_or(".");

    let base_path = Path::new(output).join(name);

    // Create directory
    if let Err(e) = std::fs::create_dir_all(&base_path) {
        return json!({"error": format!("Can't create dir: {}", e)});
    }

    let files_created: Vec<String> = match template {
        "rust-mcp" => scaffold_rust_mcp(&base_path, name),
        "python-mcp" => scaffold_python_mcp(&base_path, name),
        "nextjs" => scaffold_nextjs(&base_path, name),
        "fastapi" => scaffold_fastapi(&base_path, name),
        _ => {
            return json!({"error": format!("Unknown template: {}. Use: rust-mcp, python-mcp, nextjs, fastapi", template)})
        }
    };

    json!({
        "template": template,
        "name": name,
        "path": base_path.to_string_lossy(),
        "files_created": files_created
    })
}

fn scaffold_rust_mcp(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    // Cargo.toml
    let cargo = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
chrono = {{ version = "0.4", features = ["serde"] }}
"#,
        name
    );
    write_file(&base.join("Cargo.toml"), &cargo, &mut files);

    // src/main.rs
    let main = r#"use std::io::{self, BufRead, Write};
use serde_json::{json, Value};

mod tools;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        if let Ok(input) = line {
            if let Ok(request) = serde_json::from_str::<Value>(&input) {
                let response = handle_request(&request);
                let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap());
                let _ = stdout.flush();
            }
        }
    }
}

fn handle_request(request: &Value) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    
    match method {
        "initialize" => json!({"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": env!("CARGO_PKG_NAME"), "version": "0.1.0"}}),
        "tools/list" => json!({"tools": tools::get_definitions()}),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let args = &request["params"]["arguments"];
            let result = tools::execute(name, args);
            json!({"content": [{"type": "text", "text": serde_json::to_string(&result).unwrap()}]})
        }
        _ => json!({"error": "unknown method"})
    }
}
"#;
    std::fs::create_dir_all(base.join("src")).ok();
    write_file(&base.join("src/main.rs"), main, &mut files);

    // src/tools/mod.rs
    let tools_mod = r#"use serde_json::{json, Value};

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "hello",
            "description": "Say hello",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Name to greet"}
                },
                "required": ["name"]
            }
        }),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "hello" => {
            let n = args["name"].as_str().unwrap_or("World");
            json!({"message": format!("Hello, {}!", n)})
        }
        _ => json!({"error": format!("Unknown tool: {}", name)})
    }
}
"#;
    std::fs::create_dir_all(base.join("src/tools")).ok();
    write_file(&base.join("src/tools/mod.rs"), tools_mod, &mut files);

    files
}

fn scaffold_python_mcp(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let server = format!(
        r#"#!/usr/bin/env python3
"""MCP Server: {}"""
import asyncio
from mcp.server import Server
from mcp.server.stdio import stdio_server

server = Server("{}")

@server.list_tools()
async def list_tools():
    return [
        {{"name": "hello", "description": "Say hello", "inputSchema": {{"type": "object", "properties": {{"name": {{"type": "string"}}}}, "required": ["name"]}}}}
    ]

@server.call_tool()
async def call_tool(name: str, arguments: dict):
    if name == "hello":
        return f"Hello, {{arguments.get('name', 'World')}}!"
    raise ValueError(f"Unknown tool: {{name}}")

async def main():
    async with stdio_server() as (read, write):
        await server.run(read, write, server.create_initialization_options())

if __name__ == "__main__":
    asyncio.run(main())
"#,
        name, name
    );
    write_file(&base.join("server.py"), &server, &mut files);

    let req = "mcp>=1.0.0\n";
    write_file(&base.join("requirements.txt"), req, &mut files);

    files
}

fn scaffold_nextjs(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let package = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  }},
  "dependencies": {{
    "next": "^14.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  }}
}}
"#,
        name
    );
    write_file(&base.join("package.json"), &package, &mut files);

    std::fs::create_dir_all(base.join("app")).ok();
    let page = r#"export default function Home() {
  return <main><h1>Hello World</h1></main>
}
"#;
    write_file(&base.join("app/page.tsx"), page, &mut files);

    let layout = r#"export default function RootLayout({ children }: { children: React.ReactNode }) {
  return <html><body>{children}</body></html>
}
"#;
    write_file(&base.join("app/layout.tsx"), layout, &mut files);

    files
}

fn scaffold_fastapi(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let main = format!(
        r#"from fastapi import FastAPI

app = FastAPI(title="{}")

@app.get("/")
def root():
    return {{"message": "Hello World"}}

@app.get("/health")
def health():
    return {{"status": "ok"}}
"#,
        name
    );
    write_file(&base.join("main.py"), &main, &mut files);

    let req = "fastapi>=0.100.0\nuvicorn>=0.23.0\n";
    write_file(&base.join("requirements.txt"), req, &mut files);

    files
}

fn write_file(path: &Path, content: &str, files: &mut Vec<String>) -> bool {
    match std::fs::write(path, content) {
        Ok(_) => {
            files.push(path.to_string_lossy().to_string());
            true
        }
        Err(_) => false,
    }
}
