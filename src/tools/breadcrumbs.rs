//! Breadcrumb — multi-step operation tracking for local (mcp-windows).
//! Standalone port from autonomous breadcrumb module.
//! No autonomous imports. State stored in %LOCALAPPDATA%\CPC\state\.
// NAV: TOC at line 753 | 26 fn | 2 struct | 2026-04-11

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

// ── State structs ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct StepResult {
    step: String,
    result: String,
    completed_at: String,
    files: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Operation {
    name: String,
    steps: Vec<String>,
    current_step: usize,
    total_steps: usize,
    started_at: String,
    step_results: Vec<StepResult>,
    files_changed: Vec<String>,
    #[serde(default)]
    auto_started: bool,
}

// ── Path helpers ───────────────────────────────────────────────────────────────

fn state_dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join("CPC").join("state"))
        .unwrap_or_else(|_| PathBuf::from("C:\\CPC\\state"))
}

fn active_path() -> PathBuf {
    state_dir().join("active_operation.json")
}

fn completed_dir() -> PathBuf {
    state_dir().join("completed_ops")
}

fn checkpoint_path() -> PathBuf {
    state_dir().join("breadcrumb_checkpoint.json")
}

fn log_path() -> PathBuf {
    state_dir().join("breadcrumb.jsonl")
}

fn backup_dir() -> PathBuf {
    PathBuf::from("C:\\CPC\\backups\\breadcrumbs")
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn load_active() -> Option<Operation> {
    let path = active_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    }
}

fn save_active(op: &Operation) -> Result<(), String> {
    let path = active_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create state dir: {}", e))?;
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(op)
            .map_err(|e| format!("Serialize error: {}", e))?,
    )
    .map_err(|e| format!("Failed to write active_operation.json: {}", e))
}

fn clear_active() {
    let _ = std::fs::remove_file(active_path());
}

fn save_checkpoint(op: &Operation) {
    let checkpoint = json!({
        "operation_name": op.name,
        "resume_from_step": op.current_step,
        "total_steps": op.total_steps,
        "completed_steps": op.step_results.iter().map(|s| {
            json!({"step": s.step, "result": s.result, "completed_at": s.completed_at})
        }).collect::<Vec<_>>(),
        "files_modified": op.files_changed,
        "last_checkpoint": Local::now().to_rfc3339(),
    });
    if let Ok(text) = serde_json::to_string_pretty(&checkpoint) {
        let _ = std::fs::write(checkpoint_path(), text);
    }
}

fn clear_checkpoint() {
    let _ = std::fs::remove_file(checkpoint_path());
}

fn log_event(event: &str, op: &Operation, payload: Value) {
    let entry = json!({
        "event": event,
        "name": op.name,
        "current_step": op.current_step,
        "total_steps": op.total_steps,
        "started_at": op.started_at,
        "timestamp": Local::now().to_rfc3339(),
        "payload": payload
    });
    let log = log_path();
    if let Some(parent) = log.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", entry)
        });
}

fn safe_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_sep = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            slug.push(ch);
            last_sep = false;
        } else if !last_sep {
            slug.push('_');
            last_sep = true;
        }
    }
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "operation".to_string()
    } else {
        trimmed.to_string()
    }
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

fn breadcrumb_start(args: &Value) -> Value {
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
    let steps: Vec<String> = args
        .get("steps")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if load_active().is_some() {
        return json!({"error": "Operation already in progress. Complete or abort it first."});
    }

    let op = Operation {
        name: name.to_string(),
        steps: steps.clone(),
        current_step: 0,
        total_steps: steps.len(),
        started_at: Local::now().to_rfc3339(),
        step_results: Vec::new(),
        files_changed: Vec::new(),
        auto_started: false,
    };

    if let Err(e) = save_active(&op) {
        return json!({"error": e});
    }
    log_event("start", &op, json!({"steps": steps}));

    json!({
        "status": "started",
        "name": name,
        "steps": steps,
        "total_steps": steps.len()
    })
}

fn breadcrumb_step(args: &Value) -> Value {
    let result_text = args.get("result").and_then(|v| v.as_str()).unwrap_or("");
    let files: Vec<String> = args
        .get("files_changed")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut op = match load_active() {
        Some(o) => o,
        None => return json!({"error": "No active operation"}),
    };

    let step_name = op
        .steps
        .get(op.current_step)
        .cloned()
        .unwrap_or_else(|| format!("step_{}", op.current_step + 1));

    op.step_results.push(StepResult {
        step: step_name.clone(),
        result: result_text.to_string(),
        completed_at: Local::now().to_rfc3339(),
        files: files.clone(),
    });
    op.files_changed.extend(files);
    op.current_step += 1;

    if let Err(e) = save_active(&op) {
        return json!({"error": e});
    }
    save_checkpoint(&op);
    log_event(
        "step",
        &op,
        json!({
            "step": step_name,
            "result": result_text,
            "files_changed": op.files_changed
        }),
    );

    let remaining = op.total_steps.saturating_sub(op.current_step);
    json!({
        "step_completed": step_name,
        "current": op.current_step,
        "total": op.total_steps,
        "remaining": remaining,
        "next_step": op.steps.get(op.current_step)
    })
}

