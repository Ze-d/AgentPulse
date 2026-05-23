# Connect Real Claude Code — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor and harden both adapter scripts (install_hooks.py, monitor_hook.py), add comprehensive unit tests via TDD, install hooks to real `~/.claude/settings.json`, verify end-to-end with a live Claude Code session, and update all documentation.

**Architecture:** Two independent Python scripts in `adapters/claude-code/` — install_hooks.py manages hook configuration in `~/.claude/settings.json`, monitor_hook.py reads hook JSON from stdin and POSTs to the AgentPulse event server. Both are refactored into testable pure functions + thin I/O wrappers. Tests use pytest with tmp_path fixtures (install) and unittest.mock patches (monitor).

**Tech Stack:** Python 3, pytest, argparse, logging, urllib.request, json

---

## File Map

| File | Responsibility |
|------|---------------|
| `adapters/claude-code/install_hooks.py` | Install/uninstall/status/dry-run hooks in settings.json |
| `adapters/claude-code/monitor_hook.py` | Read hook JSON from stdin, POST to event server with retry |
| `tests/unit/__init__.py` | Unit test package marker |
| `tests/unit/test_install_hooks.py` | Unit tests for install_hooks.py functions (10 cases) |
| `tests/unit/test_monitor_hook.py` | Unit tests for monitor_hook.py functions (8 cases) |
| `tests/integration/test_e2e.py` | E2E smoke test (existing, no changes) |
| `docs/todos/connect-real-claude-code.md` | Update status to complete with verification results |
| `docs/local-development-guide.md` | Add hooks install/uninstall/debug section |
| `docs/testing/testing-strategy.md` | Add adapter unit test layer |
| `docs/ai/context-map.md` | Add adapters/ entries |

---

### Task 1: Write failing unit tests for install_hooks.py (RED)

**Files:**
- Create: `tests/unit/__init__.py`
- Create: `tests/unit/test_install_hooks.py`

- [ ] **Step 1: Create unit test package init**

```python
# tests/unit/__init__.py
```

- [ ] **Step 2: Write all 10 install_hooks unit tests**

