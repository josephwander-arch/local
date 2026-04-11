---
name: local
version: 1.1.1
description: Windows-native MCP server — shell, files, sessions, transforms, and breadcrumb operation tracking
triggers:
  - shell
  - powershell
  - run command
  - file operations
  - persistent session
  - breadcrumb
  - track operation
  - archive
  - transform
  - bulk rename
  - csv
  - registry
  - http fetch
  - smart read
  - process management
server: local
platform: windows
audience: MCP builders shipping on Windows
---

# local MCP Server — Skill Reference (v1.1.1)

## Overview

`local` is a Windows-native MCP server that gives Claude shell access, file
operations, persistent sessions, bulk transforms, and — new in v1.1.1 — a
**breadcrumb operation tracking system** for multi-step work.

**What makes local different from Desktop Commander or Anthropic's filesystem MCP:**

| Capability | local | Desktop Commander | Anthropic filesystem |
|---|---|---|---|
| Persistent sessions (CWD + env state) | Yes | No | No |
| `smart_read` with grep/lines/max_kb | Yes | No | No |
| Breadcrumb operation tracking | Yes (v1.1.1) | No | No |
| Crash recovery for multi-step ops | Yes | No | No |
| Transform primitives (bulk rename, CSV, base64) | Yes | Partial | No |
| Archive create/extract | Yes | No | No |
| Windows registry access | Yes | No | No |

**Minimum viable shell + operation tracking for non-developer users.** If you
need to run commands, move files, and not lose your place when something
crashes — local is the server.

---

## The Breadcrumb System

This is the v1.1.1 headline feature. Breadcrumbs let you track multi-step
operations with atomic persistence, crash recovery, and bulk cleanup.

### Why Breadcrumbs Exist

Without tracking, a multi-step operation (deploy a server, migrate files,
run a build pipeline) is invisible. If Claude's context resets mid-operation,
the next session has no idea what happened. Breadcrumbs solve this:

1. **Crash recovery** — every step is atomically written to disk. A crash
   between step 3 and step 4 loses zero work. Call `breadcrumb_status` after
   restart and you see the exact resume point.
2. **Auditability** — the breadcrumb log is a durable record of what local
   did, when, and what the result was.
3. **Dashboard visibility** — the `dashboard.html` that ships with local
   renders breadcrumb history so you can see completed operations at a glance.
4. **Bulk cleanup** — `breadcrumb_clear` with `older_than_days` lets you
   prune old records without hunting through files.

### The Discipline

**Rule: 3+ planned steps → start a breadcrumb FIRST.**

```
If your operation has 3 or more steps:
  1. breadcrumb_start  — before you do anything
  2. breadcrumb_step   — after EACH step, with the result
  3. breadcrumb_complete OR breadcrumb_abort — when done or abandoned
```

Never leave orphan breadcrumbs. If you started one, you must complete or
abort it. An orphan breadcrumb signals a crash to the next session.

### The Six Core Tools

| Tool | When to use |
|---|---|
| `breadcrumb_start` | Beginning of a multi-step operation. Pass a descriptive title and list of planned steps. |
| `breadcrumb_step` | After each meaningful milestone. Pass the step name and result (success/failure + detail). |
| `breadcrumb_complete` | Operation finished successfully. Pass a summary of what was accomplished. |
| `breadcrumb_abort` | Operation abandoned or failed unrecoverably. Pass the reason. |
| `breadcrumb_status` | Mid-operation health check. Returns current breadcrumb state, completed steps, and next expected step. Use after a restart to find your place. |
| `breadcrumb_backup` | Snapshot before an irreversible step. Creates a recovery point so you can describe what to roll back to if the next step fails. |

Plus the cleanup tool:

| Tool | When to use |
|---|---|
| `breadcrumb_clear` | Bulk cleanup. Params: `older_than_days` (default 30), `force` (skip confirmation), `dry_run` (preview what would be deleted). |

### breadcrumb_status vs breadcrumb_backup

These serve different purposes:

