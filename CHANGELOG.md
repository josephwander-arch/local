# Changelog

All notable changes to the local MCP server are documented here.

## [Unreleased]

## [1.2.15] - 2026-04-20

### Fixed

- **Portability fix** — all 6 source files with hardcoded `C:\My Drive\Volumes` paths now use `cpc_paths::volumes_path()` with fallback. Any user whose Google Drive mounts at a non-default path gets correct resolution instead of broken functionality.

### Changed

- **CATALOG_TOPICS stripped from dashboard.html** — removed internal knowledge-routing metadata from public HTML.
- **GitHub Actions release workflow** — `v*` tag push builds x64 + ARM64 binaries, attaches to draft release.
- **SECURITY.md** — security policy and reporting instructions.
- **Platform-split install docs** — README install section split into self-contained Windows x64 and ARM64 sub-sections.

## [1.2.14] - 2026-04-19

### Added

- **ARM64 standalone installer** (`install-local-arm64.exe`) + matching x64 installer (`install-local-x64.exe`). No new server functionality.

## [1.2.13] - 2026-04-17

### Fixed

- **Async powershell tool deadlock fix** -- replaced synchronous `std::process::Command` with `spawn` + background reader thread + `recv_timeout`, enforcing `timeout_secs` properly. Fixes 4-minute MCP deadlock on long-running child processes that previously blocked the tokio runtime.

### Changed

- **Clippy cleanup** -- removed blanket clippy suppression, applied targeted lint fixes across the codebase.

## [1.2.12] - 2026-04-17

### Added
- **5 new git tools**: `git_clone`, `git_pull`, `git_push`, `git_remote`, `git_diff_summary`. Brings local's git tool count from 8 to 13.

## [1.2.11] - 2026-04-16

### Changed
- Swapped `cpc-breadcrumbs` from path dep to git tag pin (`AIWander/cpc-breadcrumbs @ v0.1.0`) — unblocks standalone clone build (was CRITICAL-2 in pre-Stage-F audit)
- Added `license = "Apache-2.0"` + `repository` + `description` to Cargo.toml
- Committed Cargo.lock for reproducible CI builds (was CRITICAL-3 in audit)

### Fixed
- Stripped 4× stray U+009D control chars from `dashboard.html` JS string literals
- Stripped UTF-8 BOM from `src/tools/planner.rs`

### Notes
- First version of local that builds cleanly as a standalone public clone without the rust-mcp workspace.

## [1.2.9] - 2026-04-15 — HTTP body bump, breadcrumb hygiene, filter param, Apache 2.0

### Changed
- **HTTP body cap raised 100KB → 500KB** in `http_request`; fetch cap raised 50KB → 500KB in `http_fetch`.
- **Removed `auto_powershell_*` breadcrumb noise** — powershell/chain/psession_run no longer auto-start single-step breadcrumbs on every call. Breadcrumb tracking is now explicit-only.
- **`breadcrumb_list` filter param** — new `filter: "active" | "archived" | "all"`. Active entries come from live state dir; archived from Drive completed archive. Each entry includes `source: "active" | "archived"` field when filter is set.
- **Fixed `local_health.breadcrumbs.active_count` always-zero bug** — was calling `.as_array()` on the index which is an object; now delegates to `cpc_breadcrumbs::active_count()` directly.
- **Fixed `breadcrumb_start_guard.js` stale-read bug** — hook now checks `status` field and cross-references the live index before blocking; completed/aborted/archived entries no longer block new operations.
- **License changed MIT → Apache-2.0.**
- **Cargo.toml `cpc-breadcrumbs` dependency updated:** Volumes archive path now resolved via `cpc_paths::volumes_path()` with hardcoded fallback. No behavior change for existing installs.

### Added
- Two-Tier Storage section in `docs/per_machine_setup.md`.

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
- **Session ID no longer stale** — `local` now generates a
  per-process startup session ID (`sess_{server}_{pid}_{unix_ts}`) via `OnceLock`.
  State-derived session IDs (previous agent's session string) are replaced on startup.
  Override: set `CPC_SESSION_ID` env var.

### Added
- **`breadcrumb_adopt`** — Reassign ownership of a breadcrumb to the current actor.
- **`breadcrumb_list`** — List breadcrumbs from archive by scope (`active`/`today`/`week`/`all`).

## [1.2.6] - 2026-04-15 — Stage A++: cpc-breadcrumbs shared crate

### Added
- **`cpc-breadcrumbs/`** — New shared breadcrumb crate bundled as path dependency.
  Consolidates CPC's breadcrumb implementations into a single source of truth. Provides: multi-project concurrent breadcrumbs, per-project
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

The flagship feature of v1.1.1. Six breadcrumb tools adapted for local's
shell-first workflow:

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