```python
# tests/unit/test_install_hooks.py
import json
import pytest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "adapters" / "claude-code"))
import install_hooks


class TestBuildHookConfigs:
    def test_returns_dict_with_all_events(self):
        configs = install_hooks.build_hook_configs("/path/to/monitor.py")
        for event in install_hooks.HOOK_EVENTS:
            assert event in configs
            assert configs[event][0]["matcher"] == ""
            command = configs[event][0]["hooks"][0]["command"]
            assert command == "python /path/to/monitor.py"


class TestLoadSaveSettings:
    def test_load_returns_empty_when_missing(self, tmp_path):
        path = tmp_path / "nonexistent.json"
        result = install_hooks.load_settings(path)
        assert result == {}

    def test_load_returns_data_when_exists(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text('{"key": "value"}')
        result = install_hooks.load_settings(path)
        assert result == {"key": "value"}

    def test_save_creates_file_and_parent_dir(self, tmp_path):
        path = tmp_path / "subdir" / "settings.json"
        install_hooks.save_settings(path, {"hooks": {}})
        assert path.exists()
        data = json.loads(path.read_text())
        assert data == {"hooks": {}}


class TestMergeHooks:
    def test_adds_hooks_to_empty_settings(self):
        settings = {}
        hook_configs = {"SessionStart": [{"matcher": ""}]}
        result = install_hooks.merge_hooks(settings, hook_configs)
        assert result["hooks"] == hook_configs

    def test_preserves_existing_non_hook_keys(self):
        settings = {"permissions": {"allow": ["Bash(git *)"]}}
        hook_configs = {"SessionStart": [{"matcher": ""}]}
        result = install_hooks.merge_hooks(settings, hook_configs)
        assert result["permissions"] == {"allow": ["Bash(git *)"]}
        assert "SessionStart" in result["hooks"]

    def test_does_not_modify_original(self):
        settings = {"hooks": {"Stop": []}}
        original = json.dumps(settings)
        hook_configs = {"SessionStart": [{"matcher": ""}]}
        install_hooks.merge_hooks(settings, hook_configs)
        assert json.dumps(settings) == original


class TestRemoveHooksFromSettings:
    def test_removes_all_hook_events(self):
        settings = {
            "hooks": {
                "SessionStart": [{}],
                "Stop": [{}],
                "CustomThing": [{}],
            }
        }
        result = install_hooks.remove_hooks_from_settings(settings)
        assert "SessionStart" not in result["hooks"]
        assert "Stop" not in result["hooks"]
        assert "CustomThing" in result["hooks"]

    def test_handles_no_hooks_key(self):
        settings = {"env": {}}
        result = install_hooks.remove_hooks_from_settings(settings)
        assert "hooks" in result

    def test_does_not_modify_original(self):
        settings = {"hooks": {"SessionStart": [{}]}}
        original = json.dumps(settings)
        install_hooks.remove_hooks_from_settings(settings)
        assert json.dumps(settings) == original


class TestGetHookStatus:
    def test_all_installed(self):
        settings = {"hooks": {e: [{}] for e in install_hooks.HOOK_EVENTS}}
        status = install_hooks.get_hook_status(settings)
        assert all(status.values())

    def test_none_installed(self):
        settings = {}
        status = install_hooks.get_hook_status(settings)
        assert not any(status.values())

    def test_partial_installed(self):
        settings = {"hooks": {"SessionStart": [{}]}}
        status = install_hooks.get_hook_status(settings)
        assert status["SessionStart"] is True
        assert status["Stop"] is False


class TestInstall:
    def test_creates_settings_when_missing(self, tmp_path):
        path = tmp_path / "settings.json"
        result = install_hooks.install(path)
        assert result == "installed"
        assert path.exists()
        data = json.loads(path.read_text())
        for event in install_hooks.HOOK_EVENTS:
            assert event in data["hooks"]

    def test_merges_with_existing_keys(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text('{"env": {"FOO": "bar"}}')
        result = install_hooks.install(path)
        assert result == "installed"
        data = json.loads(path.read_text())
        assert data["env"] == {"FOO": "bar"}
        assert "SessionStart" in data["hooks"]

    def test_is_idempotent(self, tmp_path):
        path = tmp_path / "settings.json"
        install_hooks.install(path)
        result = install_hooks.install(path)
        assert result == "already_installed"

    def test_force_overwrites(self, tmp_path):
        path = tmp_path / "settings.json"
        install_hooks.install(path)
        result = install_hooks.install(path, force=True)
        assert result == "installed"

    def test_backup_created_before_modify(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text('{"env": {"FOO": "bar"}}')
        install_hooks.install(path)
        backup = tmp_path / "settings.json.bak"
        assert backup.exists()
        backup_data = json.loads(backup.read_text())
        assert backup_data == {"env": {"FOO": "bar"}}


class TestRemove:
    def test_cleans_hooks_preserves_others(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text(
            '{"hooks": {"SessionStart": [{}], "CustomHook": [{}]}, "env": {"X": "1"}}'
        )
        install_hooks.remove_hooks(path)
        data = json.loads(path.read_text())
        assert "SessionStart" not in data["hooks"]
        assert "CustomHook" in data["hooks"]
        assert data["env"] == {"X": "1"}

    def test_no_settings_file(self, tmp_path):
        path = tmp_path / "nonexistent.json"
        result = install_hooks.remove_hooks(path)
        assert result == "no_settings_file"

    def test_backup_created(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text('{"hooks": {"SessionStart": [{}]}}')
        install_hooks.remove_hooks(path)
        assert (tmp_path / "settings.json.bak").exists()


class TestDryRun:
    def test_does_not_modify_file(self, tmp_path):
        path = tmp_path / "settings.json"
        path.write_text('{"env": {"FOO": "bar"}}')
        before = path.read_text()
        info = install_hooks.dry_run(path)
        after = path.read_text()
        assert before == after
        assert len(info["hooks_to_install"]) == len(install_hooks.HOOK_EVENTS)
```

- [ ] **Step 3: Run tests to verify they fail (RED)**

```powershell
python -m pytest tests/unit/test_install_hooks.py -v
```

Expected: FAIL — `AttributeError: module 'install_hooks' has no attribute 'build_hook_configs'` (or similar)

- [ ] **Step 4: Commit**