- **`breadcrumb_status`** — "Where am I?" Read-only check. Use mid-operation
  to verify progress, or after a restart to find the resume point. Cheap,
  no side effects.
- **`breadcrumb_backup`** — "Save my place before I do something dangerous."
  Creates a named snapshot. Use before irreversible steps like deleting files,
  pushing to production, or overwriting configs. If the next step fails, the
  backup tells the recovery session exactly what state to restore.

**When to use which:**
```
Routine step (install a package, copy a file)  → just breadcrumb_step
Irreversible step (delete old deploy, push)    → breadcrumb_backup, THEN do it, THEN breadcrumb_step
Context lost / restart                         → breadcrumb_status to find resume point
```

### Auto-Start Triggers

As a safety net, local auto-creates breadcrumbs when certain tools are called
without an active breadcrumb:

- `powershell` — shell commands often start multi-step work
- `chain` — chained operations are inherently multi-step
- `psession_run` — persistent session commands imply ongoing work

Auto-created breadcrumbs are marked `auto_started: true` in their metadata.

**Important:** Auto-triggers are a safety net, not a substitute for discipline.
If a breadcrumb is already active, the auto-trigger is suppressed — you're
already tracking. But if you know you're about to do a 5-step operation,
start your own breadcrumb with a proper title and step plan. The auto-trigger
gives you a generic title; your manual breadcrumb gives you a meaningful one.

### Auto-Cleanup

Completed breadcrumbs older than 30 days are automatically pruned on local
startup. Configure retention with the `LOCAL_BREADCRUMB_RETENTION_DAYS`
environment variable:

```json
{
  "mcpServers": {
    "local": {
      "env": {
        "LOCAL_BREADCRUMB_RETENTION_DAYS": "60"
      }
    }
  }
}
```

### Worked Example: Deploy a Server Binary

Here's a realistic multi-step operation tracked from start to finish.

**Scenario:** Build and deploy `autonomous.exe` to `C:\CPC\servers\`.

```
Step 1: breadcrumb_start
  title: "Deploy autonomous.exe v2.3 | targets: C:\CPC\servers\autonomous.exe"
  steps: ["archive current binary", "build release", "copy binary", "verify size", "smoke test"]

Step 2: breadcrumb_backup
  label: "pre-deploy backup"
  note: "Current autonomous.exe is 12,308 KB, built 2026-04-10 01:06"
  → Recovery point saved. If deploy fails, restore from archive.

Step 3: archive_create
  source: "C:\CPC\servers\autonomous.exe"
  dest: "C:\CPC\servers\archive\autonomous_20260411.zip"

Step 4: breadcrumb_step
  step: "archive current binary"
  result: "archived to autonomous_20260411.zip (12,308 KB)"
  status: "success"

Step 5: run (cargo build --release)
  → Build succeeds, new binary at target\release\autonomous.exe (12,450 KB)

Step 6: breadcrumb_step
  step: "build release"
  result: "cargo build --release succeeded, 12,450 KB"
  status: "success"

Step 7: (copy binary to servers/)

Step 8: breadcrumb_step
  step: "copy binary"
  result: "copied to C:\CPC\servers\autonomous.exe"
  status: "success"

Step 9: (verify file size)

Step 10: breadcrumb_step
  step: "verify size"
  result: "12,450 KB confirmed at destination"
  status: "success"

Step 11: (run smoke test)

Step 12: breadcrumb_step
  step: "smoke test"
  result: "health endpoint returned OK in 1.2s"
  status: "success"

Step 13: breadcrumb_complete
  summary: "autonomous.exe v2.3 deployed. 12,308→12,450 KB. All 5 steps passed."
```

If Claude's context had reset between step 8 and step 9:
```
breadcrumb_status →
  title: "Deploy autonomous.exe v2.3"
  completed: ["archive current binary", "build release", "copy binary"]
  next: "verify size"
  backup: "pre-deploy backup" (pre-deploy state saved)
