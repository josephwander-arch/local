---
name: local-getting-started
description: 'Getting started with local — the 105-tool Windows filesystem, shell, and session server. Use when: first time using local, need file I/O, shell commands, git ops, transforms, PowerShell sessions, SQLite queries, or Windows registry access.'
---

## What Local Is

A single MCP server (local.exe) with 105 tools across 16 modules. It handles everything between your AI agent and the Windows filesystem, shell, and system.

| Module | Tools | What It Does |
|--------|-------|-------------|
| Core I/O | 22 | run, chain, read/write/append files, list dirs, processes, clipboard, archive, search |
| Sessions | 11 | Persistent shell sessions with env, history, checkpoints, recovery |
| Transforms | 14 | JSON, CSV, base64, diff, find-replace, hash, grep, scaffold |
| PowerShell | 6 | Persistent PowerShell sessions (psession_create/run/read/list/destroy/history) |
| Git | 13 | git_status, git_log, git_commit, git_stash, git_reset, git_diff, git_branch, git_checkout, git_clone, git_pull, git_push, git_remote, git_diff_summary |
| Breadcrumbs | 7 | Operation tracking: start, step, complete, abort, status, backup, clear |
| HTTP | 4 | http_request, http_fetch, http_scrape, http_download |
| Security | 2 | Command validation, audit logging |
| Shortcuts | 3 | Predefined command shortcuts with chaining |
| Smart | 2 | smart_exec (auto-pick shell), smart_read (auto-format file read) |
| Utils | 4 | Config backup, validation, markdown-to-docx |
| Bag/Tag | 3 | Key-value scratch storage for cross-tool state |
| SQLite | 1 | Direct SQLite queries |
| Registry | 1 | Windows registry reads |
| Health | 3 | Server health, tool fallback, deploy preflight |
| Plan | 2 | plan, plan_assemble |

## Key Tools

| I want to... | Use |
|--------------|-----|
| Run a shell command | run or smart_exec |
| Read/write files | read_file / write_file / append_file |
| Persistent shell session | session_create → session_run → session_read_output |
| Transform JSON/CSV | transform_json_format / transform_csv_to_json |
| Git operations | git_status, git_log, git_commit, git_clone, git_pull, git_push, git_remote |
| PowerShell with state | psession_create → psession_run → psession_read |
| Search files | search_file (by name) / transform_grep (by content) |
| Track multi-step work | breadcrumb_start → breadcrumb_step → breadcrumb_complete |
| Query a database | sqlite_query |
| Download a file | http_download |

## Common Patterns

**Run a command and get output:**
local:run(command="cargo build --release", cwd="C:/project")

**Persistent session for stateful work:**
local:session_create(name="build") → local:session_run(name="build", command="cd /project && cargo build") → local:session_read_output(name="build")

**Transform data:**
local:transform_csv_to_json(path="data.csv") or local:transform_json_format(path="config.json")

## Anti-Patterns

- Don't use `run` for PowerShell — use `powershell` or `psession_run` (proper encoding)
- Don't use `run` with pipes for complex transforms — use the transform_* tools
- Don't create throwaway sessions — reuse named sessions for related commands