```bash
git add tests/unit/__init__.py tests/unit/test_install_hooks.py
git commit -m "test: add failing unit tests for install_hooks (RED)"
```

---

### Task 2: Refactor and enhance install_hooks.py (GREEN → REFACTOR)

**Files:**
- Modify: `adapters/claude-code/install_hooks.py` (complete rewrite)

- [ ] **Step 1: Write the refactored install_hooks.py**

```python
#!/usr/bin/env python3
"""
Install Claude Code hooks for AgentPulse monitoring.

Adds hook configuration to ~/.claude/settings.json (user-level)
so AgentPulse receives lifecycle events from all Claude Code sessions.

Usage:
  python install_hooks.py                  # Install hooks (idempotent)
  python install_hooks.py --remove          # Remove hooks
  python install_hooks.py --status          # Show install status
  python install_hooks.py --dry-run         # Preview changes without modifying
  python install_hooks.py --force           # Force reinstall existing hooks
"""
import argparse
import json
import shutil
import sys
from pathlib import Path

HOOK_EVENTS = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
]

DEFAULT_SETTINGS_PATH = Path.home() / ".claude" / "settings.json"


def get_adapter_path() -> str:
    """Get absolute path to monitor_hook.py."""
    return str(Path(__file__).parent / "monitor_hook.py")


def build_hook_configs(monitor_script: str) -> dict:
    """Build the hooks configuration dict for all 6 event types."""
    hooks_config = {}
    for event in HOOK_EVENTS:
        hooks_config[event] = [
            {
                "matcher": "",
                "hooks": [
                    {"type": "command", "command": f"python {monitor_script}"}
                ],
            }
        ]
    return hooks_config


def load_settings(path: Path) -> dict:
    """Load settings JSON from path. Returns empty dict if file missing."""
    if path.exists():
        with open(path) as f:
            return json.load(f)
    return {}


def save_settings(path: Path, data: dict) -> None:
    """Save settings JSON to path, creating parent dirs as needed."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(data, f, indent=2)


def merge_hooks(settings: dict, hook_configs: dict) -> dict:
    """Return a new settings dict with hook_configs merged in."""
    settings = json.loads(json.dumps(settings))  # deep copy
    existing_hooks = settings.get("hooks", {})
    existing_hooks.update(hook_configs)
    settings["hooks"] = existing_hooks
    return settings


def remove_hooks_from_settings(settings: dict) -> dict:
    """Return a new settings dict with AgentPulse hooks removed."""
    settings = json.loads(json.dumps(settings))  # deep copy
    hooks = settings.get("hooks", {})
    for event in HOOK_EVENTS:
        hooks.pop(event, None)
    settings["hooks"] = hooks
    return settings


def get_hook_status(settings: dict) -> dict:
    """Return {event_name: bool} indicating which hooks are installed."""
    hooks = settings.get("hooks", {})
    return {event: event in hooks for event in HOOK_EVENTS}


def install(settings_path: Path, force: bool = False) -> str:
    """Install hooks. Returns 'installed' or 'already_installed'."""
    adapter_path = get_adapter_path()
    hook_configs = build_hook_configs(adapter_path)
    settings = load_settings(settings_path)

    if not force:
        existing = get_hook_status(settings)
        if all(existing.values()):
            return "already_installed"

    # Backup before modifying
    if settings_path.exists():
        backup_path = settings_path.with_suffix(".json.bak")
        shutil.copy2(settings_path, backup_path)

    new_settings = merge_hooks(settings, hook_configs)
    save_settings(settings_path, new_settings)
    return "installed"


def remove_hooks(settings_path: Path) -> str:
    """Remove hooks. Returns 'removed' or 'no_settings_file'."""
    if not settings_path.exists():
        return "no_settings_file"

    backup_path = settings_path.with_suffix(".json.bak")
    shutil.copy2(settings_path, backup_path)

    settings = load_settings(settings_path)
    new_settings = remove_hooks_from_settings(settings)
    save_settings(settings_path, new_settings)
    return "removed"


def status(settings_path: Path) -> dict:
    """Return hook status dict for the given settings path."""
    settings = load_settings(settings_path)
    return get_hook_status(settings)


def dry_run(settings_path: Path) -> dict:
    """Return dict describing what install would do."""
    adapter_path = get_adapter_path()
    settings = load_settings(settings_path)
    current_status = get_hook_status(settings)
    return {
        "hooks_to_install": [e for e, v in current_status.items() if not v],
        "hooks_already_installed": [e for e, v in current_status.items() if v],
        "monitor_script": adapter_path,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Install/uninstall AgentPulse Claude Code hooks"
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--remove", action="store_true", help="Remove hooks")
    group.add_argument("--status", action="store_true", help="Show hook install status")
    group.add_argument("--dry-run", action="store_true", help="Preview changes without modifying")
    parser.add_argument("--force", action="store_true", help="Force overwrite existing hooks")
    parser.add_argument(
        "--settings",
        type=str,
        default=str(DEFAULT_SETTINGS_PATH),
        help="Path to settings.json",
    )
    args = parser.parse_args()

    settings_path = Path(args.settings)

    if args.remove:
        result = remove_hooks(settings_path)
        if result == "no_settings_file":
            print("No settings file found. Nothing to remove.")
        else:
            print(f"AgentPulse hooks removed from {settings_path}")
    elif args.status:
        s = status(settings_path)
        for event in HOOK_EVENTS:
            mark = "[OK]" if s[event] else "[--]"
            print(f"  {mark} {event}")
    elif args.dry_run:
        info = dry_run(settings_path)
        print(f"Monitor script: {info['monitor_script']}")
        for event in info["hooks_already_installed"]:
            print(f"  [SKIP] {event} (already installed)")
        for event in info["hooks_to_install"]:
            print(f"  [WILL INSTALL] {event}")
    else:
        print("Installing AgentPulse Claude Code hooks...")
        print(f"Adapter: {get_adapter_path()}")
        print()
        result = install(settings_path, force=args.force)
        if result == "already_installed":
            print("All hooks already installed. Use --force to reinstall.")
        else:
            for event in HOOK_EVENTS:
                print(f"  [OK] {event}")
            print()
            print(f"Hooks installed to {settings_path}")
            print("AgentPulse will now receive events from all Claude Code sessions.")
            print("Make sure the AgentPulse desktop app is running.")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run tests to verify they pass (GREEN)**

```powershell
python -m pytest tests/unit/test_install_hooks.py -v
```

Expected: 16 passed (all green)

- [ ] **Step 3: Verify existing E2E tests still pass**

```powershell
python tests/integration/test_e2e.py
```

Expected: All tests pass or skip gracefully

- [ ] **Step 4: Commit**

```bash
git add adapters/claude-code/install_hooks.py
git commit -m "refactor: enhance install_hooks.py with --status, --dry-run, --force, idempotent install, auto-backup"
```

---

### Task 3: Write failing unit tests for monitor_hook.py (RED)

**Files:**
- Create: `tests/unit/test_monitor_hook.py`

- [ ] **Step 1: Write all 8 monitor_hook unit tests**

```python
# tests/unit/test_monitor_hook.py
import json
import pytest
import sys
from io import StringIO
from pathlib import Path
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "adapters" / "claude-code"))
import monitor_hook