fn breadcrumb_complete(args: &Value) -> Value {
    let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("");

    let mut op = match load_active() {
        Some(o) => o,
        None => return json!({"error": "No active operation"}),
    };

    let completed_at = Local::now().to_rfc3339();
    let duration_secs = chrono::DateTime::parse_from_rfc3339(&completed_at)
        .ok()
        .zip(chrono::DateTime::parse_from_rfc3339(&op.started_at).ok())
        .map(|(end, start)| (end - start).num_milliseconds() as f64 / 1000.0)
        .unwrap_or(0.0);

    // Archive to completed_ops
    let dir = completed_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return json!({"error": format!("Failed to create completed_ops dir: {}", e)});
    }
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.json", safe_slug(&op.name), ts);
    let archive = json!({
        "operation": op,
        "summary": summary,
        "completed_at": completed_at,
        "duration_secs": duration_secs
    });
    if let Ok(text) = serde_json::to_string_pretty(&archive) {
        let _ = std::fs::write(dir.join(&filename), text);
    }

    log_event(
        "complete",
        &op,
        json!({
            "summary": summary,
            "steps_completed": op.step_results.len(),
            "files_changed": op.files_changed
        }),
    );

    let files = op.files_changed.clone();
    let steps_done = op.step_results.len();
    let name = op.name.clone();

    // Suppress unused warning for mut — we need it for the clone above
    let _ = &mut op;

    clear_active();
    clear_checkpoint();

    json!({
        "status": "completed",
        "name": name,
        "steps_completed": steps_done,
        "files_changed": files,
        "duration_secs": duration_secs,
        "archived_as": filename
    })
}

fn breadcrumb_abort(args: &Value) -> Value {
    let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    let op = load_active();
    if let Some(ref active) = op {
        log_event("abort", active, json!({"reason": reason}));
    }
    clear_active();
    clear_checkpoint();

    json!({
        "status": "aborted",
        "name": op.as_ref().map(|o| o.name.as_str()).unwrap_or("none"),
        "reason": reason,
        "steps_completed": op.as_ref().map(|o| o.step_results.len()).unwrap_or(0)
    })
}

fn breadcrumb_status(_args: &Value) -> Value {
    match load_active() {
        Some(op) => {
            let last_activity = op
                .step_results
                .last()
                .map(|s| s.completed_at.clone())
                .unwrap_or_else(|| op.started_at.clone());

            let files_verified = op
                .files_changed
                .iter()
                .all(|f| std::path::Path::new(f).exists());

            let completed_summaries: Vec<Value> = op
                .step_results
                .iter()
                .map(|s| json!({"step": s.step, "result": s.result}))
                .collect();

            let remaining_steps: Vec<&String> = op.steps[op.current_step..].iter().collect();

            json!({
                "active": true,
                "name": op.name,
                "current_step": op.current_step,
                "total_steps": op.total_steps,
                "started_at": op.started_at,
                "last_activity": last_activity,
                "next_step": op.steps.get(op.current_step),
                "remaining_steps": remaining_steps,
                "completed_steps_summary": completed_summaries,
                "files_changed": op.files_changed,
                "files_verified": files_verified,
                "files_at_risk": op.files_changed,
                "recovery_available": op.current_step > 0,
                "resume_from_step": op.current_step
            })
        }
        None => json!({"active": false}),
    }
}

fn breadcrumb_backup(_args: &Value) -> Value {
    match load_active() {
        Some(op) => {
            let dir = backup_dir();
            if let Err(e) = std::fs::create_dir_all(&dir) {
                return json!({"error": format!("Failed to create backup dir: {}", e)});
            }
            let ts = Local::now().format("%Y%m%d_%H%M%S");
            let path = dir.join(format!("breadcrumb_backup_{}.json", ts));
            match serde_json::to_string_pretty(&op) {
                Ok(text) => match std::fs::write(&path, text) {
                    Ok(_) => json!({"status": "backed_up", "path": path.to_string_lossy()}),
                    Err(e) => json!({"error": format!("Failed to write backup: {}", e)}),
                },
                Err(e) => json!({"error": format!("Serialize error: {}", e)}),
            }
        }
        None => json!({"status": "nothing_to_backup"}),
    }
}

