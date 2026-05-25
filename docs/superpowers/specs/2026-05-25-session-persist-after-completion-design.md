# Session Persist After Completion — Design Spec

## Problem

After Claude Code finishes executing (Stop event), the session card disappears from the panel within 2 seconds. Users cannot see that a session has completed; they only know it was running.

Root cause: `db.rs::list_active_sessions()` filters out `completed` and `failed` statuses.

## Goal

Session cards persist in the panel until the Claude Code terminal process is closed. Once the terminal closes, the card is removed within ~5 seconds.

## Design

### Process Association

`monitor_hook.py` walks up the Windows process tree (via `CreateToolhelp32Snapshot`
API, ctypes) to find the Claude Code (node.exe) PID, skipping intermediate shell
processes (cmd.exe, powershell.exe, etc.). The PID is injected into **every** event
payload, not just SessionStart, so sessions that miss the first event still get a PID.

### Process Survival Detection

A background thread (`process_checker.rs`, spawned in `lib.rs::run()`) polls every 5 seconds:

1. Query all sessions that have a PID via `db.list_sessions_with_pid()`
2. For each, check if the PID is alive via `sysinfo` crate (`System::refresh_processes`)
3. Delete sessions whose PID no longer exists via `db.delete_session()`

### State Machine Fix

Stop and Failure events now take priority over the `(Starting, _)` catch-all transition,
ensuring `SessionStart → Stop` correctly yields `Completed` instead of `Running`.

## Implementation Notes

**Key finding:** `os.getppid()` returns the PID of the transient shell process
(cmd.exe) that sits between Claude Code and the hook script, not the actual
node.exe PID. This caused sessions to disappear immediately because the process
checker found the shell PID dead. The fix (`_walk_process_tree_to_cc()`) uses
the Windows Toolhelp API to enumerate the parent chain and skip known shell names.

**Unchanged from design:** All 10 files modified as planned. Additional changes:
- `monitor_hook.py`: PID injected on every event (not just SessionStart)
- `state_machine.rs`: Stop/Failure reordered for correct transition priority
- `event_server.rs`: Backfill PID on existing sessions if previously null

### Data Flow

```
Claude Code starts → hook fires SessionStart
  → monitor_hook.py: _walk_process_tree_to_cc() traverses parent chain
    → Skips cmd.exe/powershell.exe → finds node.exe (CC process)
      → Injects process_pid into every event payload
        → Rust stores pid on session (backfills if null)
          → Process checker thread polls every 5s
            → PID dead? → delete session from DB
              → Next frontend poll (2s interval) → card disappears
```

### Changes

| # | File | Change |
|---|------|--------|
| 1 | `adapters/claude-code/monitor_hook.py` | Add `_walk_process_tree_to_cc()` (ctypes + Toolhelp32 API, skips shell wrappers); inject PID on every event |
| 2 | `Cargo.toml` | Add `sysinfo` dependency |
| 3 | `lib.rs` — `AgentSession` / `AgentEvent` | Add `pid: Option<u32>` / `process_pid: Option<u32>` |
| 4 | `event_server.rs` | Extract PID from every event; backfill on existing session if null |
| 5 | `db.rs` | Add `pid` column; `list_all_sessions()` (no filter); `delete_session()`; `list_sessions_with_pid()` |
| 6 | `commands.rs` | Use `list_all_sessions()` |
| 7 | `process_checker.rs` (new) | Background PID checker thread (sysinfo, 5s interval) |
| 8 | `lib.rs` — `run()` | Spawn process checker |
| 9 | `state_machine.rs` | Stop/Failure transitions reordered before `(Starting, _)` catch-all |
| 10 | `sessionStore.ts` + `agent.ts` | Show all sessions, `activeSessions` getter for header count |

### Behavior

- Terminal opens → card appears (unchanged)
- Terminal running → card updates (unchanged)
- Terminal completes → card stays, shows "done" in blue
- Terminal closes → card removed within ~5s
- Header `[N active]` counts only non-terminal sessions