class TestReadStdin:
    def test_returns_dict_for_valid_json(self):
        stdin_data = (
            '{"session_id": "test-001", "hook_event_name": "SessionStart"}'
        )
        with patch.object(sys, "stdin", StringIO(stdin_data)):
            result = monitor_hook.read_stdin()
            assert result == {
                "session_id": "test-001",
                "hook_event_name": "SessionStart",
            }

    def test_returns_none_for_empty_stdin(self):
        with patch.object(sys, "stdin", StringIO("")):
            result = monitor_hook.read_stdin()
            assert result is None

    def test_returns_none_for_whitespace_only(self):
        with patch.object(sys, "stdin", StringIO("  \n  ")):
            result = monitor_hook.read_stdin()
            assert result is None

    def test_exits_on_invalid_json(self):
        with patch.object(sys, "stdin", StringIO("not valid json")):
            with pytest.raises(SystemExit) as excinfo:
                monitor_hook.read_stdin()
            assert excinfo.value.code == 1


class TestSendEvent:
    def test_success_returns_201(self):
        mock_response = MagicMock()
        mock_response.status = 201
        mock_response.__enter__ = MagicMock(return_value=mock_response)
        mock_response.__exit__ = MagicMock(return_value=False)
        with patch("urllib.request.urlopen", return_value=mock_response):
            status = monitor_hook.send_event(
                "http://test/api/events", {"key": "val"}, 5
            )
            assert status == 201

    def test_server_error_returns_status_code(self):
        import urllib.error
        mock_error = urllib.error.HTTPError("url", 500, "Error", {}, None)
        with patch("urllib.request.urlopen", side_effect=mock_error):
            status = monitor_hook.send_event(
                "http://test/api/events", {"key": "val"}, 5
            )
            assert status == 500

    def test_retries_on_connection_refused(self):
        import urllib.error
        with patch(
            "urllib.request.urlopen",
            side_effect=urllib.error.URLError("refused"),
        ):
            with patch("time.sleep", return_value=None):
                status = monitor_hook.send_event(
                    "http://test/api/events", {"key": "val"}, 5
                )
                assert status == -1

    def test_connection_error_returns_negative(self):
        with patch(
            "urllib.request.urlopen",
            side_effect=OSError("timeout"),
        ):
            with patch("time.sleep", return_value=None):
                status = monitor_hook.send_event(
                    "http://test/api/events", {"key": "val"}, 5
                )
                assert status < 0
