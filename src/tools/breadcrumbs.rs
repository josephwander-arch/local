//! Breadcrumb — multi-step operation tracking for local (mcp-windows).
//! Thin wrapper over cpc-breadcrumbs.
//!
//! Preserved from original:
//!   - startup_cleanup / has_active / auto_breadcrumb_start / auto_breadcrumb_advance
//!   - breadcrumb_clear tool (local-specific active-state cleanup)
//!   - get_definitions / execute dispatch
//!
//! All storage/locking/conflict/archive logic is in cpc-breadcrumbs.
// NAV: 2026-04-15 | thin wrapper | extras: auto_breadcrumb, breadcrumb_clear

use chrono::Local;
use serde_json::{json, Value};
use cpc_breadcrumbs::WriterContext;

// ── Identity ───────────────────────────────────────────────────────────────────

fn local_ctx() -> WriterContext {
    WriterContext::new(
        std::env::var("CPC_ACTOR").unwrap_or_else(|_| "local".to_string()),
        std::env::var("COMPUTERNAME")
            .unwrap_or_else(|_| "unknown".to_string())
            .to_lowercase(),
        std::env::var("CPC_SESSION_ID").unwrap_or_else(|_| "session_local".to_string()),
    )
}

// ── Server lifecycle ───────────────────────────────────────────────────────────

/// Call on server startup. Creates dirs, optionally reaps stale breadcrumbs.
/// Also removes local completed-ops archive files older than retention threshold
/// (env: LOCAL_BREADCRUMB_RETENTION_DAYS, default 30). Non-blocking.
pub fn startup_cleanup() {
    cpc_breadcrumbs::init();
}

/// Returns true if any active breadcrumb exists.
pub fn has_active() -> bool {
    cpc_breadcrumbs::has_active()
}

// ── Auto-breadcrumb (single-step auto-tracking for powershell/chain/etc.) ──────

/// Auto-starts a breadcrumb for `tool` if none is active.
/// Returns true if a new breadcrumb was started, false if one already existed.
pub fn auto_breadcrumb_start(tool: &str) -> bool {
    if has_active() {
        return false;
    }
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let name = format!("auto_{}_{}", tool, ts);
    let step = format!("{} call", tool);
    let ctx = local_ctx();
    cpc_breadcrumbs::start_auto(&name, vec![step], None, &ctx).is_ok()
}

