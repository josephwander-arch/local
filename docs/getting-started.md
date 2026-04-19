---
title: "Local MCP Server — Windows Filesystem, Shell, and Session Management for AI Agents"
description: "Getting started guide for the Local Rust MCP server. Gives Claude and other AI agents 76 tools for Windows filesystem I/O, shell execution, persistent sessions, PowerShell automation, data transforms, git, SQLite, security auditing, and more over the Model Context Protocol."
keywords:
  - filesystem MCP
  - Windows MCP
  - shell MCP
  - session management
  - registry access
  - PowerShell automation
  - Windows file tools
  - MCP server
  - model context protocol server
  - rust mcp server
  - Claude Desktop MCP
  - Claude Code MCP
  - Windows shell automation
  - file transform tools
  - git MCP tools
  - SQLite MCP
---

# Getting Started with Local

Local is a Rust MCP server that provides 76 tools for Windows filesystem operations, shell execution, persistent sessions, data transforms, git, PowerShell, SQLite, security auditing, and more. It ships as a single binary with no runtime dependencies and connects to Claude Desktop, Claude Code, or any MCP-compatible client over standard JSON-RPC on stdin/stdout.

## Installation

### Prerequisites

- **Rust toolchain** (stable, 2021 edition or later) --- only needed if building from source
- **Windows 10/11** (registry and PowerShell session tools require Windows APIs)

### Build from source

```bash
git clone https://github.com/josephwander-arch/local.git
cd local
cargo build --release -p local
```

The output binary lands at `target/release/local.exe`. It is a single file with no runtime dependencies.

### Pre-built binaries

Download the latest Windows binaries from the [latest release](https://github.com/josephwander-arch/local/releases/latest):
- `local-v1.2.13-x64.exe` --- Windows x64
- `local-v1.2.13-aarch64.exe` --- Windows ARM64

### Configure for Claude Desktop

Add the server to your Claude Desktop config at `%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "local": {
      "command": "C:/path/to/local.exe",
      "args": []
    }
  }
}
```

### Configure for Claude Code

Add it to `~/.claude/mcp.json` (global) or `.mcp.json` (per-project):

```json
{
  "mcpServers": {
    "local": {
      "command": "C:/path/to/local.exe",
      "args": []
    }
  }
}
```

Restart Claude Desktop or Claude Code after editing. The 76 tools will appear in your tool list.

## Architecture Overview

```
local.exe  (MCP tool server, stdin/stdout JSON-RPC)
  |
  +-- raw (22)         Shell exec, file I/O, clipboard, archive, process mgmt
  +-- http (4)         HTTP requests, fetch, scrape, download
  +-- session (11)     Persistent shell sessions with env, checkpoints, recovery
  +-- transforms (14)  JSON/CSV/base64, diff, grep, bulk rename, scaffolding
  +-- git (4)          Status, log, commit, stash
  +-- psession (6)     Persistent PowerShell sessions
  +-- security (2)     Command validation, audit logging
  +-- shortcuts (3)    Named command shortcuts
  +-- smart (2)        Intelligent exec and read wrappers
  +-- utils (4)        Config backup, validation, md-to-docx
  +-- health (3)       Server health, fallback routing, deploy preflight
  +-- bagtag (3)       Key-value context tagging
  +-- sqlite (1)       SQLite query execution
  +-- registry (1)     Windows registry reads
  +-- breadcrumbs (7)  Operation tracking and continuity
  +-- plan (2)         Multi-step plan creation and assembly
```

All modules compile into one binary. The MCP server reads JSON-RPC requests from stdin, dispatches to the appropriate module, and returns results on stdout. No sidecar processes, no Node runtime, no Python --- just a Rust binary.

## Tool Categories and Usage Examples

Every example below shows the raw JSON-RPC call. When using Claude Desktop or Claude Code, the client builds these calls automatically from natural-language requests.

### Core I/O and Shell (22 tools)

The raw module handles file reading/writing, shell command execution, clipboard access, process management, archives, and system info.

**Run a shell command:**

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "run",
  "arguments": {"command": "dir C:\\Users\\me\\Documents"}
}}
```

**Read and write files:**

```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
  "name": "read_file",
  "arguments": {"path": "C:/project/config.json"}
}}
```

```json
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
  "name": "write_file",
  "arguments": {"path": "C:/project/output.txt", "content": "Hello from Local"}
}}
```

**Chain multiple commands** with dependency between steps:

```json
{"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
  "name": "chain",
  "arguments": {"commands": ["cd C:\\project", "git status", "cargo build --release"]}
}}
```

Other notable raw tools: `powershell` (one-shot PowerShell), `archive_create` / `archive_extract` (zip/tar), `clipboard_read` / `clipboard_write`, `search_file` (content search), `tail_file` (follow log output), `port_check`, `system_info`.

### Sessions (11 tools)

Persistent shell sessions survive across multiple tool calls. Each session maintains its own working directory, environment variables, and command history. Sessions support checkpointing and crash recovery.

**Create a session and run commands in it:**

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "session_create",
  "arguments": {"name": "build", "shell": "bash"}
}}
```