```

- [ ] **Step 2: Run tests to verify they fail (RED)**

```powershell
python -m pytest tests/unit/test_monitor_hook.py -v
```

Expected: FAIL — functions like `read_stdin()` and `send_event()` don't exist yet on the monitor_hook module in the expected signature form.

- [ ] **Step 3: Commit**

```bash
git add tests/unit/test_monitor_hook.py
git commit -m "test: add failing unit tests for monitor_hook (RED)"
```

---

### Task 4: Refactor and enhance monitor_hook.py (GREEN → REFACTOR)

**Files:**
- Modify: `adapters/claude-code/monitor_hook.py` (complete rewrite)

- [ ] **Step 1: Write the refactored monitor_hook.py**

```python
#!/usr/bin/env python3
"""
Claude Code monitor hook for AgentPulse.

Reads hook JSON from stdin (Claude Code passes hook data via stdin),
and POSTs it to the local AgentPulse event server.

Usage in Claude Code settings.json:
  {
    "hooks": {
      "PostToolUse": [
        {
          "matcher": "",
          "hooks": [
            { "type": "command", "command": "python /path/to/monitor_hook.py" }
          ]
        }
      ]
    }
  }

Environment variables:
  AGENTPULSE_URL         - Event server URL (default: http://127.0.0.1:17878/api/events)
  AGENTPULSE_TIMEOUT     - Request timeout in seconds (default: 5)
  AGENTPULSE_LOG_LEVEL   - Logging level: DEBUG, INFO, WARNING, ERROR (default: INFO)
"""
import argparse
import json
import logging
import os
import sys
import time
import urllib.error
import urllib.request

AGENTPULSE_URL = os.environ.get(
    "AGENTPULSE_URL", "http://127.0.0.1:17878/api/events"
)
DEFAULT_TIMEOUT = int(os.environ.get("AGENTPULSE_TIMEOUT", "5"))
MAX_RETRIES = 3
RETRY_DELAY = 1.0