fn breadcrumb_clear(args: &Value) -> Value {
    let older_than_days = args.get("older_than_days").and_then(|v| v.as_u64());
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

    let dir = completed_dir();
    let mut cleared: u64 = 0;
    let mut bytes_freed: u64 = 0;
    let mut paths_removed: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let cutoff = older_than_days.and_then(|days| {
        std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(days * 86400))
    });

    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Some(cutoff_time) = cutoff {
                    match std::fs::metadata(&path).and_then(|m| m.modified()) {
                        Ok(modified) if modified > cutoff_time => continue,
                        Err(_) => continue,
                        _ => {}
                    }
                }
                let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let path_str = path.to_string_lossy().to_string();
                if dry_run {
                    paths_removed.push(path_str);
                    cleared += 1;
                    bytes_freed += file_size;
                } else {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {
                            paths_removed.push(path_str);
                            cleared += 1;
                            bytes_freed += file_size;
                        }
                        Err(e) => errors.push(format!("{}: {}", path_str, e)),
                    }
                }
            }
        }
    }

    let remaining = if dir.exists() {
        std::fs::read_dir(&dir)
            .ok()
            .map(|d| d.flatten().count())
            .unwrap_or(0)
    } else {
        0
    };

    let mut active_cleared = false;
    if force {
        if let Some(op) = load_active() {
            if dry_run {
                active_cleared = true;
            } else {
                log_event("abort", &op, json!({"reason": "breadcrumb_clear force=true"}));
                clear_active();
                clear_checkpoint();
                active_cleared = true;
            }
        }
    }

    let mut result = json!({
        "dry_run": dry_run,
        "cleared": cleared,
        "bytes_freed": bytes_freed,
        "remaining": remaining,
        "paths_removed": paths_removed,
    });
    if force {
        result["active_cleared"] = json!(active_cleared);
    }
    if !errors.is_empty() {
        result["errors"] = json!(errors);
    }
    result
}

// ── Public interface ──────────────────────────────────────────────────────────

/// Called on server startup. Removes archived breadcrumbs older than
/// LOCAL_BREADCRUMB_RETENTION_DAYS (default 30). Non-blocking — warns on failure.
pub fn startup_cleanup() {
    let retention_days: u64 = std::env::var("LOCAL_BREADCRUMB_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let dir = completed_dir();
    if !dir.exists() {
        return;
    }

    let cutoff = match std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(retention_days * 86400))
    {
        Some(t) => t,
        None => return,
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[breadcrumb cleanup] Cannot read completed_ops dir: {}", e);
            return;
        }
    };

    let mut removed: u64 = 0;
    let mut bytes_freed: u64 = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let modified = match std::fs::metadata(&path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if modified < cutoff {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    removed += 1;
                    bytes_freed += size;
                }
                Err(e) => eprintln!("[breadcrumb cleanup] Failed to remove {:?}: {}", path, e),
            }
        }
    }

    if removed > 0 {
        eprintln!(
            "[breadcrumb cleanup] Removed {} archive(s), {} bytes freed (retention={}d)",
            removed, bytes_freed, retention_days
        );
    }
}

/// Returns true if an active operation is currently in progress.
pub fn has_active() -> bool {
    load_active().is_some()
}

/// Auto-starts a breadcrumb for `tool` if none is active.
/// Returns true if a new breadcrumb was started, false if one already existed.
pub fn auto_breadcrumb_start(tool: &str) -> bool {
    if has_active() {
        return false;
    }
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let name = format!("auto_{}_{}", tool, ts);
    let step = format!("{} call", tool);
    let op = Operation {
        name: name.clone(),
        steps: vec![step.clone()],
        current_step: 0,
        total_steps: 1,
        started_at: Local::now().to_rfc3339(),
        step_results: Vec::new(),
        files_changed: Vec::new(),
        auto_started: true,
    };
    if save_active(&op).is_err() {
        return false;
    }
    log_event("start", &op, json!({"steps": [step], "auto_started": true}));
    true
}

