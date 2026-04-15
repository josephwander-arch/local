# Local — Per-Machine Setup

This guide covers everything you need to do on each machine where you want to run the `local` MCP server.

## Per-Machine Checklist

| Item | Per-machine? | How to set up |
|---|---|---|
| MCP binary | Yes | Download from GitHub release → `C:\CPC\servers\local.exe`. Pick right arch (`_arm64.exe` or `_x64.exe`). |
| Claude Desktop config | Yes | Edit `%APPDATA%\Claude\claude_desktop_config.json` — copy entry from `claude_desktop_config.example.json` in this repo into your `mcpServers` object. |
| Per-machine paths | Yes | Will be auto-detected by `cpc-paths` (forthcoming). For now, set env vars or hardcode in your config. See "Path Configuration" below. |
| User preferences | Yes | Open Claude Desktop → Settings → Profile → paste your preferences. (UI-only, can't script.) |
| Skills (optional) | Yes | If using CPC skills system, mirror from your Drive's `Volumes/skills/{skill}/` to `%LOCALAPPDATA%\claude-skills\{skill}\`. |
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

## Future: cpc-setup.exe (planned)

A single-binary helper that automates this entire per-machine setup is planned. It will:
- Detect platform + architecture
- Download the right MCP server binary from GitHub releases
- Auto-detect Volumes / install / backup paths and write `.cpc-config.toml`
- Mirror skills from your Drive (if using CPC skills system)
- Generate a `claude_desktop_config.json` snippet ready to paste

Until cpc-setup.exe ships, follow the manual steps above.