logging.basicConfig(
    level=getattr(logging, os.environ.get("AGENTPULSE_LOG_LEVEL", "INFO")),
    format="%(asctime)s [AgentPulse] %(levelname)s %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)


def read_stdin() -> dict | None:
    """Read hook JSON from stdin. Returns None if empty, exits on parse error."""
    raw_input = sys.stdin.read().strip()
    if not raw_input:
        logger.info("No stdin data, skipping")
        return None
    try:
        return json.loads(raw_input)
    except json.JSONDecodeError as e:
        logger.error("Failed to parse stdin as JSON: %s", e)
        sys.exit(1)


def send_event(url: str, data: dict, timeout: int) -> int:
    """POST event JSON to AgentPulse server. Returns HTTP status or -1 on failure."""
    body = json.dumps(data).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    last_error = None
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                status = resp.status
                if status == 201:
                    logger.info("Event sent successfully (attempt %d)", attempt)
                else:
                    logger.warning(
                        "Server returned %d (attempt %d)", status, attempt
                    )
                return status
        except urllib.error.HTTPError as e:
            logger.warning(
                "Server returned %d (attempt %d)", e.code, attempt
            )
            return e.code
        except (urllib.error.URLError, OSError) as e:
            last_error = e
            if attempt < MAX_RETRIES:
                logger.warning(
                    "Connection failed (attempt %d/%d): %s",
                    attempt,
                    MAX_RETRIES,
                    e,
                )
                time.sleep(RETRY_DELAY)

    logger.error(
        "Failed to send event after %d attempts: %s", MAX_RETRIES, last_error
    )
    return -1


def main():
    parser = argparse.ArgumentParser(
        description="AgentPulse Claude Code monitor hook"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="Print payload to stdout instead of POSTing to server",
    )
    args = parser.parse_args()

    hook_data = read_stdin()
    if hook_data is None:
        sys.exit(0)

    if args.test:
        print(json.dumps(hook_data, indent=2))
        sys.exit(0)

    status = send_event(AGENTPULSE_URL, hook_data, DEFAULT_TIMEOUT)
    if status < 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run tests to verify they pass (GREEN)**

```powershell
python -m pytest tests/unit/test_monitor_hook.py -v
```

Expected: 8 passed (all green)

- [ ] **Step 3: Commit**

```bash
git add adapters/claude-code/monitor_hook.py
git commit -m "refactor: enhance monitor_hook.py with logging, retry, --test mode, configurable timeout"
```

---

### Task 5: Run full test suite and verify (VERIFY)

- [ ] **Step 1: Run all unit tests**

```powershell
python -m pytest tests/unit/ -v
```

Expected: 24 passed (16 install_hooks + 8 monitor_hook)

- [ ] **Step 2: Run Rust tests**

```powershell
cd apps/desktop/src-tauri
cargo test
```

Expected: all Rust tests pass (25 tests as per docs)

- [ ] **Step 3: Run frontend type check**

```powershell
cd apps/desktop
npx vue-tsc --noEmit
```

Expected: zero errors

- [ ] **Step 4: Commit**

```bash
# No code changes to commit; this is verification only
```

---

### Task 6: Install hooks for real (DEPLOY)

- [ ] **Step 1: Run dry-run first to preview**

```powershell
python adapters/claude-code/install_hooks.py --dry-run
```

Expected: Shows list of 6 events as `[WILL INSTALL]`

- [ ] **Step 2: Check current status**

```powershell
python adapters/claude-code/install_hooks.py --status
```

Expected: All 6 show `[--]` (not installed)

- [ ] **Step 3: Run the installer**

```powershell
python adapters/claude-code/install_hooks.py
```

Expected: 6 `[OK]` lines printed, hooks installed

- [ ] **Step 4: Verify hooks were written to settings.json**

```powershell
python -c "import json; from pathlib import Path; s = json.loads(Path.home().joinpath('.claude', 'settings.json').read_text()); print(json.dumps(s.get('hooks', {}), indent=2))"
```

Expected: Output shows 6 hook events, each pointing to `monitor_hook.py` with correct `python` command

- [ ] **Step 5: Verify idempotent behavior**

```powershell
python adapters/claude-code/install_hooks.py
```

Expected: "All hooks already installed. Use --force to reinstall."

- [ ] **Step 6: Verify status reports all installed**

```powershell
python adapters/claude-code/install_hooks.py --status
```

Expected: All 6 show `[OK]`

- [ ] **Step 7: Commit**

No commit needed — this is a local environment mutation

---

### Task 7: Smoke test with real Claude Code session (SMOKE TEST)

- [ ] **Step 1: Start AgentPulse in background**

```powershell
cd apps/desktop
npm run tauri dev
```

Leave it running. Expected: Floating window appears, event server on :17878.

- [ ] **Step 2: Open a new Claude Code session in a different project**

Open a new terminal, cd to any other project, run `claude`. Execute a simple action (e.g., ask "what is 2+2").

- [ ] **Step 3: Observe AgentPulse UI**

Expected:
- AgentPulse floating window shows a session card appear with project name
- Status changes: Starting → Running → Completed
- Session card shows tool names used during the session

- [ ] **Step 4: Run E2E test while real CC session is active**

```powershell
python tests/integration/test_e2e.py
```

Expected: All tests pass (or skip gracefully if server not reachable)

- [ ] **Step 5: Verify monitor_hook --test mode**

```powershell
echo '{"session_id":"smoke-test","hook_event_name":"SessionStart","cwd":"/tmp/test"}' | python adapters/claude-code/monitor_hook.py --test
```

Expected: Prints formatted JSON to stdout, exit 0, no HTTP request made

- [ ] **Step 6: Commit**

No commit needed — this is testing only

---

### Task 8: Update documentation (DOCS)

- [ ] **Step 1: Update connect-real-claude-code.md**

Replace the content of `docs/todos/connect-real-claude-code.md`:

```markdown
# TODO: 接入真实 Claude Code 事件

## 状态

**已完成** — 2026-05-23

## 所做的工作

1. 重构 `install_hooks.py`：添加 `--dry-run`、`--status`、`--force`、幂等安装、自动备份
2. 重构 `monitor_hook.py`：添加 stderr 日志、3 次重试、可配置超时、`--test` 模式
3. 添加 24 个单元测试（16 install_hooks + 8 monitor_hook）
4. 运行 `install_hooks.py` 将 6 个 hook 事件注册到 `~/.claude/settings.json`
5. 用真实 Claude Code session 进行端到端验证

## 验证结果

- hooks 成功写入 `~/.claude/settings.json`
- AgentPulse 浮窗正确显示真实 CC session 状态变化
- E2E 测试通过
- 单元测试全部通过

## 相关文件

- [adapters/claude-code/install_hooks.py](../../adapters/claude-code/install_hooks.py)
- [adapters/claude-code/monitor_hook.py](../../adapters/claude-code/monitor_hook.py)
- [tests/unit/test_install_hooks.py](../../tests/unit/test_install_hooks.py)
- [tests/unit/test_monitor_hook.py](../../tests/unit/test_monitor_hook.py)
- [设计文档](../superpowers/specs/2026-05-23-connect-real-claude-code-design.md)
- [实现计划](../superpowers/plans/2026-05-23-connect-real-claude-code-plan.md)
```

- [ ] **Step 2: Update local-development-guide.md**

In `docs/local-development-guide.md`, replace the "五、接入 Claude Code" section (lines 245-280) with:

```markdown
## 五、接入 Claude Code

### 5.1 安装 Hooks

```powershell
# 预览将要执行的操作
python adapters/claude-code/install_hooks.py --dry-run

# 安装 hooks
python adapters/claude-code/install_hooks.py

# 查看安装状态
python adapters/claude-code/install_hooks.py --status
```

这会在 `~/.claude/settings.json` 中写入 6 个 hook 事件的配置：

- SessionStart
- PreToolUse
- PostToolUse
- PostToolUseFailure
- Notification
- Stop

### 5.2 验证 Hooks 是否生效

```powershell
# 方式 1: 使用 --status
python adapters/claude-code/install_hooks.py --status

# 方式 2: 直接查看配置
python -c "import json; from pathlib import Path; s = json.loads(Path.home().joinpath('.claude', 'settings.json').read_text()); print(json.dumps(s.get('hooks', {}), indent=2))"
```

应能看到 `"hooks"` 字段包含 6 个事件配置，每个指向 `monitor_hook.py`。

### 5.3 卸载 Hooks

```powershell
python adapters/claude-code/install_hooks.py --remove
```

### 5.4 调试 Hook 事件

```powershell
# 测试 monitor_hook.py 是否正确解析 stdin（不发送到服务器）
echo '{"session_id":"debug-1","cwd":"/tmp/test","hook_event_name":"SessionStart"}' | python adapters/claude-code/monitor_hook.py --test

# 开启详细日志
$env:AGENTPULSE_LOG_LEVEL = "DEBUG"
echo '{"session_id":"debug-1","cwd":"/tmp/test","hook_event_name":"SessionStart"}' | python adapters/claude-code/monitor_hook.py

# 指定事件服务器地址（如果非默认端口）
$env:AGENTPULSE_URL = "http://127.0.0.1:9999/api/events"
echo '...' | python adapters/claude-code/monitor_hook.py
```

### 5.5 使用流程

1. 启动 AgentPulse：`npm run tauri dev`（或运行打包后的 exe）
2. 正常使用 Claude Code
3. AgentPulse 浮窗自动显示 CC session 状态

### 5.6 安全说明

- 安装和卸载操作会自动在修改前备份 `settings.json` → `settings.json.bak`
- 重复安装不会覆盖已有配置（幂等操作），使用 `--force` 强制覆盖
- 卸载时只移除 AgentPulse 的 6 个 hook 事件，保留其他自定义 hooks
```

- [ ] **Step 3: Update testing-strategy.md**

Replace the content of `docs/testing/testing-strategy.md`:

```markdown
# Testing Strategy

## Layers

| Layer | Tool | Scope |
|-------|------|-------|
| Rust unit | `cargo test` | db, state_machine, event_server, types (in-module tests) |
| Rust integration | `cargo test` | db integration (in-module `#[cfg(test)]`) |
| Python unit | `pytest tests/unit/` | install_hooks.py (10 cases), monitor_hook.py (8 cases) |
| Python E2E | `python tests/integration/test_e2e.py` | Full pipeline: POST events → verify DB state |
| TypeScript type-check | `npx vue-tsc --noEmit` | Frontend type safety |

## Running All Tests

```powershell
# Rust
cd apps/desktop/src-tauri
cargo test

# Python
python -m pytest tests/unit/ -v
python tests/integration/test_e2e.py

# TypeScript
cd apps/desktop
npx vue-tsc --noEmit
```
```

- [ ] **Step 4: Update context-map.md**

Replace the content of `docs/ai/context-map.md`:

```markdown
# Context Map

## Key Files

| File | Purpose |
|------|---------|
| `adapters/claude-code/install_hooks.py` | Install/uninstall/status/dry-run hooks in ~/.claude/settings.json |
| `adapters/claude-code/monitor_hook.py` | Read hook JSON from stdin, POST to event server :17878 |
| `apps/desktop/src-tauri/src/lib.rs` | Shared Rust types (AgentEvent, AgentSession, AgentStatus) |
| `apps/desktop/src-tauri/src/db.rs` | SQLite schema, CRUD for sessions and events |
| `apps/desktop/src-tauri/src/state_machine.rs` | Session status transition validation |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP server :17878, event normalization |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri commands (get_sessions, get_session_detail) |
| `apps/desktop/src-tauri/src/tray.rs` | System tray with show/hide/quit |
| `apps/desktop/src/types/agent.ts` | TypeScript types matching Rust structs |
| `apps/desktop/src/stores/sessionStore.ts` | Pinia store: polling, expand/collapse |
| `apps/desktop/src/components/FloatingPanel.vue` | Main panel, session list, error banner |
| `apps/desktop/src/components/SessionCard.vue` | Single session card with status dot |
| `apps/desktop/src/components/ExpandedDetail.vue` | Expanded session detail with actions |
| `tests/unit/test_install_hooks.py` | Unit tests for install_hooks.py |
| `tests/unit/test_monitor_hook.py` | Unit tests for monitor_hook.py |
| `tests/integration/test_e2e.py` | E2E pipeline smoke test |
```

- [ ] **Step 5: Commit**

```bash
git add docs/todos/connect-real-claude-code.md docs/local-development-guide.md docs/testing/testing-strategy.md docs/ai/context-map.md
git commit -m "docs: update documentation for real Claude Code connection and adapter tests"
```

---

### Task 9: Final commit with all remaining changes

- [ ] **Step 1: Verify nothing left uncommitted**

```powershell
git status
```

Expected: Clean working tree

- [ ] **Step 2: Review everything at once**

```powershell
git log --oneline -10
```

Expected: Shows the full chain of commits for this feature

- [ ] **Step 3: Final verification — run all tests one last time**

```powershell
python -m pytest tests/unit/ -v
python tests/integration/test_e2e.py
cd apps/desktop/src-tauri && cargo test
```

Expected: All pass

---

## Execution Order

```
Task 1 (RED install tests) → Task 2 (GREEN install code) → Task 3 (RED monitor tests) → Task 4 (GREEN monitor code) → Task 5 (full test suite) → Task 6 (install hooks) → Task 7 (smoke test) → Task 8 (docs) → Task 9 (final verify)
```
