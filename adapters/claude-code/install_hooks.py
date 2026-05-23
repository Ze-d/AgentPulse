#!/usr/bin/env python3
"""
Install Claude Code hooks for AgentPulse monitoring.

Adds hook configuration to ~/.claude/settings.json (user-level)
so AgentPulse receives lifecycle events from all Claude Code sessions.

Usage:
  python install_hooks.py          # Install hooks
  python install_hooks.py --remove # Remove hooks
"""
import json
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

SETTINGS_PATH = Path.home() / ".claude" / "settings.json"


def get_adapter_path() -> str:
    """Get absolute path to monitor_hook.py."""
    return str(Path(__file__).parent / "monitor_hook.py")

def install():
    adapter_path = get_adapter_path()

    # Build hooks config for each event
    hooks_config = {}
    for event in HOOK_EVENTS:
        hooks_config[event] = [
            {
                "matcher": "",
                "hooks": [
                    {"type": "command", "command": f"python {adapter_path}"}
                ],
            }
        ]

    # Read existing settings
    settings = {}
    if SETTINGS_PATH.exists():
        with open(SETTINGS_PATH) as f:
            settings = json.load(f)

    # Merge hooks
    existing_hooks = settings.get("hooks", {})
    existing_hooks.update(hooks_config)
    settings["hooks"] = existing_hooks

    # Write back
    SETTINGS_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(SETTINGS_PATH, "w") as f:
        json.dump(settings, f, indent=2)

    print(f"AgentPulse hooks installed to {SETTINGS_PATH}")
    print(f"Events: {', '.join(HOOK_EVENTS)}")


def remove():
    if not SETTINGS_PATH.exists():
        print("No settings file found. Nothing to remove.")
        return

    with open(SETTINGS_PATH) as f:
        settings = json.load(f)

    hooks = settings.get("hooks", {})
    for event in HOOK_EVENTS:
        hooks.pop(event, None)

    settings["hooks"] = hooks
    with open(SETTINGS_PATH, "w") as f:
        json.dump(settings, f, indent=2)

    print(f"AgentPulse hooks removed from {SETTINGS_PATH}")


def main():
    if "--remove" in sys.argv:
        remove()
    else:
        print("Installing AgentPulse Claude Code hooks...")
        print(f"Adapter: {get_adapter_path()}")
        print()
        install()
        print()
        print("Done! AgentPulse will now receive events from all Claude Code sessions.")
        print("Make sure the AgentPulse desktop app is running.")


if __name__ == "__main__":
    main()
