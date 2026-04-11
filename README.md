# local MCP Server

**Windows-native MCP server for shell execution, file operations, persistent sessions, transforms, and operation tracking.**

Version 1.1.1 · Apache 2.0 · [GitHub](https://github.com/josephwander-arch/local-mcp)

---

## What's New in v1.1.1: Breadcrumb Operation Tracking

The **breadcrumb subsystem** is the flagship feature of v1.1.1. It gives Claude
(and you) crash-recoverable, auditable tracking for every multi-step operation.

Seven tools — `breadcrumb_start`, `breadcrumb_step`, `breadcrumb_complete`,
`breadcrumb_abort`, `breadcrumb_status`, `breadcrumb_backup` — plus
`breadcrumb_clear` for bulk cleanup. Ported from the autonomous server and
adapted for local's shell-first workflow.

**Why this matters:**

- **Crash recovery** — every step is atomically persisted. If Claude's context
  resets mid-deploy, the next session calls `breadcrumb_status` and picks up
  exactly where it left off.
- **Auditability** — a durable log of what happened, when, and whether it
  succeeded.
- **Auto-start triggers** — `powershell`, `chain`, and `psession_run` auto-create
  breadcrumbs when none is active, so multi-step work is never invisible.
- **Auto-cleanup** — completed breadcrumbs older than 30 days are pruned on
  startup (configurable via `LOCAL_BREADCRUMB_RETENTION_DAYS`).

See [examples/breadcrumb_basics.md](examples/breadcrumb_basics.md) for a
start-to-finish walkthrough.

---

## What Makes local Different

| Capability | local | Desktop Commander | Anthropic filesystem |
|---|---|---|---|
| Persistent sessions (CWD + env state) | Yes | No | No |
| `smart_read` with grep/lines/max_kb | Yes | No | No |
| Breadcrumb operation tracking | **Yes (v1.1.1)** | No | No |
| Crash recovery for multi-step ops | Yes | No | No |
| Transform primitives (bulk rename, CSV, base64) | Yes | Partial | No |
| Archive create/extract | Yes | No | No |
| Windows registry access | Yes | No | No |

**Minimum viable shell + operation tracking for non-developer users.** If you
need to run commands, move files, and not lose your place when something
crashes — local is the server.

---

## Tool Categories

**97 tools total.** Grouped by capability:

### Shell & Execution (6 tools)
`run` · `powershell` · `chain` · `smart_exec` · `plan` · `plan_assemble`

### Sessions — standard (11 tools)
`session_create` · `session_run` · `session_read_output` · `session_history` ·
`session_cd` · `session_get_env` · `session_set_env` · `session_checkpoint` ·
`session_recover` · `session_destroy` · `session_list`

### Sessions — persistent (6 tools)
`psession_create` · `psession_run` · `psession_read` · `psession_destroy` · `psession_list` · `psession_history`

### Files (6 tools)
`read_file` · `smart_read` · `write_file` · `append_file` · `list_dir` · `search_file`

### Breadcrumbs (7 tools)
`breadcrumb_start` · `breadcrumb_step` · `breadcrumb_complete` ·
`breadcrumb_abort` · `breadcrumb_status` · `breadcrumb_backup` · `breadcrumb_clear`

### Transforms (14 tools)
`transform_find_replace` · `transform_bulk_rename` · `transform_csv_to_json` ·
`transform_json_to_csv` · `transform_json_format` · `transform_json_minify` ·
`transform_base64_encode` · `transform_base64_decode` · `transform_extract_lines` ·
`transform_grep` · `transform_hash_file` · `transform_file_stats` ·
`transform_diff_file` · `transform_scaffold`

### Archive (2 tools)
`archive_create` · `archive_extract`

### HTTP (4 tools)
`http_fetch` · `http_download` · `http_request` · `http_scrape`

### Git (8 tools)
`git_branch` · `git_checkout` · `git_commit` · `git_diff` · `git_log` ·
`git_reset` · `git_stash` · `git_status`

### WSL (4 tools)
`wsl_run` · `wsl_bg` · `wsl_log` · `wsl_status`


### System (5 tools)
`system_info` · `list_process` · `kill_process` · `port_check` · `get_env`

### Clipboard & Notifications (3 tools)
`clipboard_read` · `clipboard_write` · `notify`

### Registry (1 tool)
`registry_read`

### SQLite (1 tool)
`sqlite_query`

### Recovery (3 tools)
`recovery_clear` · `recovery_resume` · `recovery_status`

### Shortcuts (3 tools)
`shortcut_run` · `shortcut_chain` · `shortcut_list`

### Security (2 tools)
`security_check_cmd` · `security_audit_log`

### Config & Deploy (4 tools)
`config_validate` · `config_backup` · `config_backup_operating` · `deploy_preflight`

### Infrastructure (3 tools)
`server_health` · `tool_fallback` · `tail_file`

### Agent Identity — bag tools (3 tools)
`bag_tag` · `bag_read` · `bag_clear`

### Document Conversion (1 tool)
`md2docx`

---

## Install

### Prerequisites

- Windows 10/11 (x64 or ARM64)
- Claude Desktop or any MCP-compatible client
- Git (optional, used by some session tools)

### 1. Get the binary

Download `local.exe` for your architecture from the
[releases page](https://github.com/josephwander-arch/local-mcp/releases).

| Architecture | Binary |
|---|---|
| x64 | `local-x86_64-pc-windows-msvc.exe` |
| ARM64 | `local-aarch64-pc-windows-msvc.exe` |

Place it wherever you keep MCP server binaries (e.g. `C:\CPC\servers\`).

### 2. Configure Claude Desktop

Copy `claude_desktop_config.example.json` into your Claude Desktop config, or
add the `local` block to your existing `mcpServers`:

```json
{
  "mcpServers": {
    "local": {
      "command": "C:\\CPC\\servers\\local.exe",
      "args": [],
      "env": {
        "LOCAL_BREADCRUMB_RETENTION_DAYS": "30"
      }
    }
  }
}
```

### 3. Verify

Run the included health check:

```powershell
.\doctor.ps1
```

This checks that the binary exists, the state directory is writable, and git
is available.

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

---

## Compatible With

Works with any MCP client. Common install channels:

- **Claude Desktop** (the main chat app) — add to `claude_desktop_config.json`. See `claude_desktop_config.example.json` in this repo.
- **Claude Code** — add to `~/.claude/mcp.json`, or point your `CLAUDE.md` at `skills/local.md` to load it as a skill instead.
- **OpenAI Codex CLI** — register via Codex's MCP config, or load the skill directly.
- **Gemini CLI** — register via Gemini's MCP config, or load the skill directly.

**Two install layouts:**

1. **Local folder** — clone or download this repo, then point your client at the local directory or the extracted `.exe` binary.
2. **Installed binary** — grab the `.exe` from the [Releases](https://github.com/josephwander-arch/local/releases) page, place it wherever you keep your MCP binaries, then register its path in your client's config.

**Also ships as a skill** — if your client supports Anthropic skill files, load `skills/local.md` directly. Skill-only mode gives you the behavioral guidance without running the server; useful for planning, review, or read-only workflows.

### First-run tip: enable "always-loaded tools"

For the smoothest experience, enable **tools always loaded** in your Claude client settings (Claude Desktop: Settings → Tools, or equivalent in Claude Code / Codex / Gemini). This ensures Claude recognizes the tool surface on first use without needing to re-discover it every session. Most users hit friction on day one because this is off by default.

### Bootstrap the rest of the toolkit *(optional convenience)*

`local` is not a required install path — each of the other four MCP servers can be installed directly using the steps in Compatible With above. But if you already have `local` running, you can skip the manual work for the rest.

Once `local` is running, Claude can install hands, manager, echo, and workflow for you using local's shell, HTTP, archive, and file-editing tools. Ask Claude something like:

> `Install hands, manager, echo, and workflow from github.com/josephwander-arch/ and register them in my Claude Desktop config.`

Claude will use `http_download` to pull each binary from GitHub Releases, place them alongside `local.exe`, and edit `claude_desktop_config.json` for you. One manual install, four automated follow-ups.

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Copyright 2026 Joseph Wander.

---

## Donations

If local saves you time, consider supporting development:

**$NeverRemember** (Cash App)

---

## Contact

- **GitHub:** [github.com/josephwander-arch](https://github.com/josephwander-arch/)
- **Email:** protipsinc@gmail.com
- **Issues:** [github.com/josephwander-arch/local-mcp/issues](https://github.com/josephwander-arch/local-mcp/issues)
