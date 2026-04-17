# Local — Per-Machine Setup

This guide covers everything you need to do on each machine where you want to run the `local` MCP server.

## Per-Machine Checklist

| Item | Per-machine? | How to set up |
|---|---|---|
| MCP binary | Yes | Download from GitHub release → `C:\CPC\servers\local.exe`. Pick right arch (`_arm64.exe` or `_x64.exe`). |
| Claude Desktop config | Yes | Edit `%APPDATA%\Claude\claude_desktop_config.json` — copy entry from `claude_desktop_config.example.json` in this repo into your `mcpServers` object. |
| Per-machine paths | Yes | Will be auto-detected by `cpc-paths` (forthcoming). For now, set env vars or hardcode in your config. See "Path Configuration" below. |
| User preferences | Yes | Open Claude Desktop → Settings → Profile → paste your preferences. (UI-only, can't script.) |
| Skills (optional) | Yes | If using the skills system, mirror skill folders to `%LOCALAPPDATA%\claude-skills\{skill}\`. |
| Volumes / knowledge base | No (Drive-synced) | If Volumes is on Google Drive, just verify Drive is syncing on each machine. No copy needed. |

## Local-Specific Notes

- **Persistent shell sessions:** `local:session_create` spawns a tracked PowerShell process that survives across tool calls within a session. Environment variables and current working directory persist between commands. Use this for multi-step shell workflows instead of repeated `run` calls.
- **Git tools:** The `git_*` tools require `git` in your PATH. Install [Git for Windows](https://git-scm.com/download/win) if not already present; verify with `git --version` in PowerShell.
- **Breadcrumbs (v1.2.6+):** Live breadcrumb state is stored at `C:\CPC\state\breadcrumbs\`. Completed breadcrumbs archive to `{CPC_VOLUMES_PATH}/breadcrumbs/completed/{date}/`. Auto-reap (automatic pruning of old completed breadcrumbs) is disabled by default — enable via the `CPC_BREADCRUMB_AUTO_REAP_HOURS` env var (e.g., `168` for weekly reap).
- **Breadcrumb retention:** The `LOCAL_BREADCRUMB_RETENTION_DAYS` env var (default `30`) controls how long completed breadcrumbs are kept locally before pruning at startup.

**Test post-install:** `local:run` with `whoami` should return your Windows username cleanly with no errors.

## Path Configuration

**Coming in `cpc-paths` (next release):** automatic detection of Volumes path, install path, backups path. Auto-writes `.cpc-config.toml` on first run. Until then, paths are detected via env vars with fallbacks:

| Path | Env var | Default fallback |
|---|---|---|
| Volumes (knowledge base) | `CPC_VOLUMES_PATH` | `C:\My Drive\Volumes` (Windows) |
| Install (server binaries) | `CPC_INSTALL_PATH` | `C:\CPC\servers` (Windows) |
| Backups | `CPC_BACKUPS_PATH` | `%LOCALAPPDATA%\CPC\backups` (Windows) |

If you're on a different platform or your Drive is mounted elsewhere, set the env vars in your shell profile or system environment before launching Claude Desktop.

## Two-Tier Storage

CPC servers write data to exactly one of two tiers. Understanding the split prevents sync corruption and data loss.

### Tier 1 — Volumes (cross-machine, Drive-syncable)

| What | Example path |
|---|---|
| Knowledge base (Operating files, CATALOG.md) | `C:\My Drive\Volumes\` |
| Breadcrumb archive (completed ops) | `{Volumes}/breadcrumbs/completed/{date}/bc_*.json` |
| Handoffs, transcripts, skills | `{Volumes}/handoffs/`, `{Volumes}/skills/` |
| Workflow API patterns (planned) | `{Volumes}/api_patterns/` |

**Resolver:** `cpc_paths::volumes_path()` → env var `CPC_VOLUMES_PATH` → auto-detect → hardcoded default.

**Rule:** write-once or write-rarely. Safe for Drive sync because writes are sequential, not concurrent.

### Tier 2 — Local data (per-machine, never sync)

| What | Example path |
|---|---|
| MCP server binaries | `C:\CPC\servers\local.exe` |
| Active breadcrumb state | `C:\CPC\state\breadcrumbs\active.index.json` |
| Active session state | `C:\CPC\state\sessions\` |
| Logs (high churn) | `C:\CPC\logs\` |
| Chrome debug profile (hands server) | `C:\CPC\chrome-debug-profile\` |
| OS keyring (credentials, no path) | OS-managed |

**Rule:** anything with concurrent writes, OS file locks, executable code, or per-machine identity belongs here. **Never Drive-sync this tier.**

### Do NOT sync

- `C:\CPC\state\breadcrumbs\` — FLOCK-sensitive; concurrent-synced writes corrupt the active index
- Chrome profile — Drive sync corrupts the SQLite profile DB while Chrome holds a lock
- MCP binaries — per-arch; wrong binary on ARM64 vs x64 will crash silently

### Cross-machine setup walkthrough

To run `local` on a second machine:

1. Install Google Drive (or equivalent) and verify `C:\My Drive\Volumes\` is syncing
2. Download the right MCP binary from GitHub releases (`_arm64.exe` or `_x64.exe`)
3. Copy to `C:\CPC\servers\local.exe` on the new machine
4. Copy your `claude_desktop_config.json` entry from `claude_desktop_config.example.json`
5. Re-enter any credentials (OS keyring is per-machine — credentials do not sync)
6. Active state starts fresh on the new machine — that is correct and expected

### Legacy-path compatibility

Existing installs with data at `C:\CPC\*` continue to work unchanged. The cpc-paths library auto-detects these paths and uses them directly (legacy-fallback branch). No migration required.

## Future: cpc-setup.exe (planned)

A single-binary helper that automates this entire per-machine setup is planned. It will:
- Detect platform + architecture
- Download the right MCP server binary from GitHub releases
- Auto-detect Volumes / install / backup paths and write `.cpc-config.toml`
- Mirror skills from your Drive (if using the skills system)
- Generate a `claude_desktop_config.json` snippet ready to paste

Until cpc-setup.exe ships, follow the manual steps above.
