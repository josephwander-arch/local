# Changelog

All notable changes to the local MCP server are documented here.

## [1.2.1] - 2026-04-15 — Phase C Fix3

### Changed
- **Shared dependency updates** — aligned with workspace-wide Cargo.toml changes from Phase C fix3 cycle
- **Breadcrumb and session tools** — minor stability improvements from monorepo integration

## [1.1.1] - 2026-04-11

### Added — Breadcrumb Operation Tracking Subsystem

The flagship feature of v1.1.1. Six breadcrumb tools ported from the autonomous
server and adapted for local's shell-first workflow:

- **`breadcrumb_start`** — begin tracking a multi-step operation with a
  descriptive title and planned step list
- **`breadcrumb_step`** — record each completed step with result and status
- **`breadcrumb_complete`** — mark an operation as successfully finished
- **`breadcrumb_abort`** — mark an operation as abandoned/failed with a reason
- **`breadcrumb_status`** — check current progress or find resume point after
  a restart
- **`breadcrumb_backup`** — create a recovery snapshot before irreversible steps

Plus the cleanup tool:

- **`breadcrumb_clear`** — bulk cleanup with `older_than_days`, `force`, and
  `dry_run` parameters

### Added — Multi-Step Persistence

Every breadcrumb step is atomically written to disk. A crash between step 3
and step 4 loses zero work. The next session calls `breadcrumb_status` and
sees the exact resume point.

### Added — Auto-Start Triggers

Three shell tools auto-create breadcrumbs when called without an active
breadcrumb:

- `powershell` — shell commands often start multi-step work
- `chain` — chained operations are inherently multi-step
- `psession_run` — persistent session commands imply ongoing work

Auto-created breadcrumbs are marked `auto_started: true` in metadata.

### Added — Auto-Cleanup

Completed breadcrumbs older than 30 days are automatically pruned on server
startup. Configurable via the `LOCAL_BREADCRUMB_RETENTION_DAYS` environment
variable.

### Added — Shipped Hooks

Three Claude Code hooks ship with the release:

- `breadcrumb_start_guard.js` — PreToolUse hook that blocks vague breadcrumb
  titles, requiring component + mutable targets in the name
- `breadcrumb_enforcer.js` — PostToolUse hook that nudges the breadcrumb
  lifecycle: TodoWrite plan → breadcrumb_start → breadcrumb_step → complete
- `post_bash.js` — PostToolUse hook that logs all Bash commands to an audit trail
- `activity_log_writer.js` — PostToolUse hook that writes to the dashboard
  activity feed

### Added — Dashboard

`dashboard.html` renders breadcrumb history, activity logs, and server status
in a standalone browser page. Fed by `activity_log_writer.js`.

### Added — Health Check Script

`doctor.ps1` validates binary presence, state directory writability, and git
availability.

## [1.1.0] - 2026-03-15

### Added
- Persistent PowerShell sessions (`psession_*` tools)
- `smart_read` with grep, lines, and max_kb filters
- `smart_exec` with timeout, retry, and output size limits
- Transform primitives: bulk rename, CSV/JSON conversion, base64, scaffolding
- Archive create/extract
- `chain` for sequential command execution
- `session_checkpoint` and `session_recover` for session persistence
- Windows registry read access
- `deploy_preflight` pre-deployment checks
- Security audit logging

## [1.0.0] - 2026-02-01

### Added
- Initial release
- Shell execution (`run`, `powershell`)
- File operations (`read_file`, `write_file`, `append_file`, `list_dir`, `search_file`)
- Persistent sessions (`session_create`, `session_run`, `session_destroy`)
- System tools (`system_info`, `list_process`, `kill_process`, `port_check`)
- HTTP tools (`http_fetch`, `http_download`, `http_request`)
- Clipboard access
- Bag key-value storage
