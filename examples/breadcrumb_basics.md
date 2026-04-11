# Breadcrumb Basics — Start / Step / Complete

This example walks through a complete breadcrumb lifecycle: tracking a
server deployment from start to finish.

## Scenario

Build and deploy `myapp.exe` to `C:\apps\`. Five steps, tracked with
breadcrumbs so that any crash is recoverable.

## Step 1: Start the Breadcrumb

```
breadcrumb_start(
  title: "deploy myapp v2.1 | targets: C:\\apps\\myapp.exe",
  steps: ["archive current", "build release", "copy binary", "verify", "smoke test"]
)
```

The title names the component (`myapp`) and declares the mutable target
(`C:\apps\myapp.exe`). This is required — vague titles like "deploy stuff"
are rejected by the `breadcrumb_start_guard` hook.

## Step 2: Backup Before Irreversible Work

```
breadcrumb_backup(
  label: "pre-deploy snapshot",
  note: "Current myapp.exe is 8,400 KB, built 2026-04-01"
)
```

This creates a recovery point. If the deploy fails, the next session knows
exactly what state to restore.

## Step 3: Archive the Current Binary

```
archive_create(
  source: "C:\\apps\\myapp.exe",
  dest: "C:\\apps\\archive\\myapp_20260411.zip"
)

breadcrumb_step(
  step: "archive current",
  result: "archived to myapp_20260411.zip",
  status: "success"
)
```

## Step 4: Build the Release

```
run(command: "cmd /c cargo build --release 2>&1")

breadcrumb_step(
  step: "build release",
  result: "cargo build succeeded, 8,600 KB",
  status: "success"
)
```

## Step 5: Copy the Binary

```
copy_file(
  source: "target\\release\\myapp.exe",
  dest: "C:\\apps\\myapp.exe"
)

breadcrumb_step(
  step: "copy binary",
  result: "copied to C:\\apps\\myapp.exe",
  status: "success"
)
```

## Step 6: Verify

```
transform_file_stats(path: "C:\\apps\\myapp.exe")
# → 8,600 KB, modified just now

breadcrumb_step(
  step: "verify",
  result: "8,600 KB confirmed",
  status: "success"
)
```

## Step 7: Smoke Test

```
run(command: "C:\\apps\\myapp.exe --health")
# → OK

breadcrumb_step(
  step: "smoke test",
  result: "health endpoint OK",
  status: "success"
)
```

## Step 8: Complete

```
breadcrumb_complete(
  summary: "myapp v2.1 deployed. 8,400→8,600 KB. All 5 steps passed."
)
```

## What If It Crashes?

If context resets between step 5 and step 6, the next session runs:

```
breadcrumb_status()
```

Response:
```
title: "deploy myapp v2.1 | targets: C:\\apps\\myapp.exe"
completed: ["archive current", "build release", "copy binary"]
next: "verify"
backup: "pre-deploy snapshot" (pre-deploy state saved)
```

Pick up at "verify" with full context.

## What If a Step Fails?

If the build fails:

```
breadcrumb_step(
  step: "build release",
  result: "cargo build failed: missing dependency",
  status: "failure"
)

breadcrumb_abort(
  reason: "build failed — missing dependency, needs manual fix"
)
```

Never leave an orphan breadcrumb. Abort it with a reason so the next session
knows what happened.
