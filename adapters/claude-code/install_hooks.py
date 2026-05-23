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
                    {"type": "command", "command": f'python "{monitor_script}"'}
                ],
            }
        ]
    return hooks_config


def load_settings(path: Path) -> dict:
    """Load settings JSON from path. Returns empty dict if file missing."""
    if path.exists():
        try:
            with open(path) as f:
                return json.load(f)
        except (json.JSONDecodeError, PermissionError) as e:
            print(f"Error: Cannot read {path}: {e}", file=sys.stderr)
            sys.exit(1)
    return {}


def save_settings(path: Path, data: dict) -> None:
    """Save settings JSON to path, creating parent dirs as needed."""
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(path, "w") as f:
            json.dump(data, f, indent=2)
    except (OSError, PermissionError) as e:
        print(f"Error: Cannot write to {path}: {e}", file=sys.stderr)
        sys.exit(1)


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
    """Return dict describing what install would do, without modifying anything."""
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
