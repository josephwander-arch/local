# Changelog

All notable changes to the local MCP server are documented here.

## [1.2.8] - 2026-04-15 — Stage C-A: cpc-paths integration + local_health tool

### Added
- **`local_health` MCP tool** — diagnostic health check exposing `cpc_paths::health_check()` (path resolution status for Volumes, install, backups), active breadcrumb count, today's archived breadcrumb count, and active session count.
- **`cpc-paths` dependency** (v0.1.0) — portable path discovery library. Pinned to git tag for reproducibility.
- `session::active_count()` — public helper returning in-memory session count (used by `local_health`).

## [1.2.7] - 2026-04-15 — Stage A+++: identity detection fixes + breadcrumb_adopt/breadcrumb_list

### Fixed
- **Identity detection: no more codex bias** — `set_from_initialize()` now overrides
  stale state-derived actor with live `clientInfo.name` from the MCP initialize
  handshake. Claude Desktop (`clientInfo.name = "claude-ai"`) now correctly resolves
  to `actor = "claude"` instead of reading the previous session's `"codex"` value
  from `CPC_STATE.json`.
- **Hostname fallback for `writer_machine`** — Added `hostname = "0.4"` to
  `cpc-breadcrumbs`. New `machine_name()` helper uses
  `COMPUTERNAME → HOSTNAME → hostname::get()` syscall → `"unknown"` (true last resort).
  `local_ctx()` now calls `cpc_breadcrumbs::machine_name()` instead of bare env var.
- **Session ID no longer stale** — Both `local` and `autonomous` now generate a
  per-process startup session ID (`sess_{server}_{pid}_{unix_ts}`) via `OnceLock`.
  State-derived session IDs (previous agent's session string) are replaced on startup.
  Override: set `CPC_SESSION_ID` env var.

### Added
- **`breadcrumb_adopt`** — Reassign ownership of a breadcrumb to the current actor.
  Available in both `local` and `autonomous` servers.
- **`breadcrumb_list`** — List breadcrumbs from archive by scope (`active`/`today`/`week`/`all`).
  Available in both `local` and `autonomous` servers.

## [1.2.6] - 2026-04-15 — Stage A++: cpc-breadcrumbs shared crate

### Added
- **`cpc-breadcrumbs/`** — New shared breadcrumb crate bundled as path dependency.
  Replaces both the autonomous and local standalone implementations with a single
  source of truth. Provides: multi-project concurrent breadcrumbs, per-project
  file locking with 3s retry (fs2 flock), conflict detection (30s window,
  different writer_session → `conflict_warning` in response), Drive-synced archiving
  on complete/abort (`C:\My Drive\Volumes\breadcrumbs\completed\{date}\bc_{id}.json`),
  and auto-reap on server startup via `CPC_BREADCRUMB_AUTO_REAP_HOURS` env var.

### Changed
- **`src/tools/breadcrumbs.rs`** — Replaced 750-line standalone implementation with
  thin wrapper over `cpc_breadcrumbs`. All public functions preserved:
  `startup_cleanup`, `has_active`, `auto_breadcrumb_start`, `auto_breadcrumb_advance`,
  `get_definitions`, `execute`.
- **New tool schemas** — `breadcrumb_start` now accepts optional `project_id`.
  `breadcrumb_step`, `breadcrumb_complete`, `breadcrumb_abort`, `breadcrumb_backup`
  now accept optional `breadcrumb_id` (required only when >1 active breadcrumb).
- **Backward compatibility preserved** — Callers that pass no `project_id` get
  project `_ungrouped`. Callers that pass no `breadcrumb_id` work as long as there
  is exactly one active breadcrumb (same as before). Only errors on ambiguity
  (>1 active, no id provided).
- **`breadcrumb_clear`** — Updated to clear local active state (`C:\CPC\state\breadcrumbs\`).
  Drive archives are write-once and not cleared by this tool.

### Storage layout (new)
- Active:  `C:\CPC\state\breadcrumbs\active.index.json` + `projects\{project_id}.jsonl`
- Archive: `C:\My Drive\Volumes\breadcrumbs\completed\{YYYY-MM-DD}\bc_{id}.json`

### Environment variables
- `CPC_BREADCRUMB_AUTO_REAP_HOURS` — Set to positive integer N to auto-reap
  breadcrumbs with `last_activity_at > N hours ago` on server startup.
  Unset (default) = auto-reap disabled.

### Version
- `Cargo.toml`: version bumped to `1.2.6`

## [1.2.5] - 2026-04-15 — Post-v1.2.2 Monorepo Sync

### Changed
- **`src/tools/session.rs`** — session tool improvements; removed session_new/session_old intermediates, consolidated into final implementation
- **`src/tools/mod.rs`** — module registry updated for final session/shortcuts/smart tool set
- **`src/tools/shortcuts.rs`** — shortcut_chain and shortcut_run improvements
- **`src/tools/smart.rs`** — smart_exec / smart_read updates
- **`src/main.rs`** — tool registration + NAV index updated
- **11 additional tool files** (auto_backup, breadcrumbs, git, log, planner, psession, registry, sqlite, toc, vision, wsl) — accumulated monorepo fixes and stability improvements
- `Cargo.toml`: version bumped to `1.2.5` (post-v1.2.2 unified baseline, pre-CI)
- `windows.rs` removed (consolidated into other modules)

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
