//! Breadcrumb — multi-step operation tracking for local (mcp-windows).
//! Thin wrapper over cpc-breadcrumbs.
//!
//! Preserved from original:
//!   - startup_cleanup
//!   - breadcrumb_clear tool (local-specific active-state cleanup)
//!   - breadcrumb_list with filter param (active | archived | all)
//!   - get_definitions / execute dispatch
//!
//! Removed in v1.2.9: auto_breadcrumb_start / auto_breadcrumb_advance
//! (was auto-starting breadcrumbs for every powershell/chain/psession_run call,
//! polluting breadcrumb_list with single-step noise)
//!
//! All storage/locking/conflict/archive logic is in cpc-breadcrumbs.
// NAV: TOC at line 424 | 14 fn | 0 struct | 2026-04-15

use serde_json::{json, Value};
use cpc_breadcrumbs::WriterContext;
use std::sync::OnceLock;

// ── Per-process startup session ID ─────────────────────────────────────────────

static STARTUP_SESSION_ID: OnceLock<String> = OnceLock::new();

fn startup_session_id() -> &'static str {
    STARTUP_SESSION_ID.get_or_init(|| {
        if let Ok(v) = std::env::var("CPC_SESSION_ID") {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("sess_local_{}_{}", pid, ts)
    })
}

// ── Identity ───────────────────────────────────────────────────────────────────

fn local_ctx() -> WriterContext {
    WriterContext::new(
        std::env::var("CPC_ACTOR").unwrap_or_else(|_| "local".to_string()),
        cpc_breadcrumbs::machine_name(),
        std::env::var("CPC_SESSION_ID").unwrap_or_else(|_| startup_session_id().to_string()),
    )
}

// ── Server lifecycle ───────────────────────────────────────────────────────────

