//! Tool modules for MCP-Windows
// NAV: TOC at line 121 | 3 fn | 0 struct | 2026-04-15

pub mod raw;
pub mod http;
pub mod session;
pub mod transforms;
pub mod security;
pub mod shortcuts;
pub mod smart;
pub mod log;
pub mod utils;
pub mod auto_backup;
pub mod git;
pub mod health;
pub mod toc;
pub mod planner;
pub mod bagtag;
pub mod sqlite;
pub mod psession;
pub mod registry;
pub mod breadcrumbs;

use serde_json::Value;

/// Run on server startup: remove breadcrumb archives older than retention threshold.
pub fn breadcrumbs_startup_cleanup() {
    breadcrumbs::startup_cleanup();
}

/// Get all tool definitions
pub fn get_all_definitions() -> Vec<Value> {
    let mut defs = Vec::new();
    defs.extend(raw::get_definitions());
    defs.extend(http::get_definitions());
    defs.extend(session::get_definitions());
    defs.extend(transforms::get_definitions());
    defs.extend(security::get_definitions());
    defs.extend(shortcuts::get_definitions());
    defs.extend(smart::get_definitions());
    defs.extend(utils::get_definitions());
    defs.extend(git::get_definitions());
    defs.extend(health::get_definitions());
    defs.push(planner::get_definition());
    defs.extend(bagtag::get_definitions());
    defs.extend(sqlite::get_definitions());
    defs.extend(psession::get_definitions());
    defs.extend(registry::get_definitions());
    defs.extend(breadcrumbs::get_definitions());
    defs.push(serde_json::json!({"name": "plan_assemble", "description": "Enrich a plan with cross-server requirements.", "inputSchema": {"type": "object", "properties": {"plan": {"type": "object"}}, "required": ["plan"]}}));
    defs
}

/// Execute tool by name
pub fn execute(name: &str, args: &Value) -> Value {
    // Route by prefix
    if name.starts_with("raw_") {
        return raw::execute(name, args);
    }
    // Session tools
    if name.starts_with("session_") {
        return session::execute(name, args);
    }
    if name.starts_with("http_") {
        return http::execute(name, args);
    }
    if name.starts_with("transform_") {
        return transforms::execute(name, args);
    }
    if name.starts_with("security_") {
        return security::execute(name, args);
    }
    if name.starts_with("shortcut_") {
        // Shortcuts need session executor for running commands
        return shortcuts::execute(name, args, session::execute);
    }
    if name.starts_with("smart_") {
        return smart::execute(name, args);
    }
    if name.starts_with("util_") || name == "md2docx" {
        return utils::execute(name, args);
    }
    if name.starts_with("git_") {
        return git::execute(name, args);
    }
    if name == "local_health" || name.starts_with("server_") || name.starts_with("tool_fallback") || name.starts_with("preflight_") {
        return health::execute(name, args);
    }    // Standalone tools in raw module
    if name == "run" || name == "chain" || name == "read_file" || name == "read"
        || name == "write_file" || name == "write" || name == "append_file" || name == "append"
        || name == "list_dir" || name == "list_process" || name == "kill_process"
        || name == "get_env" || name == "clipboard_read" || name == "clipboard_write"
        || name == "powershell" || name == "notify"
        || name == "archive_create" || name == "archive_extract"
        || name == "search_file" || name == "search_files"
        || name == "system_info"
        || name == "recovery_status" || name == "recovery_resume" || name == "resume_operation"
        || name == "recovery_clear" || name == "clear_recovery" {
        return raw::execute(name, args);
    }
    
    
    if name.starts_with("psession_") {
        return psession::execute(name, args);
    }
    if name.starts_with("bag_") {
        return bagtag::execute(name, args);
    }
    if name == "registry_read" {
        return registry::execute(name, args);
    }
    if name == "sqlite_query" { return sqlite::execute(name, args); }
    if name == "port_check" || name == "tail_file" { return raw::execute(name, args); }
    if name == "plan" { return planner::plan(args); }
    if name == "assemble" { return planner::assemble(args); }
    if name.starts_with("breadcrumb_") { return breadcrumbs::execute(name, args); }

    serde_json::json!({"error": format!("Unknown tool: {}", name)})
}

// === FILE NAVIGATION ===
// Generated: 2026-04-15T20:31:15
// Total: 118 lines | 3 functions | 0 structs | 0 constants
//
// IMPORTS: serde_json
//
// FUNCTIONS:
//   pub +breadcrumbs_startup_cleanup: 26-28
//   pub +get_all_definitions: 31-51
//   pub +execute: 54-118 [med]
//
// === END FILE NAVIGATION ===