/// Advances an auto-started breadcrumb and completes it (single-step auto ops self-complete).
/// No-ops if the active breadcrumb was not auto-started.
pub fn auto_breadcrumb_advance(result: &Value) {
    let mut op = match load_active() {
        Some(o) if o.auto_started => o,
        _ => return,
    };

    let step_name = op
        .steps
        .get(op.current_step)
        .cloned()
        .unwrap_or_else(|| format!("step_{}", op.current_step + 1));

    let result_text = serde_json::to_string(result).unwrap_or_else(|_| result.to_string());
    op.step_results.push(StepResult {
        step: step_name.clone(),
        result: result_text.clone(),
        completed_at: Local::now().to_rfc3339(),
        files: Vec::new(),
    });
    op.current_step += 1;

    if save_active(&op).is_err() {
        return;
    }
    save_checkpoint(&op);
    log_event("step", &op, json!({"step": step_name, "result": result_text}));

    if op.current_step >= op.total_steps {
        let dir = completed_dir();
        let _ = std::fs::create_dir_all(&dir);
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.json", safe_slug(&op.name), ts);
        let archive = json!({
            "operation": op,
            "summary": "auto-completed",
            "completed_at": Local::now().to_rfc3339(),
            "auto_started": true
        });
        if let Ok(text) = serde_json::to_string_pretty(&archive) {
            let _ = std::fs::write(dir.join(&filename), text);
        }
        log_event("complete", &op, json!({"auto_completed": true}));
        clear_active();
        clear_checkpoint();
    }
}

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "breadcrumb_start",
            "description": "Start a tracked multi-step operation. Prevents concurrent operations.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Operation name (be specific: include component and target files)"
                    },
                    "steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Ordered list of planned steps"
                    }
                },
                "required": ["name", "steps"]
            }
        }),
        json!({
            "name": "breadcrumb_step",
            "description": "Record completion of the current step and advance to the next.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "result": {
                        "type": "string",
                        "description": "What was accomplished in this step"
                    },
                    "files_changed": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Absolute paths of files modified in this step"
                    }
                },
                "required": ["result"]
            }
        }),
        json!({
            "name": "breadcrumb_complete",
            "description": "Mark the active operation as complete and archive it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Brief summary of what was accomplished"
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "breadcrumb_abort",
            "description": "Abort the active operation with a reason.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reason": {
                        "type": "string",
                        "description": "Why the operation is being aborted"
                    }
                },
                "required": ["reason"]
            }
        }),
        json!({
            "name": "breadcrumb_status",
            "description": "Get status of the current active operation.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "breadcrumb_backup",
            "description": "Backup the active operation state to C:\\CPC\\backups\\breadcrumbs\\.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "breadcrumb_clear",
            "description": "Clear completed/aborted breadcrumb archives. Use dry_run=true to preview first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "older_than_days": {
                        "type": "integer",
                        "description": "Only clear archives older than this many days. Unset = clear all."
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Also clear any active breadcrumb (dangerous). Default false."
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Report what would be cleared without actually clearing. Default false."
                    }
                },
                "required": []
            }
        }),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "breadcrumb_start"    => breadcrumb_start(args),
        "breadcrumb_step"     => breadcrumb_step(args),
        "breadcrumb_complete" => breadcrumb_complete(args),
        "breadcrumb_abort"    => breadcrumb_abort(args),
        "breadcrumb_status"   => breadcrumb_status(args),
        "breadcrumb_backup"   => breadcrumb_backup(args),
        "breadcrumb_clear"    => breadcrumb_clear(args),
        _ => json!({"error": format!("Unknown breadcrumb tool: {}", name)}),
    }
}

// === FILE NAVIGATION ===
// Generated: 2026-04-11T11:25:54
// Total: 750 lines | 26 functions | 2 structs | 0 constants
//
// IMPORTS: chrono, serde, serde_json, std
//
// STRUCTS:
//   StepResult: 13-18
//   Operation: 21-31
//
// FUNCTIONS:
//   state_dir: 35-39
//   active_path: 41-43
//   completed_dir: 45-47
//   checkpoint_path: 49-51
//   log_path: 53-55
//   backup_dir: 57-59
//   load_active: 63-72
//   save_active: 74-86
//   clear_active: 88-90
//   save_checkpoint: 92-106
//   clear_checkpoint: 108-110
//   log_event: 112-134
//   safe_slug: 136-154
//   breadcrumb_start: 158-196
//   breadcrumb_step: 198-252 [med]
//   breadcrumb_complete: 254-314 [med]
//   breadcrumb_abort: 316-331
//   breadcrumb_status: 333-374
//   breadcrumb_backup: 376-395
//   breadcrumb_clear: 397-484 [med]
//   pub +startup_cleanup: 490-546 [med]
//   pub +has_active: 549-551
//   pub +auto_breadcrumb_start: 555-577
//   pub +auto_breadcrumb_advance: 581-626
//   pub +get_definitions: 628-737 [LARGE]
//   pub +execute: 739-750
//
// === END FILE NAVIGATION ===