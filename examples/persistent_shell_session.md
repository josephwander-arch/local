# Persistent Shell Session

This example shows how to use persistent sessions to maintain working directory
and environment variables across multiple commands.

## Why Persistent Sessions?

One-shot commands (`run`, `powershell`) spin up a shell, execute, and tear down.
Every command starts fresh in the default directory with no environment state.

Persistent sessions keep CWD, environment variables, and command history across
calls — essential for multi-step workflows.

## Create a Session

```
session_create(name: "build", shell: "powershell")
```

## Navigate and Set Environment

```
session_run(name: "build", command: "cd C:\\projects\\myapp")
session_set_env(name: "build", key: "RUST_LOG", value: "debug")
session_run(name: "build", command: "echo $env:RUST_LOG")
# → debug
```

CWD and `RUST_LOG` persist for all subsequent commands in this session.

## Run a Build

```
session_run(name: "build", command: "cargo build --release 2>&1")
```

The command runs in `C:\projects\myapp` with `RUST_LOG=debug` — no need to
repeat the setup.

## Check History

```
session_history(name: "build")
```

Returns every command run in this session with timestamps.

## Checkpoint Before Risky Work

```
session_checkpoint(name: "build")
```

Saves the session state (CWD, env, history) so you can recover after a crash.

## Recover After Restart

If local restarts (sessions are in-memory):

```
session_recover(name: "build")
```

Restores from the last checkpoint. CWD and env are back.

## Clean Up

```
session_destroy(name: "build")
```

Always destroy sessions when done. They hold resources. If you forget,
they accumulate until local restarts.

## When to Use Persistent Sessions vs One-Shot

| Scenario | Use |
|---|---|
| Single independent command | `run` or `powershell` |
| CWD must persist | Persistent session |
| Env vars must persist | Persistent session |
| Sequence of related commands | Persistent session or `chain` |
| Quick check (file exists?) | `run` |

## PowerShell-Specific Sessions

For PowerShell-native workflows, use the `psession_*` tools:

```
psession_create(name: "ps-work")
psession_run(name: "ps-work", command: "Get-ChildItem C:\\apps")
psession_read(name: "ps-work")
psession_destroy(name: "ps-work")
```

Same lifecycle, PowerShell semantics. Note: `psession_run` auto-triggers
a breadcrumb if none is active.