/// Call on server startup. Creates dirs, optionally reaps stale breadcrumbs.
/// Also removes local completed-ops archive files older than retention threshold
/// (env: LOCAL_BREADCRUMB_RETENTION_DAYS, default 30). Non-blocking.
pub fn startup_cleanup() {
    cpc_breadcrumbs::init();
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

fn breadcrumb_adopt(args: &Value) -> Value {
    let breadcrumb_id = match args.get("breadcrumb_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return json!({ "error": "breadcrumb_id is required" }),
    };
    let ctx = local_ctx();
    match cpc_breadcrumbs::adopt(breadcrumb_id, &ctx) {
        Ok(v) => v,
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn breadcrumb_list(args: &Value) -> Value {
    // filter: "active" | "archived" | "all"  (new in v1.2.9)
    // scope: "active" | "today" | "week" | "all"  (legacy, used when filter is absent)
    let filter = args.get("filter").and_then(|v| v.as_str());
    let scope  = args.get("scope").and_then(|v| v.as_str());

    match filter {
        Some("active") => {
            // Live active entries from Volumes\breadcrumbs\active\
            let result = cpc_breadcrumbs::status(None, Some("active"))
                .unwrap_or_else(|e| json!({ "error": e.to_string() }));
            let mut out = result;
            if let Some(bcs) = out.get_mut("breadcrumbs").and_then(|v| v.as_array_mut()) {
                for bc in bcs.iter_mut() {
                    if let Some(obj) = bc.as_object_mut() {
                        obj.insert("source".to_string(), json!("active"));
                    }
                }
            }
            out["filter"] = json!("active");
            out
        }
        Some("archived") => {
            // Archived entries from Volumes\breadcrumbs\completed\{date}\
            let eff_scope = scope.unwrap_or("today");
            let mut result = cpc_breadcrumbs::list(Some(eff_scope))
                .unwrap_or_else(|e| json!({ "error": e.to_string() }));
            if let Some(bcs) = result.get_mut("breadcrumbs").and_then(|v| v.as_array_mut()) {
                for bc in bcs.iter_mut() {
                    if let Some(obj) = bc.as_object_mut() {
                        obj.insert("source".to_string(), json!("archived"));
                    }
                }
            }
            result["filter"] = json!("archived");
            result
        }
        Some("all") => {
            // Merge active (state dir) + archived (Drive)
            let active_result = cpc_breadcrumbs::status(None, Some("active"))
                .unwrap_or_else(|e| json!({ "error": e.to_string() }));
            let eff_scope = scope.unwrap_or("today");
            let archived_result = cpc_breadcrumbs::list(Some(eff_scope))
                .unwrap_or_else(|e| json!({ "error": e.to_string() }));

            let mut combined: Vec<Value> = Vec::new();
            if let Some(bcs) = active_result.get("breadcrumbs").and_then(|v| v.as_array()) {
                for bc in bcs {
                    let mut entry = bc.clone();
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("source".to_string(), json!("active"));
                    }
                    combined.push(entry);
                }
            }
            if let Some(bcs) = archived_result.get("breadcrumbs").and_then(|v| v.as_array()) {
                for bc in bcs {
                    let mut entry = bc.clone();
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("source".to_string(), json!("archived"));
                    }
                    combined.push(entry);
                }
            }
            json!({
                "filter": "all",
                "count": combined.len(),
                "breadcrumbs": combined
            })
        }
        None => {
            // Legacy: scope param (default: today, from cpc_breadcrumbs::list)
            cpc_breadcrumbs::list(scope)
                .unwrap_or_else(|e| json!({ "error": e.to_string() }))
        }
        _ => json!({ "error": "Invalid filter value. Accepted: active | archived | all" }),
    }
}

/// Clear active breadcrumb state (Volumes active dir).
/// Does NOT touch Drive archives — those are permanent.
fn breadcrumb_clear(args: &Value) -> Value {
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

    let count = cpc_breadcrumbs::active_count();

    if dry_run {
        return json!({
            "dry_run": true,
            "active_breadcrumbs": count,
            "note": "Set dry_run=false to clear"
        });
    }

    if !force && count > 0 {
        return json!({
            "error": format!("{} active breadcrumb(s) in progress. Use force=true to clear them.", count)
        });
    }

    let mut cleared = 0u64;
    let mut errors: Vec<String> = Vec::new();

    // Abort each active breadcrumb (archives it to completed/)
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

    let mut result = json!({
        "cleared": cleared,
        "note": "Active breadcrumbs aborted and archived. Drive archives are permanent."
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
            "name": "breadcrumb_adopt",
            "description": "Reassign ownership of a breadcrumb to the current actor. Use when picking up an operation abandoned by another session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "breadcrumb_id": { "type": "string", "description": "ID of the breadcrumb to adopt" }
                },
                "required": ["breadcrumb_id"]
            }
        }),
        json!({
            "name": "breadcrumb_list",
            "description": "List breadcrumbs. Use filter param for explicit source selection. Each entry includes a source field ('active' or 'archived') when filter is set.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "active — live state dir only; archived — Drive completed archive only; all — both merged with source field. When omitted, falls through to scope param." },
                    "scope": { "type": "string", "description": "Legacy: active | today | week | all. Default: today. Used when filter is not set, or as the archive window for filter=archived|all." }
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
        "breadcrumb_adopt"    => breadcrumb_adopt(args),
        "breadcrumb_list"     => breadcrumb_list(args),
        "breadcrumb_clear"    => breadcrumb_clear(args),
        _ => json!({ "error": format!("Unknown breadcrumb tool: {}", name) }),
    }
}

// === FILE NAVIGATION ===
// 2026-04-15 | thin wrapper over cpc-breadcrumbs
// v1.2.9: removed auto_breadcrumb_start/advance; added breadcrumb_list filter param
// pub: startup_cleanup, get_definitions, execute
// private: local_ctx, breadcrumb_{start,step,complete,abort,status,backup,adopt,list,clear}

// === FILE NAVIGATION ===
// Generated: 2026-04-15T20:31:12
// Total: 421 lines | 14 functions | 0 structs | 1 constants
//
// IMPORTS: cpc_breadcrumbs, serde_json, std
//
// CONSTANTS:
//   static STARTUP_SESSION_ID: 21
//
// FUNCTIONS:
//   startup_session_id: 23-38
//   local_ctx: 42-48
//   pub +startup_cleanup: 55-57
//   breadcrumb_start: 61-75
//   breadcrumb_step: 77-91
//   breadcrumb_complete: 93-102
//   breadcrumb_abort: 104-113
//   breadcrumb_status: 115-118
//   breadcrumb_backup: 120-124
//   breadcrumb_adopt: 126-136
//   breadcrumb_list: 138-215 [med]
//   breadcrumb_clear: 219-291 [med]
//   pub +get_definitions: 295-400 [LARGE]
//   pub +execute: 402-415
//
// === END FILE NAVIGATION ===