```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
  "name": "session_run",
  "arguments": {"name": "build", "command": "cargo build --release 2>&1"}
}}
```

**Checkpoint and recover:** `session_checkpoint` saves session state; `session_recover` restores it after a crash. `session_history` retrieves the full command log.

### Transforms (14 tools)

Data transformation tools for JSON, CSV, base64, file diffing, grep, bulk rename, and project scaffolding.

**Format JSON:**

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "transform_json_format",
  "arguments": {"path": "C:/data/raw.json", "indent": 2}
}}
```

**Convert CSV to JSON:**

```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
  "name": "transform_csv_to_json",
  "arguments": {"path": "C:/data/records.csv"}
}}
```

**Diff two files:**

```json
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
  "name": "transform_diff_file",
  "arguments": {"path_a": "C:/v1/config.json", "path_b": "C:/v2/config.json"}
}}
```

Other transform tools: `transform_bulk_rename`, `transform_find_replace`, `transform_hash_file`, `transform_file_stats`, `transform_extract_lines`, `transform_grep`, `transform_scaffold`.

### Git (4 tools)

Git operations without needing a separate git CLI in the path.

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "git_status",
  "arguments": {"repo": "C:/project"}
}}
```

```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
  "name": "git_commit",
  "arguments": {"repo": "C:/project", "message": "fix: resolve null check", "add_all": true}
}}
```

### PowerShell Sessions (6 tools)

Persistent PowerShell sessions for multi-step Windows automation. Unlike the one-shot `powershell` tool, these sessions keep state between calls.

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "psession_create",
  "arguments": {"name": "admin"}
}}
```

```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
  "name": "psession_run",
  "arguments": {"name": "admin", "command": "Get-Service | Where-Object {$_.Status -eq 'Running'} | Select-Object -First 5"}
}}
```

### Security (2 tools)

`security_check_cmd` validates a command against a blocklist before execution. `security_audit_log` retrieves the audit trail of all commands run through the server.

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
  "name": "security_check_cmd",
  "arguments": {"command": "rm -rf /"}
}}
```

## Common Workflows

**Build and test a project:** `session_create` a persistent session, `session_run` your build commands, `session_checkpoint` before risky steps. If something fails, `session_recover` to roll back.

**Bulk file operations:** `list_dir` to discover files, `transform_bulk_rename` to rename in batch, `transform_find_replace` for content changes across files. Use `archive_create` to back up before destructive changes.

**Data pipeline:** `http_fetch` to pull data, `transform_csv_to_json` or `transform_json_format` to reshape it, `write_file` to save the result, `sqlite_query` to load it into a database.

**Git workflow:** `git_status` to check state, `transform_diff_file` to review changes, `git_commit` to commit, `git_stash` to shelve work-in-progress.

**Windows administration:** `psession_create` for a persistent PowerShell session, `psession_run` for cmdlets, `registry_read` to inspect registry keys, `system_info` for machine details.

## Tips and Troubleshooting

**Use sessions for multi-step work.** One-shot `run` calls start a fresh shell each time. If your workflow needs a working directory, environment variables, or build state to persist, create a session instead.

**Chain vs. session.** `chain` executes a list of commands sequentially in one call but does not persist between calls. Use it for quick multi-step operations. Use sessions when state must survive across multiple tool invocations.

**PowerShell gotchas.** The one-shot `powershell` tool and `psession_run` both execute PowerShell, but `psession_run` keeps variables and imported modules alive between calls. Avoid piping cargo or rustc output through PowerShell --- it can corrupt binary output.

**Security auditing.** All commands executed through Local are logged. Use `security_audit_log` to review what ran. Use `security_check_cmd` to pre-validate commands if you are building automated pipelines.

**Recovery after crashes.** If a session dies unexpectedly, use `recovery_status` to check for recoverable sessions, then `recovery_resume` to restore them. `recovery_clear` cleans up stale recovery data.

**SQLite queries.** `sqlite_query` executes read or write SQL against any SQLite database file. Pass the database path and the SQL statement. Results come back as JSON arrays.

**Deploy preflight.** Before deploying a new binary, run `deploy_preflight` to verify the build artifact exists, the target path is writable, and no conflicting process is running.

## Further Reading

- [GitHub repository](https://github.com/josephwander-arch/local) --- source code, issues, and releases
- [Model Context Protocol specification](https://modelcontextprotocol.io/) --- the protocol Local implements
- [Claude Desktop MCP setup](https://docs.anthropic.com/en/docs/claude-desktop/mcp) --- general MCP server configuration
- [Claude Code MCP setup](https://docs.anthropic.com/en/docs/claude-code/mcp) --- adding MCP servers to Claude Code