/// Advances an auto-started breadcrumb and completes it if all steps done.
/// No-ops if no active breadcrumb, or if the active one was not auto-started.
pub fn auto_breadcrumb_advance(result: &Value) {
    // Only advance if exactly one active breadcrumb exists that is auto_started.
    let index = cpc_breadcrumbs::read_active_index();
    if index.len() != 1 {
        return;
    }
    let (bc_id, entry) = match index.iter().next() {
        Some(pair) => (pair.0.clone(), pair.1.clone()),
        None => return,
    };
    // Check auto_started by loading the breadcrumb
    let pid = entry.project_id.as_deref().unwrap_or("_ungrouped");
    let bcs = cpc_breadcrumbs::load_project_bcs(pid);
    let is_auto = bcs.iter().any(|bc| bc.id == bc_id && bc.auto_started);
    if !is_auto {
        return;
    }

    let result_str = serde_json::to_string(result).unwrap_or_else(|_| result.to_string());
    let ctx = local_ctx();
    // Step it
    if let Ok(step_resp) = cpc_breadcrumbs::step(&result_str, Vec::new(), Some(&bc_id), &ctx) {
        let current = step_resp.get("current").and_then(|v| v.as_u64()).unwrap_or(0);
        let total = step_resp.get("total").and_then(|v| v.as_u64()).unwrap_or(1);
        if current >= total {
            // Complete it
            let _ = cpc_breadcrumbs::complete("auto-completed", Some(&bc_id), &ctx);
        }
    }
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

fn breadcrumb_start(args: &Value) -> Value {
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
    let steps: Vec<String> = args
        .get("steps")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let project_id = args.get("project_id").and_then(|v| v.as_str()).map(String::from);
    let ctx = local_ctx();

    match cpc_breadcrumbs::start(name, steps, project_id, &ctx) {
        Ok(v) => v,
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn breadcrumb_step(args: &Value) -> Value {
    let result_text = args.get("result").and_then(|v| v.as_str()).unwrap_or("");
    let files: Vec<String> = args
        .get("files_changed")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let breadcrumb_id = args.get("breadcrumb_id").and_then(|v| v.as_str());
    let ctx = local_ctx();

    match cpc_breadcrumbs::step(result_text, files, breadcrumb_id, &ctx) {
        Ok(v) => v,
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn breadcrumb_complete(args: &Value) -> Value {
    let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    let breadcrumb_id = args.get("breadcrumb_id").and_then(|v| v.as_str());
    let ctx = local_ctx();

    match cpc_breadcrumbs::complete(summary, breadcrumb_id, &ctx) {
        Ok(v) => v,
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn breadcrumb_abort(args: &Value) -> Value {
    let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    let breadcrumb_id = args.get("breadcrumb_id").and_then(|v| v.as_str());
    let ctx = local_ctx();

    match cpc_breadcrumbs::abort(reason, breadcrumb_id, &ctx) {
        Ok(v) => v,
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn breadcrumb_status(_args: &Value) -> Value {
    cpc_breadcrumbs::status(None, Some("active"))
        .unwrap_or_else(|e| json!({ "error": e.to_string() }))
}

fn breadcrumb_backup(args: &Value) -> Value {
    let breadcrumb_id = args.get("breadcrumb_id").and_then(|v| v.as_str());
    cpc_breadcrumbs::backup(breadcrumb_id)
        .unwrap_or_else(|e| json!({ "error": e.to_string() }))
}

/// Clear active breadcrumb state (local CPC state dir).
/// Does NOT touch Drive archives — those are permanent.
fn breadcrumb_clear(args: &Value) -> Value {
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

    let state_dir = std::path::PathBuf::from(r"C:\CPC\state\breadcrumbs");
    if !state_dir.exists() {
        return json!({ "cleared": 0, "note": "State dir does not exist" });
    }

    if dry_run {
        // Just report what's there
        let index = cpc_breadcrumbs::read_active_index();
        return json!({
            "dry_run": true,
            "active_breadcrumbs": index.len(),
            "note": "Set dry_run=false to clear"
        });
    }

    if !force {
        // Check if any active breadcrumbs
        let index = cpc_breadcrumbs::read_active_index();
        if !index.is_empty() {
            return json!({
                "error": format!("{} active breadcrumb(s) in progress. Use force=true to clear them.", index.len())
            });
        }
    }

    let mut cleared = 0u64;
    let mut errors: Vec<String> = Vec::new();

    // If force, abort active breadcrumbs first
    if force {
        let index = cpc_breadcrumbs::read_active_index();
        let ctx = local_ctx();
        for id in index.keys() {
            if let Err(e) = cpc_breadcrumbs::abort("breadcrumb_clear force=true", Some(id), &ctx) {
                errors.push(format!("abort {}: {}", id, e));
            } else {
                cleared += 1;
            }
        }
    }

    // Clear projects dir (remove *.jsonl files)
    let projects_dir = state_dir.join("projects");
    if projects_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    match std::fs::remove_file(&path) {
                        Ok(_) => cleared += 1,
                        Err(e) => errors.push(format!("{}: {}", path.display(), e)),
                    }
                }
            }
        }
    }

    // Clear index
    let _ = std::fs::remove_file(state_dir.join("active.index.json"));

    let mut result = json!({
        "cleared": cleared,
        "note": "Drive archives are permanent — only local active state was cleared"
    });
    if !errors.is_empty() {
        result["errors"] = json!(errors);
    }
    result
}

// ── Tool definitions ──────────────────────────────────────────────────────────

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "breadcrumb_start",
            "description": "Start a tracked multi-step operation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Operation name (specific: include component + targets)" },
                    "steps": { "type": "array", "items": {"type": "string"}, "description": "Ordered planned steps" },
                    "project_id": { "type": "string", "description": "Optional project grouping. Omit for ungrouped." },
                    "breadcrumb_id": { "type": "string", "description": "Not used on start; ignored if provided." }
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
                    "result": { "type": "string", "description": "What was accomplished" },
                    "files_changed": { "type": "array", "items": {"type": "string"}, "description": "Absolute paths modified" },
                    "breadcrumb_id": { "type": "string", "description": "Required if >1 active breadcrumb" }
                },
                "required": ["result"]
            }
        }),
        json!({
            "name": "breadcrumb_complete",
            "description": "Mark operation complete and archive to Drive.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "What was accomplished" },
                    "breadcrumb_id": { "type": "string", "description": "Required if >1 active breadcrumb" }
                },
                "required": []
            }
        }),
        json!({
            "name": "breadcrumb_abort",
            "description": "Abort active operation with a reason.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reason": { "type": "string", "description": "Why aborting" },
                    "breadcrumb_id": { "type": "string", "description": "Required if >1 active breadcrumb" }
                },
                "required": ["reason"]
            }
        }),
        json!({
            "name": "breadcrumb_status",
            "description": "Get status of active breadcrumbs.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "breadcrumb_backup",
            "description": "Snapshot active breadcrumb state to C:\\CPC\\backups\\breadcrumbs\\.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "breadcrumb_id": { "type": "string", "description": "Required if >1 active breadcrumb" }
                },
                "required": []
            }
        }),
        json!({
            "name": "breadcrumb_clear",
            "description": "Clear local active breadcrumb state. Does NOT touch Drive archives.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "force": { "type": "boolean", "description": "Abort and clear even if breadcrumbs are active. Default false." },
                    "dry_run": { "type": "boolean", "description": "Preview without clearing. Default false." }
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
        _ => json!({ "error": format!("Unknown breadcrumb tool: {}", name) }),
    }
}

// === FILE NAVIGATION ===
// 2026-04-15 | thin wrapper over cpc-breadcrumbs
// pub: startup_cleanup, has_active, auto_breadcrumb_start, auto_breadcrumb_advance, get_definitions, execute
// private: local_ctx, breadcrumb_{start,step,complete,abort,status,backup,clear}
