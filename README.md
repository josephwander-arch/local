# local MCP Server

[![CI](https://github.com/AIWander/local/actions/workflows/ci.yml/badge.svg)](https://github.com/AIWander/local/actions/workflows/ci.yml)

**Windows-native MCP server for shell execution, file operations, persistent sessions, transforms, and operation tracking.**

Version 1.2.15 Â· Apache 2.0 Â· [GitHub](https://github.com/AIWander/local)

**Part of [CPC](https://github.com/AIWander) (Copy Paste Compute)** — a multi-agent AI orchestration platform. Related repos: [manager](https://github.com/AIWander/manager) Â· [hands](https://github.com/AIWander/hands) Â· [workflow](https://github.com/AIWander/workflow) Â· [cpc-paths](https://github.com/AIWander/cpc-paths) Â· [cpc-breadcrumbs](https://github.com/AIWander/cpc-breadcrumbs)

---

## What's New in v1.2.15: Portability Fix + Standalone Uninstaller

v1.2.15 ships a path-portability fix for users whose Google Drive isn't mounted at `C:\My Drive\Volumes` — 6 source files now resolve paths via `cpc_paths::volumes_path()` with hardcoded fallback. Also adds standalone `uninstall-local-arm64.exe` + `uninstall-local-x64.exe` binaries (paired with the installer from v1.2.14).

### Highlights since v1.1.1

| Version | Headline |
|---------|----------|
| v1.2.15 | Path portability fix (cpc_paths migration) + standalone uninstaller binaries |
| v1.2.14 | ARM64 standalone installer + x64 installer |
| v1.2.13 | Async powershell deadlock fix, clippy cleanup |
| v1.2.12 | 5 new git tools: `git_clone`, `git_pull`, `git_push`, `git_remote`, `git_diff_summary` |
| v1.2.11 | First standalone public build — git deps, Cargo.lock, mojibake cleanup |
| v1.2.9 | HTTP body cap raised to 500KB, breadcrumb auto-start noise removed, `breadcrumb_list` filter param, license changed to Apache-2.0 |
| v1.2.8 | `local_health` diagnostic tool, `cpc-paths` portable path discovery |
| v1.2.7 | Identity detection fixes, `breadcrumb_adopt` + `breadcrumb_list` tools |
| v1.2.6 | `cpc-breadcrumbs` shared crate — multi-project concurrent breadcrumbs, file locking, archiving |

<details>
<summary>Full release history (v1.1.1 and earlier)</summary>

### v1.1.1 — Breadcrumb Operation Tracking

The breadcrumb subsystem was the flagship feature of v1.1.1. Seven tools —
`breadcrumb_start`, `breadcrumb_step`, `breadcrumb_complete`,
`breadcrumb_abort`, `breadcrumb_status`, `breadcrumb_backup` — plus
`breadcrumb_clear` for bulk cleanup.

- **Crash recovery** — every step is atomically persisted
- **Auto-start triggers** — `powershell`, `chain`, and `psession_run` auto-create breadcrumbs
- **Auto-cleanup** — completed breadcrumbs older than 30 days pruned on startup
- **Shipped hooks** — `breadcrumb_start_guard.js`, `breadcrumb_enforcer.js`, `post_bash.js`, `activity_log_writer.js`
- **Dashboard** — `dashboard.html` renders breadcrumb history and activity logs

### v1.1.0 — Persistent Sessions & Transforms

Persistent PowerShell sessions (`psession_*`), `smart_read` with grep/lines/max_kb,
transform primitives (bulk rename, CSV/JSON, base64, scaffolding), archive create/extract,
Windows registry read, `deploy_preflight`, security audit logging.

### v1.0.0 — Initial Release

Shell execution, file operations, persistent sessions, system tools, HTTP tools, clipboard access.

</details>

---

## What Makes local Different

| Capability | local | Desktop Commander | Anthropic filesystem |
|---|---|---|---|
| Persistent sessions (CWD + env state) | Yes | No | No |
| `smart_read` with grep/lines/max_kb | Yes | No | No |
| Breadcrumb operation tracking | Yes | No | No |
| Crash recovery for multi-step ops | Yes | No | No |
| Transform primitives (bulk rename, CSV, base64) | Yes | Partial | No |
| Archive create/extract | Yes | No | No |
| Windows registry access | Yes | No | No |

**Minimum viable shell + operation tracking for non-developer users.** If you
need to run commands, move files, and not lose your place when something
crashes — local is the server.

`local` is designed as a standalone, publicly-consumable MCP server for Windows environments. It ships as a single Rust binary with zero runtime dependencies — install it, point your MCP client at it, and you have a complete shell + filesystem + transforms toolchain.

---

## Tool Categories

**105 tools total.** Grouped by capability:

### Shell & Execution (6 tools)
`run` Â· `powershell` Â· `chain` Â· `smart_exec` Â· `plan` Â· `plan_assemble`

### Sessions — standard (11 tools)
`session_create` Â· `session_run` Â· `session_read_output` Â· `session_history` Â·
`session_cd` Â· `session_get_env` Â· `session_set_env` Â· `session_checkpoint` Â·
`session_recover` Â· `session_destroy` Â· `session_list`

### Sessions — persistent (6 tools)
`psession_create` Â· `psession_run` Â· `psession_read` Â· `psession_destroy` Â· `psession_list` Â· `psession_history`

### Files (6 tools)
`read_file` Â· `smart_read` Â· `write_file` Â· `append_file` Â· `list_dir` Â· `search_file`

### Breadcrumbs (9 tools)
`breadcrumb_start` Â· `breadcrumb_step` Â· `breadcrumb_complete` Â·
`breadcrumb_abort` Â· `breadcrumb_status` Â· `breadcrumb_backup` Â· `breadcrumb_clear` Â·
`breadcrumb_adopt` Â· `breadcrumb_list`

### Transforms (14 tools)
`transform_find_replace` Â· `transform_bulk_rename` Â· `transform_csv_to_json` Â·
`transform_json_to_csv` Â· `transform_json_format` Â· `transform_json_minify` Â·
`transform_base64_encode` Â· `transform_base64_decode` Â· `transform_extract_lines` Â·
`transform_grep` Â· `transform_hash_file` Â· `transform_file_stats` Â·
`transform_diff_file` Â· `transform_scaffold`

### Archive (2 tools)
`archive_create` Â· `archive_extract`

### HTTP (4 tools)
`http_fetch` Â· `http_download` Â· `http_request` Â· `http_scrape`

### Git (13 tools)
`git_branch` Â· `git_checkout` Â· `git_clone` Â· `git_commit` Â· `git_diff` Â·
`git_diff_summary` Â· `git_log` Â· `git_pull` Â· `git_push` Â· `git_remote` Â·
`git_reset` Â· `git_stash` Â· `git_status`

### WSL (4 tools)
`wsl_run` Â· `wsl_bg` Â· `wsl_log` Â· `wsl_status`

### System (5 tools)
`system_info` Â· `list_process` Â· `kill_process` Â· `port_check` Â· `get_env`

### Clipboard & Notifications (3 tools)
`clipboard_read` Â· `clipboard_write` Â· `notify`

### Registry (1 tool)
`registry_read`

### SQLite (1 tool)
`sqlite_query`

### Recovery (3 tools)
`recovery_clear` Â· `recovery_resume` Â· `recovery_status`

### Shortcuts (3 tools)
`shortcut_run` Â· `shortcut_chain` Â· `shortcut_list`

### Security (2 tools)
`security_check_cmd` Â· `security_audit_log`

### Config & Deploy (4 tools)
`config_validate` Â· `config_backup` Â· `config_backup_operating` Â· `deploy_preflight`

### Infrastructure (4 tools)
`server_health` Â· `local_health` Â· `tool_fallback` Â· `tail_file`

### Agent Identity — bag tools (3 tools)
`bag_tag` Â· `bag_read` Â· `bag_clear`

### Document Conversion (1 tool)
`md2docx`

---

## Install

### Windows x64

1. Download `local-v1.2.15-x64.exe` from the [latest release](https://github.com/AIWander/local/releases/latest).
2. Rename to `local.exe` and place in `%LOCALAPPDATA%\CPC\servers\`.
3. Add to your `claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "local": {
         "command": "%LOCALAPPDATA%\\CPC\\servers\\local.exe"
       }
     }
   }
   ```
4. Restart Claude Desktop.

---

### Windows ARM64

1. Download `local-v1.2.15-aarch64.exe` from the [latest release](https://github.com/AIWander/local/releases/latest).
2. Rename to `local.exe` and place in `%LOCALAPPDATA%\CPC\servers\`.
3. Add to your `claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "local": {
         "command": "%LOCALAPPDATA%\\CPC\\servers\\local.exe"
       }
     }
   }
   ```
4. Restart Claude Desktop.

---

### Prerequisites

- Windows 10/11 (x64 or ARM64)
- Claude Desktop or any MCP-compatible client
- Git (optional, used by some session tools)

For full per-machine setup (paths, breadcrumb config, git requirements), see [`docs/per_machine_setup.md`](./docs/per_machine_setup.md).

### Build from Source

```bash
git clone https://github.com/AIWander/local.git
cd local
cargo build --release
```

Binary appears at `target/release/local.exe`. Requires Rust stable toolchain — nightly is not required.

---

## Quickstart

**Run a command:**
```
run(command="echo hello world")
```

**Read a file safely:**
```
smart_read(path="C:\\big_log.txt", grep="ERROR", max_kb=20)
```

**Track a multi-step operation:**
```
breadcrumb_start(title="deploy app v2 | targets: C:\\app\\server.exe", steps=["archive", "build", "copy", "verify"])
# ... do work, calling breadcrumb_step after each ...
breadcrumb_complete(summary="deployed successfully")
```

**Persistent shell session:**
```
session_create(name="deploy", shell="powershell")
session_run(name="deploy", command="cd C:\\myapp")
session_run(name="deploy", command="dotnet build")
session_destroy(name="deploy")
```

See the [examples/](examples/) directory for more detailed walkthroughs.

---

## Shipped Hooks

The `hooks/` directory contains Claude Code hooks that enforce breadcrumb
discipline:

| Hook | Type | Purpose |
|---|---|---|
| `breadcrumb_start_guard.js` | PreToolUse | Blocks vague breadcrumb titles — requires component + targets |
| `breadcrumb_enforcer.js` | PostToolUse | Nudges for breadcrumb lifecycle: plan → start → step → complete |
| `post_bash.js` | PostToolUse | Logs all Bash commands to an audit trail |
| `activity_log_writer.js` | PostToolUse | Writes tool calls to the dashboard's activity feed |

Install by copying to your hooks directory and adding them to your Claude Code
`settings.json`. Run `doctor.ps1` to verify the state path is writable.

---

## Dashboard

`dashboard.html` is a standalone HTML file that renders breadcrumb history,
activity logs, and server status. Open it in any browser — no server required.

The `activity_log_writer.js` hook feeds tool-call data into the dashboard's
activity log in real time.

---

## Configuration

| Environment Variable | Default | Purpose |
|---|---|---|
| `LOCAL_BREADCRUMB_RETENTION_DAYS` | `30` | Auto-prune completed breadcrumbs older than N days |

---

## Compatible With

`local` is designed to work standalone — one binary, pointed at by one MCP client, and you have shell + filesystem + breadcrumbs. Pair it with other CPC servers when you want broader capabilities.

- Pair with [manager](https://github.com/AIWander/manager) when you want multi-backend orchestration on top of local's execution tools.
- Pair with [hands](https://github.com/AIWander/hands) when a script needs to reach into a browser or Windows UI layer.
- Pair with [workflow](https://github.com/AIWander/workflow) when scripts call APIs you've graduated from browser discovery to stored HTTP patterns.

Host clients: Claude Desktop (add to `claude_desktop_config.json`; see `claude_desktop_config.example.json`), Claude Code (`~/.claude/mcp.json`), OpenAI Codex CLI, or Gemini CLI. If your client supports Anthropic skill files, you can load `skills/local.md` directly for skill-only (no-server) mode — useful when you want the behavioral guidance without booting the binary.

### First-run tip for Claude clients

Toggle **tools always loaded** in Claude's tool settings (Claude Desktop: Settings → Tools). `local` exposes ~105 tools across shell, sessions, transforms, and git — clients that lazy-load occasionally miss the full set on first use. Always-loaded ensures every `local:*` tool is visible as soon as the server registers.

### Bootstrap the rest of the stack via local itself

Since `local` ships `http_download`, `write_file`, and shell execution, it's a natural installer for its siblings. Ask Claude:

> `Install hands, manager, and workflow from github.com/AIWander/ and register them in my Claude Desktop config.`

Claude uses `http_download` to pull each release binary, places them alongside `local.exe`, edits `claude_desktop_config.json`, and verifies each starts. One manual `local` install, three automated follow-ups.

## Failure modes

`local` is a thin layer over real OS operations, so failures mostly map directly to what the OS would tell you:

- **Path outside the workspace** — file tools return an explicit `path_not_allowed` error. They never silently write to an unexpected location; set your workspace root deliberately.
- **Command not found / non-zero exit** — `run`, `powershell`, and `session_run` surface the real exit code and captured stderr. Read the error rather than retrying blindly.
- **Long-running process hangs** — use `psession_*` (persistent shell) for commands that need interactive state; `run` is best for short one-shots with a hard timeout.
- **Git operation against a dirty tree** — `git_*` tools refuse destructive operations (reset --hard, force checkout) unless explicitly confirmed. Commit or stash first.
- **HTTP tools against TLS-broken hosts** — `http_*` bubbles the underlying TLS error; it does not silently fall back to insecure mode.

## Contributing

Issues welcome; PRs considered but this is primarily maintained as part of the CPC stack.

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Copyright 2026 Joseph Wander.

---

## Contact

- **GitHub:** [github.com/AIWander](https://github.com/AIWander/)
- **Email:** josephwander@gmail.com
- **Issues:** [github.com/AIWander/local/issues](https://github.com/AIWander/local/issues)
