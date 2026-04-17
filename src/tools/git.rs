//! Git tools - Local repository management for rollback recovery
//!
//! Tools (5):
//! git_status, git_log, git_commit, git_stash, git_reset
//! git_stash, git_reset, git_branch

use serde_json::{json, Value};
use std::process::Command;

/// Tool definitions for MCP
pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "git_status",
            "description": "Get git status: branch, modified, staged files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Repository path (default: C:\\rust-mcp)",
                        "default": "C:\\rust-mcp"
                    }
                }
            }
        }),
        json!({
            "name": "git_log",
            "description": "Get commit history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Repository path (default: C:\\rust-mcp)",
                        "default": "C:\\rust-mcp"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max commits to show (default: 10)",
                        "default": 10
                    },
                    "oneline": {
                        "type": "boolean",
                        "description": "Use compact one-line format (default: true)",
                        "default": true
                    }
                }
            }
        }),
        json!({
            "name": "git_commit",
            "description": "Stage files and create commit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Repository path (default: C:\\rust-mcp)",
                        "default": "C:\\rust-mcp"
                    },
                    "message": {
                        "type": "string",
                        "description": "Commit message"
                    },
                    "files": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files to stage (empty = all changed)"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Stage all changes (-a)",
                        "default": false
                    }
                },
                "required": ["message"]
            }
        }),
        json!({
            "name": "git_stash",
            "description": "Git stash operations: push, pop, list, drop.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Repository path (default: C:\\rust-mcp)",
                        "default": "C:\\rust-mcp"
                    },
                    "action": {
                        "type": "string",
                        "description": "push, pop, list, drop, show",
                        "default": "push"
                    },
                    "message": {
                        "type": "string",
                        "description": "Stash message (for push)"
                    },
                    "index": {
                        "type": "integer",
                        "description": "Stash index (for pop, drop, show)",
                        "default": 0
                    }
                }
            }
        }),
        json!({
            "name": "git_reset",
            "description": "Reset to commit. Use for rollback.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Repository path (default: C:\\rust-mcp)",
                        "default": "C:\\rust-mcp"
                    },
                    "target": {
                        "type": "string",
                        "description": "Commit hash, HEAD~N, or branch",
                        "default": "HEAD~1"
                    },
                    "mode": {
                        "type": "string",
                        "description": "soft, mixed, hard (default: hard)",
                        "default": "hard"
                    }
                }
            }
        }),
        json!({
            "name": "git_diff",
            "description": "Get git diff of working directory or staged changes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Repository path (default: C:\\\\rust-mcp)", "default": "C:\\\\rust-mcp" },
                    "staged": { "type": "boolean", "description": "Show staged changes (default: false)", "default": false },
                    "file": { "type": "string", "description": "Specific file to diff (optional)" }
                }
            }
        }),
        json!({
            "name": "git_branch",
            "description": "List, create, or delete branches.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Repository path (default: C:\\\\rust-mcp)", "default": "C:\\\\rust-mcp" },
                    "action": { "type": "string", "description": "list (default), create, delete", "default": "list" },
                    "name": { "type": "string", "description": "Branch name (for create/delete)" }
                }
            }
        }),
        json!({
            "name": "git_checkout",
            "description": "Switch branch or restore file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": { "type": "string", "description": "Repository path (default: C:\\\\rust-mcp)", "default": "C:\\\\rust-mcp" },
                    "target": { "type": "string", "description": "Branch name or file path" },
                    "create": { "type": "boolean", "description": "Create new branch (-b flag)", "default": false }
                },
                "required": ["target"]
            }
        }),
    ]
}

fn get_repo_path(args: &Value) -> String {
    args["repo_path"]
        .as_str()
        .unwrap_or("C:\\rust-mcp")
        .to_string()
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "git_status" => git_status(args),
        "git_log" => git_log(args),
        "git_commit" => git_commit(args),
        "git_stash" => git_stash(args),
        "git_reset" => git_reset(args),
        "git_diff" => git_diff(args),
        "git_branch" => git_branch(args),
        "git_checkout" => git_checkout(args),
        _ => json!({"error": format!("Unknown git tool: {}", name)}),
    }
}

