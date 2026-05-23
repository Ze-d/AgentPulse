# Connect Real Claude Code — Design Document

**Date:** 2026-05-23
**Status:** in-review
**Parent:** [AgentPulse v0.1 Design](2026-05-22-agentpulse-v01-design.md)

## Overview

Wire AgentPulse to real Claude Code sessions. The entire pipeline (HTTP server, state machine, SQLite, frontend) is already built. The missing piece is registering hooks in `~/.claude/settings.json` so Claude Code pushes lifecycle events to AgentPulse.

Beyond just running the installer, this work hardens both adapter scripts and adds comprehensive test coverage.

## Scope

### In scope

- Enhance `install_hooks.py`: `--dry-run`, `--status`, `--force`, idempotent install, auto-backup
- Enhance `monitor_hook.py`: stderr logging, retry on connection failure, configurable timeout, `--test` mode
- Add unit tests for both scripts (pytest, 16 test cases)
- Run installer, verify end-to-end with real Claude Code session
- Update documentation

### Out of scope

- Changing hook event types or hook protocol
- Modifying Rust event server or frontend
- Cross-platform packaging/deployment of adapters
- Hook configuration UI in the desktop app

## Design

### 1. install_hooks.py Enhancements

| Feature | Behavior |
|---------|----------|
| `--dry-run` | Print what would be written, make no changes, exit 0 |
| `--status` | Print table of 6 hook events with install status (installed/missing) |
| Idempotent install | If all 6 hooks already present, report "already installed" and exit 0 |
| `--force` | Overwrite existing hooks even if already installed |
| Backup | Before any modification, copy `settings.json` → `settings.json.bak` |
| Output | Each hook writes a clear status line: `[OK]` / `[SKIP]` / `[FAIL]` |

**Code structure refactor:**

```python
def load_settings(path: str) -> dict
def save_settings(path: str, data: dict) -> None
def build_hook_configs(monitor_script: str) -> dict
def install_hooks(settings: dict, hook_configs: dict, force: bool) -> tuple[dict, list[str]]
def remove_hooks(settings: dict) -> tuple[dict, list[str]]
def get_hook_status(settings: dict, hook_configs: dict) -> dict
def main() -> None  # argparse, dispatch to functions above
```

### 2. monitor_hook.py Enhancements

| Feature | Behavior |
|---------|----------|
| stderr logging | `logging` module, level controlled by `AGENTPULSE_LOG_LEVEL` env var |
| Retry | Up to 3 retries with 1s delay on connection errors |
| Timeout | `AGENTPULSE_TIMEOUT` env var (default 5s) |
| `--test` mode | Parse stdin, print payload to stdout, skip HTTP POST |
| Error messages | Distinguish: "stdin empty", "server unreachable", "server returned {code}" |

**Code structure refactor:**

```python
def read_stdin() -> dict | None
def send_event(url: str, data: dict, timeout: int) -> requests.Response
def main() -> None  # argparse, orchestrate read + send
```

### 3. Tests (TDD)

#### tests/unit/test_install_hooks.py

```
test_install_creates_settings_when_missing     ← target file absent
test_install_merges_with_existing_keys         ← settings.json has other config
test_install_is_idempotent                     ← repeat install is no-op
test_force_overwrites_existing_hooks           ← --force replaces
test_remove_cleans_hooks_preserves_others      ← --remove only drops hooks
test_status_reports_all_installed              ← --status when all present
test_status_reports_partial                    ← --status when some present
test_status_reports_none                       ← --status when none present
test_dry_run_does_not_modify_file              ← --dry-run leaves file intact
test_backup_created_before_modify              ← .bak created on modification
```

#### tests/unit/test_monitor_hook.py

```
test_read_stdin_returns_dict                   ← valid JSON on stdin
test_read_stdin_empty_returns_none             ← empty stdin
test_send_event_success_201                    ← server returns 201
test_send_event_server_error_500               ← server returns 500
test_send_event_retry_on_refused               ← retries on ConnectionError
test_send_event_timeout                        ← timeout raises
```

#### Existing E2E test

`tests/integration/test_e2e.py` — unchanged. Run once more after hooks are installed to confirm real pipeline.

### 4. Documentation Updates

| File | Change |
|------|--------|
| `docs/todos/connect-real-claude-code.md` | Mark complete, add install/verify steps and result |
| `docs/local-development-guide.md` | New section: "Installing Claude Code Hooks" with install/uninstall/debug commands |
| `docs/testing/testing-strategy.md` | Add "Adapter Unit Tests" row to test layers table |
| `docs/ai/context-map.md` | Add `adapters/` entries to file reference map |

## Execution Order

```
1. Write failing unit tests for install_hooks.py    ← RED
2. Refactor + enhance install_hooks.py              ← GREEN → REFACTOR
3. Write failing unit tests for monitor_hook.py     ← RED
4. Enhance monitor_hook.py                          ← GREEN → REFACTOR
5. Run full test suite (unit + E2E)                 ← VERIFY
6. Run install_hooks.py for real                    ← DEPLOY
7. Launch AgentPulse, trigger real CC events        ← SMOKE TEST
8. Update docs                                      ← DOCS
9. Commit                                           ← DONE
```

## Edge Cases

- `settings.json` doesn't exist → create it
- `settings.json` has other hooks already → merge, preserve order
- `monitor_hook.py` called but server not running → retry, then exit 1
- User runs installer twice → idempotent, no-op
- User has custom hooks with same names → `--force` to overwrite, otherwise warn
- stdin contains non-JSON → exit 1 with clear error
