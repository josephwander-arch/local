# local MCP Server — Recommended CLAUDE.md Instructions

Copy the block below into your `CLAUDE.md` or `~/.claude/CLAUDE.md`.

---

```markdown
## local MCP Server Discipline

### Breadcrumb Protocol (mandatory for 3+ step operations)
- **Before step 1:** `breadcrumb_start` with a descriptive title and planned step list
- **After each step:** `breadcrumb_step` with step name, result, and status
- **Before irreversible steps:** `breadcrumb_backup` to create a recovery point
- **When finished:** `breadcrumb_complete` with a summary
- **When abandoning:** `breadcrumb_abort` with a reason — never leave orphans
- **After restart:** `breadcrumb_status` first to check for in-progress operations

Auto-start triggers (powershell, chain, psession_run) are a safety net.
Start your own breadcrumbs for planned work — auto-start titles are generic.

### Archive-First Rule
Before overwriting or deleting any file that matters:
```
archive_create(source="path/to/file", dest="path/to/archive/file_YYYYMMDD.zip")
```
No exceptions for deployments, binary replacements, or config overwrites.

### smart_read Over read_file
For files you haven't seen or files over 50KB:
- `smart_read(path, grep="pattern")` — search without reading everything
- `smart_read(path, lines="1-100")` — read a range
- `smart_read(path, max_kb=30)` — cap output size

Never `read_file` on an unknown large file. You'll burn context.

### Persistent Sessions
- Use persistent sessions when CWD or env vars must persist across commands
- Always `session_destroy` when done — sessions hold resources
- Use `session_checkpoint` before risky operations for recovery

### Shell Gotchas
- PowerShell can't cleanly pipe cargo output — use `run(command="cmd /c cargo build 2>&1")`
- Always double-quote paths with spaces
- Use `chain` for sequential commands, not manual run-run-run
- `psession_run` auto-triggers a breadcrumb if none is active

### Cleanup
- `breadcrumb_clear(dry_run=true)` before `breadcrumb_clear(force=true)`
- Completed breadcrumbs auto-prune after 30 days (configurable via LOCAL_BREADCRUMB_RETENTION_DAYS)
```

---

**What this gives you:** The breadcrumb discipline turns local from "a bag of
shell tools" into a tracked operations system. Every multi-step operation is
recoverable, auditable, and visible in the dashboard. The archive-first rule
prevents the "I overwrote the working binary and now I have to rebuild from
scratch" disaster. The smart_read rule prevents the "I read a 2MB log and now
I have 3K tokens left" disaster.