fn git_status(args: &Value) -> Value {
    let repo = get_repo_path(args);

    // Get branch
    let branch_out = Command::new("git")
        .args(["-C", &repo, "rev-parse", "--abbrev-ref", "HEAD"])
        .output();

    let branch = branch_out
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Get status
    let status_out = Command::new("git")
        .args(["-C", &repo, "status", "--porcelain"])
        .output();

    let status_text = status_out
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut modified = Vec::new();
    let mut staged = Vec::new();
    let mut untracked = Vec::new();

    for line in status_text.lines() {
        if line.len() < 3 {
            continue;
        }
        let index = line.chars().nth(0).unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');
        let file = line[3..].to_string();

        if index != ' ' && index != '?' {
            staged.push(file.clone());
        }
        if worktree == 'M' || worktree == 'D' {
            modified.push(file.clone());
        }
        if index == '?' {
            untracked.push(file);
        }
    }

    json!({
        "branch": branch,
        "clean": modified.is_empty() && staged.is_empty() && untracked.is_empty(),
        "modified": modified,
        "staged": staged,
        "untracked": untracked
    })
}

fn git_log(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let limit = args["limit"].as_i64().unwrap_or(10);
    let oneline = args["oneline"].as_bool().unwrap_or(true);

    let format = if oneline {
        "--oneline"
    } else {
        "--format=%H|%s|%an|%ar"
    };

    let output = Command::new("git")
        .args(["-C", &repo, "log", format, &format!("-{}", limit)])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            if oneline {
                let commits: Vec<Value> = text
                    .lines()
                    .map(|line| {
                        let parts: Vec<&str> = line.splitn(2, ' ').collect();
                        json!({
                            "hash": parts.get(0).unwrap_or(&""),
                            "message": parts.get(1).unwrap_or(&"")
                        })
                    })
                    .collect();
                json!({"commits": commits})
            } else {
                let commits: Vec<Value> = text
                    .lines()
                    .map(|line| {
                        let parts: Vec<&str> = line.split('|').collect();
                        json!({
                            "hash": parts.get(0).unwrap_or(&""),
                            "message": parts.get(1).unwrap_or(&""),
                            "author": parts.get(2).unwrap_or(&""),
                            "when": parts.get(3).unwrap_or(&"")
                        })
                    })
                    .collect();
                json!({"commits": commits})
            }
        }
        Ok(o) => json!({"error": String::from_utf8_lossy(&o.stderr).to_string()}),
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn git_commit(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let message = match args["message"].as_str() {
        Some(m) => m,
        None => return json!({"error": "Missing commit message"}),
    };
    let all = args["all"].as_bool().unwrap_or(false);

    // Stage files
    if let Some(files) = args["files"].as_array() {
        for file in files {
            if let Some(f) = file.as_str() {
                let _ = Command::new("git").args(["-C", &repo, "add", f]).output();
            }
        }
    } else if all {
        let _ = Command::new("git")
            .args(["-C", &repo, "add", "-A"])
            .output();
    }

    // Commit
    let output = Command::new("git")
        .args(["-C", &repo, "commit", "-m", message])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            if o.status.success() {
                json!({
                    "success": true,
                    "message": stdout.lines().next().unwrap_or("")
                })
            } else {
                json!({
                    "success": false,
                    "error": if stderr.is_empty() { stdout } else { stderr }
                })
            }
        }
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn git_stash(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let action = args["action"].as_str().unwrap_or("push");
    let index = args["index"].as_i64().unwrap_or(0);

    let output = match action {
        "push" => {
            let mut cmd_args = vec!["-C", &repo, "stash", "push"];
            if let Some(msg) = args["message"].as_str() {
                cmd_args.push("-m");
                cmd_args.push(msg);
            }
            Command::new("git").args(&cmd_args).output()
        }
        "pop" => Command::new("git")
            .args(["-C", &repo, "stash", "pop", &format!("stash@{{{}}}", index)])
            .output(),
        "list" => Command::new("git")
            .args(["-C", &repo, "stash", "list"])
            .output(),
        "drop" => Command::new("git")
            .args([
                "-C",
                &repo,
                "stash",
                "drop",
                &format!("stash@{{{}}}", index),
            ])
            .output(),
        "show" => Command::new("git")
            .args([
                "-C",
                &repo,
                "stash",
                "show",
                "-p",
                &format!("stash@{{{}}}", index),
            ])
            .output(),
        _ => return json!({"error": format!("Unknown stash action: {}", action)}),
    };

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            json!({
                "success": o.status.success(),
                "output": if stdout.is_empty() { stderr } else { stdout }
            })
        }
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn git_reset(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let target = args["target"].as_str().unwrap_or("HEAD~1");
    let mode = args["mode"].as_str().unwrap_or("hard");

    // Get current commit first
    let before = Command::new("git")
        .args(["-C", &repo, "log", "--oneline", "-1"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let mode_flag = format!("--{}", mode);
    let output = Command::new("git")
        .args(["-C", &repo, "reset", &mode_flag, target])
        .output();

    match output {
        Ok(o) => {
            let after = Command::new("git")
                .args(["-C", &repo, "log", "--oneline", "-1"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();

            json!({
                "success": o.status.success(),
                "before": before,
                "after": after,
                "target": target,
                "mode": mode
            })
        }
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn git_diff(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let staged = args["staged"].as_bool().unwrap_or(false);
    let file = args["file"].as_str();

    let mut cmd_args = vec!["-C", &repo, "diff"];
    if staged {
        cmd_args.push("--cached");
    }
    let file_str;
    if let Some(f) = file {
        file_str = f.to_string();
        cmd_args.push(&file_str);
    }

    match Command::new("git").args(&cmd_args).output() {
        Ok(output) => {
            let diff = String::from_utf8_lossy(&output.stdout).to_string();
            if diff.is_empty() {
                json!({"diff": "", "message": "No changes"})
            } else {
                json!({"diff": diff})
            }
        }
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn git_branch(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let action = args["action"].as_str().unwrap_or("list");
    let name = args["name"].as_str();

    match action {
        "list" => {
            match Command::new("git")
                .args(["-C", &repo, "branch", "-a"])
                .output()
            {
                Ok(output) => {
                    let branches = String::from_utf8_lossy(&output.stdout).to_string();
                    json!({"branches": branches.trim()})
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        "create" => {
            let branch_name = match name {
                Some(n) => n,
                None => return json!({"error": "name required for create"}),
            };
            match Command::new("git")
                .args(["-C", &repo, "branch", branch_name])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        json!({"created": branch_name})
                    } else {
                        json!({"error": String::from_utf8_lossy(&output.stderr).to_string()})
                    }
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        "delete" => {
            let branch_name = match name {
                Some(n) => n,
                None => return json!({"error": "name required for delete"}),
            };
            match Command::new("git")
                .args(["-C", &repo, "branch", "-d", branch_name])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        json!({"deleted": branch_name})
                    } else {
                        json!({"error": String::from_utf8_lossy(&output.stderr).to_string()})
                    }
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        _ => json!({"error": format!("Unknown action: {}. Use list, create, or delete", action)}),
    }
}

fn git_checkout(args: &Value) -> Value {
    let repo = get_repo_path(args);
    let target = match args["target"].as_str() {
        Some(t) => t,
        None => return json!({"error": "target is required"}),
    };
    let create = args["create"].as_bool().unwrap_or(false);

    let mut cmd_args = vec!["-C".to_string(), repo, "checkout".to_string()];
    if create {
        cmd_args.push("-b".to_string());
    }
    cmd_args.push(target.to_string());

    match Command::new("git").args(&cmd_args).output() {
        Ok(output) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .trim()
            .to_string();
            if output.status.success() {
                json!({"switched_to": target, "output": combined})
            } else {
                json!({"error": combined})
            }
        }
        Err(e) => json!({"error": e.to_string()}),
    }
}
