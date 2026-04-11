# Archive Before Edit

The archive-first discipline prevents the "I overwrote the working binary
and now I have to rebuild from scratch" disaster.

## The Rule

Before overwriting or deleting any file that matters:

```
archive_create(source: "path/to/file", dest: "path/to/archive/file_YYYYMMDD.zip")
```

No exceptions for deployments, binary replacements, or config overwrites.

## Example: Replace a Config File

**Wrong:**
```
write_file(path: "C:\\apps\\config.toml", content: "new config content...")
# If the new config is broken, the old one is gone forever.
```

**Right:**
```
archive_create(
  source: "C:\\apps\\config.toml",
  dest: "C:\\apps\\archive\\config_20260411.zip"
)

write_file(path: "C:\\apps\\config.toml", content: "new config content...")
# If the new config is broken, restore from archive.
```

## Example: Deploy a Binary

```
# 1. Archive the current binary
archive_create(
  source: "C:\\CPC\\servers\\local.exe",
  dest: "C:\\CPC\\servers\\archive\\local_20260411.zip"
)

# 2. Copy the new binary
copy_file(
  source: "target\\release\\local.exe",
  dest: "C:\\CPC\\servers\\local.exe"
)

# 3. If the new binary is broken:
archive_extract(
  source: "C:\\CPC\\servers\\archive\\local_20260411.zip",
  dest: "C:\\CPC\\servers\\"
)
```

## Combining with Breadcrumbs

For multi-step operations, pair archive-first with breadcrumb tracking:

```
breadcrumb_start(
  title: "replace local.exe | targets: C:\\CPC\\servers\\local.exe",
  steps: ["archive", "copy", "verify"]
)

breadcrumb_backup(label: "pre-replace", note: "local.exe 12,308 KB")

archive_create(source: "C:\\CPC\\servers\\local.exe", dest: "...")
breadcrumb_step(step: "archive", result: "archived", status: "success")

copy_file(source: "...", dest: "C:\\CPC\\servers\\local.exe")
breadcrumb_step(step: "copy", result: "copied", status: "success")

server_health()
breadcrumb_step(step: "verify", result: "health OK", status: "success")

breadcrumb_complete(summary: "local.exe replaced successfully")
```

The breadcrumb backup + archive gives you two layers of safety:
1. The archive is the actual file backup
2. The breadcrumb backup records what state to restore if recovery is needed