```

The new session picks up at "verify size" with full context of what happened.

---

## Core Concepts

### Smart Read

`smart_read` is the file reading tool that won't blow your context window.

| Parameter | What it does |
|---|---|
| `path` | File to read |
| `grep` | Return only lines matching this pattern |
| `lines` | Return a specific line range, e.g. `"10-50"` |
| `max_kb` | Cap the response size (default varies by file type) |

**Patterns:**
```
# Find a function definition in a large file
smart_read(path="src/main.rs", grep="fn handle_request")

# Read just the first 100 lines of a config
smart_read(path="config.toml", lines="1-100")

# Read a huge log without blowing context
smart_read(path="app.log", grep="ERROR", max_kb=50)
```

Always prefer `smart_read` over `read_file` for files you haven't seen before.
If the file is over 100KB and you read it raw, you just burned half your context.

### Persistent Sessions

One-shot commands (`run`, `powershell`) spin up a shell, run, and tear down.
Persistent sessions keep state:

```
session_create(name="deploy", shell="powershell")
session_run(name="deploy", command="cd C:\CPC\servers")
session_run(name="deploy", command="$env:RUST_LOG='debug'")
session_run(name="deploy", command=".\autonomous.exe --health")
# CWD and env vars persist across all three commands
session_destroy(name="deploy")
```

**When to use persistent sessions:**
- You need CWD to persist (navigating directories)
- You need environment variables to persist
- You're running a sequence of related commands
- You need to check `session_history` later

**When one-shot is fine:**
- Single independent command
- Command that doesn't depend on prior state
- Quick check (file exists, process running)

**Important:** Always `session_destroy` when done. Sessions hold resources.
If you forget, they'll accumulate until local restarts.

### Archive-First Discipline

Before replacing or deleting any file that matters:

```
archive_create(source="path/to/file", dest="path/to/archive/file_YYYYMMDD.zip")
```

This is not optional for deployments. If a new binary is broken and you
already overwrote the old one without archiving, your only recovery is
rebuilding from source — which might not even be at the same commit.

---

## Common Patterns

### Build and Deploy
```
1. breadcrumb_start (title includes binary name + target path)
2. breadcrumb_backup (snapshot current state)
3. archive_create (archive current binary)
4. run/powershell (build)                    → breadcrumb_step
5. copy new binary                           → breadcrumb_step
6. verify (size, health check)               → breadcrumb_step
7. breadcrumb_complete
```

### Bulk File Transform
```
1. breadcrumb_start (title: "bulk rename *.log → *.log.bak in /logs")
2. transform_bulk_rename (pattern, replacement, dry_run=true) → breadcrumb_step (preview)
3. Review preview
4. transform_bulk_rename (dry_run=false) → breadcrumb_step (executed)
5. breadcrumb_complete
```

### Investigate and Fix
```
1. smart_read (grep for error pattern)
2. If fix is 1-2 steps: just do it, no breadcrumb needed
3. If fix is 3+ steps: breadcrumb_start, then tracked steps
```

### Data Pipeline
```
1. http_fetch or http_download (get data)
2. transform_csv_to_json or transform_json_format (reshape)
3. write_file (output)
# If all three are needed → breadcrumb it
```

---

## Tool Reference

### Shell

| Tool | Purpose |
|---|---|
| `run` | Execute a command in a one-shot shell. Returns stdout + stderr + exit code. |
| `powershell` | Execute PowerShell specifically. Use for Windows-native operations. |
| `chain` | Run multiple commands in sequence. Stops on first failure unless `continue_on_error` is set. |
| `smart_exec` | Like `run` but with timeout, retry, and output size limits built in. |

### Files

| Tool | Purpose |
|---|---|
| `read_file` | Read a file's contents. For large files, prefer `smart_read`. |
| `smart_read` | Read with `grep`, `lines`, and `max_kb` filters. Context-safe. |
| `write_file` | Write content to a file. Creates parent directories. |
| `append_file` | Append content to an existing file. |
| `list_dir` | List directory contents with metadata. |
| `search_file` | Search for files by name pattern across directories. |
| `copy_file` | Copy a file between locations. |

### Sessions

| Tool | Purpose |
|---|---|
| `session_create` | Create a persistent shell session with a name. |
| `session_run` | Run a command in a named session (CWD + env persist). |
| `session_read_output` | Read buffered output from a session. |
| `session_history` | View command history for a session. |
| `session_cd` | Change directory in a session. |
| `session_get_env` / `session_set_env` | Read/write environment variables in a session. |
| `session_checkpoint` | Save session state for recovery. |
| `session_recover` | Restore a session from checkpoint. |
| `session_destroy` | Tear down a session and free resources. |
| `session_list` | List all active sessions. |
| `psession_create` / `psession_run` / `psession_read` / `psession_destroy` / `psession_list` | PowerShell-specific persistent sessions. Same lifecycle, PowerShell semantics. |

### Transforms

| Tool | Purpose |
|---|---|
| `transform_find_replace` | Find and replace across files. Supports regex. |
| `transform_bulk_rename` | Rename files matching a pattern. Has `dry_run` mode. |
| `transform_csv_to_json` | Convert CSV to JSON. |
| `transform_json_to_csv` | Convert JSON to CSV. |
| `transform_json_format` | Pretty-print or reformat JSON. |
| `transform_json_minify` | Minify JSON (strip whitespace). |
| `transform_base64_encode` / `transform_base64_decode` | Base64 operations. |
| `transform_extract_lines` | Extract line ranges from a file. |
| `transform_grep` | Grep across files (when you don't need `smart_read`). |
| `transform_hash_file` | Compute file hashes (SHA256, MD5, etc.). |
| `transform_file_stats` | File metadata: size, dates, permissions. |
| `transform_diff_file` | Diff two files. |
| `transform_scaffold` | Generate file/directory structures from templates. |

### Breadcrumbs

| Tool | Purpose |
|---|---|
| `breadcrumb_start` | Begin tracking a multi-step operation. |
| `breadcrumb_step` | Record a completed step with result. |
| `breadcrumb_complete` | Mark operation as successfully finished. |
| `breadcrumb_abort` | Mark operation as abandoned/failed. |
| `breadcrumb_status` | Check current breadcrumb state and progress. |
| `breadcrumb_backup` | Create a recovery snapshot before an irreversible step. |
| `breadcrumb_clear` | Bulk cleanup. Params: `older_than_days`, `force`, `dry_run`. |

### Archive

| Tool | Purpose |
|---|---|
| `archive_create` | Create a zip/tar archive from files or directories. |
| `archive_extract` | Extract an archive to a destination. |

### HTTP

| Tool | Purpose |
|---|---|
| `http_fetch` | Fetch a URL and return the body. |
| `http_download` | Download a URL to a file path. |
| `http_request` | Full HTTP request with method, headers, body. |
| `http_scrape` | Scrape a URL with content extraction. |

### Registry

| Tool | Purpose |
|---|---|
| `registry_read` | Read a Windows registry key or value. |

### System

| Tool | Purpose |
|---|---|
| `system_info` | OS version, CPU, memory, disk. |
| `list_process` | List running processes. |
| `kill_process` | Kill a process by name or PID. |
| `port_check` | Check if a port is in use. |
| `get_env` | Read an environment variable. |
| `clipboard_read` / `clipboard_write` | System clipboard access. |
| `notify` | Send a Windows notification. |

### Infrastructure

| Tool | Purpose |
|---|---|
| `server_health` | Health check for local itself. |
| `tool_fallback` | Route to alternative tool if primary fails. |
| `bag_tag` / `bag_read` / `bag_clear` | Key-value scratch storage for passing data between tools. |
| `config_validate` | Validate MCP config JSON. |
| `deploy_preflight` | Pre-deployment checks. |
| `security_check_cmd` | Validate a command before execution (safety gate). |
| `security_audit_log` | Append to the security audit trail. |

---

## The Activity Log

Local writes an activity log to `C:\CPC\logs\local_activity.jsonl`. Every
tool call is recorded with timestamp, tool name, parameters, result status,
and duration.

View it via:
- **dashboard.html** — ships with local, renders the log as a timeline
- **smart_read** — `smart_read(path="C:\\CPC\\logs\\local_activity.jsonl", grep="ERROR", max_kb=20)`
- **tail_file** — `tail_file(path="C:\\CPC\\logs\\local_activity.jsonl", lines=50)`

The log is append-only. It grows over time. Use `smart_read` with `grep`
to search it rather than reading the whole thing.

---

## Anti-Patterns

| Don't | Do Instead |
|---|---|
| Read a 500KB file with `read_file` | Use `smart_read` with `grep` or `lines` or `max_kb` |
| Run 5+ commands without a breadcrumb | `breadcrumb_start` before step 1 |
| Leave sessions open after work is done | `session_destroy` each session |
| Overwrite a binary without archiving | `archive_create` first, always |
| Use `run` for a sequence of related commands | Use `chain` or a persistent session |
| Ignore `breadcrumb_status` after restart | First thing after restart: check for active breadcrumbs |
| Use `breadcrumb_backup` for every step | Only before irreversible steps — it's not free |
| Pipe cargo output through PowerShell | PowerShell mangles cargo's stderr — use `run` with `cmd /c` |
| Paths with spaces, unquoted | Always double-quote paths: `"C:\Program Files\app.exe"` |
| Rely on auto-start breadcrumbs for planned work | Start your own — auto-start titles are generic |
| Skip `breadcrumb_abort` when abandoning | Orphan breadcrumbs look like crashes to the next session |
| Call `breadcrumb_clear(force=true)` casually | Use `dry_run=true` first to see what would be deleted |

---

## Troubleshooting

### "breadcrumb_status shows an operation I didn't start"

That's an auto-started breadcrumb from a previous `powershell`, `chain`, or
`psession_run` call. Check its steps to understand the context. If it's stale,
`breadcrumb_abort` it with a reason like "stale auto-started breadcrumb from
prior session" and start fresh.

### "Session command returns empty output"

The command may have written to stderr only. Check `session_read_output` which
captures both streams. Also verify the session still exists with `session_list`
— sessions can be destroyed by local restarts.

### "PowerShell command fails but works in a terminal"

Common causes:
- **Execution policy** — local's PowerShell may have a restricted policy.
  Try `powershell(command="Set-ExecutionPolicy -Scope Process Bypass; your-command")`.
- **Cargo/Rust output** — PowerShell doesn't handle cargo's stderr well.
  Use `run(command="cmd /c cargo build --release 2>&1")` instead.
- **Path encoding** — PowerShell uses backtick escaping. If your path has
  spaces, wrap in double quotes inside the command string.

### "smart_read returns truncated output"

That's working as intended — `max_kb` caps the response. If you need more,
increase `max_kb` or use `lines` to target the specific range. Don't remove
the cap entirely on files you haven't measured.

### "breadcrumb_clear deleted things I wanted to keep"

Always run with `dry_run=true` first. The default `older_than_days=30` only
touches completed breadcrumbs. Active and aborted breadcrumbs are never
auto-cleared — only `force=true` removes those.

### "archive_create fails on locked files"

Windows locks open files. If archiving a running server binary, stop it first,
archive, then restart. For log files that are being written to, use
`copy_file` to a temp location first, then archive the copy.

### "Local server won't start / transport errors"

1. Check if the port is in use: `port_check`
2. Check if another local instance is running: `list_process` grep for local
3. Verify the binary exists and is the right version
4. Check `server_health` — if it responds, the server is up but the MCP
   transport layer may need reconnection in your client

### "Persistent session lost state after local restart"

Sessions are in-memory. A local restart destroys all sessions. Use
`session_checkpoint` before risky operations to save state, and
`session_recover` after restart. For critical state, write it to a file
instead of relying on session persistence.
