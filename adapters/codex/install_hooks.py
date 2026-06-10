#!/usr/bin/env python3
"""
Install Codex hooks for AgentPulse monitoring.

Adds hook configuration to ~/.codex/config.toml (user-level) in TOML format
so AgentPulse receives lifecycle events from all Codex CLI sessions.

The hook data is passed via stdin as JSON — Codex CLI uses the same
command-type hook mechanism as Claude Code.

Usage:
  python install_hooks.py                  # Install hooks (idempotent)
  python install_hooks.py --remove          # Remove hooks
  python install_hooks.py --status          # Show install status
  python install_hooks.py --dry-run         # Preview changes without modifying
  python install_hooks.py --force           # Force reinstall existing hooks
"""
import argparse
import shutil
import sys
from pathlib import Path

HOOK_EVENTS = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "UserPromptSubmit",
    "Stop",
]

DEFAULT_CONFIG_PATH = Path.home() / ".codex" / "config.toml"


def get_adapter_path() -> str:
    """Get absolute path to monitor_hook.py (same dir as this script)."""
    return str(Path(__file__).parent / "monitor_hook.py")


def build_hook_entry(event: str, command: str) -> str:
    """Format a single hook event entry as TOML."""
    return (
        f'{event} = [\n'
        f'  {{ matcher = "", hooks = [\n'
        f'    {{ type = "command", command = \'{command}\' }}\n'
        f'  ] }}\n'
        f']'
    )


def _parse_toml_sections(raw: str) -> dict:
    """Parse TOML into {section_name: list_of_lines} dict.

    This is a minimal parser that handles the subset of TOML used by
    Codex config files. It only cares about top-level [section] headers
    and top-level key=value pairs (stored under "_top").
    """
    sections = {"_top": []}
    current_section = "_top"

    for line in raw.split("\n"):
        stripped = line.strip()
        # Top-level section header: [section_name]
        if stripped.startswith("[") and stripped.endswith("]") and not stripped.startswith("[["):
            current_section = stripped[1:-1]
            if current_section not in sections:
                sections[current_section] = []
        else:
            sections.setdefault(current_section, []).append(line)

    return sections


def _find_event_start(lines: list[str], event: str) -> int | None:
    """Find the line index where a hook event entry starts."""
    for i, line in enumerate(lines):
        if line.strip().startswith(f"{event} ="):
            return i
    return None


def _find_event_end(lines: list[str], start: int) -> int:
    """Find the line index where a hook event entry ends.
    An event entry ends at the next top-level key (non-indented '=' sign)
    or at the end of lines."""
    for i in range(start + 1, len(lines)):
        stripped = lines[i].strip()
        # Top-level key = value (non-indented and not part of our array)
        if "=" in stripped and not stripped.startswith((" ", "\t", "{", "]")):
            return i
    return len(lines)


def merge_hooks_to_toml(raw: str, monitor_script: str) -> str:
    """Merge our 6 hook events into the raw TOML content.

    If a [hooks] section already exists, we update our 6 events within
    it while preserving all other entries. If not, we append a new
    [hooks] section at the end.
    """
    escaped = monitor_script.replace("\\", "\\\\").replace("'", "\\'")
    sections = _parse_toml_sections(raw)

    # Build our hook entries
    our_entries = {}
    for event in HOOK_EVENTS:
        our_entries[event] = build_hook_entry(event, f'python "{escaped}"')

    if "hooks" in sections:
        lines = sections["hooks"]
        # Remove any existing entries for our events
        for event in HOOK_EVENTS:
            start = _find_event_start(lines, event)
            while start is not None:
                end = _find_event_end(lines, start)
                # Replace the event block with empty lines (preserve line count for simplicity)
                lines = lines[:start] + lines[end:]
                start = _find_event_start(lines, event)

        # Append our entries
        for event in HOOK_EVENTS:
            lines.append(our_entries[event])

        sections["hooks"] = lines
    else:
        # Add a new [hooks] section with our entries
        entries = [our_entries[event] for event in HOOK_EVENTS]
        sections["hooks"] = entries

    # Reassemble: preserve original section order, put hooks last if new
    output_lines = []

    # Handle _top (key=value pairs before any section)
    if "_top" in sections and sections["_top"]:
        output_lines.extend(sections["_top"])

    # Track which sections we've seen in original order
    seen_hooks = False
    current_section = "_top"
    ordered_sections: list[tuple[str, list[str]]] = []
    for line in raw.split("\n"):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]") and not stripped.startswith("[["):
            current_section = stripped[1:-1]
            if current_section not in [s[0] for s in ordered_sections]:
                ordered_sections.append((current_section, sections.get(current_section, [])))
        elif current_section == "_top":
            pass  # already handled

    # Add remaining sections not seen in original order
    for name, lines_list in sections.items():
        if name == "_top":
            continue
        if name not in [s[0] for s in ordered_sections]:
            ordered_sections.append((name, lines_list))

    # Output each section
    for name, lines_list in ordered_sections:
        output_lines.append(f"[{name}]")
        for line in lines_list:
            output_lines.append(line)
        output_lines.append("")  # blank line between sections

    return "\n".join(output_lines).rstrip("\n") + "\n"


def install(config_path: Path, force: bool = False) -> str:
    """Install Codex hooks. Returns 'installed' or 'already_installed'."""
    adapter_path = get_adapter_path()

    config_path.parent.mkdir(parents=True, exist_ok=True)

    if config_path.exists():
        existing_raw = config_path.read_text(encoding="utf-8")
    else:
        existing_raw = ""

    if not force:
        all_installed = all(
            f"{event} =" in existing_raw for event in HOOK_EVENTS
        )
        if all_installed:
            return "already_installed"

    # Backup before modifying
    if config_path.exists():
        backup_path = config_path.with_suffix(".toml.bak")
        shutil.copy2(config_path, backup_path)

    new_raw = merge_hooks_to_toml(existing_raw, adapter_path)
    config_path.write_text(new_raw, encoding="utf-8")
    return "installed"


def remove_hooks(config_path: Path) -> str:
    """Remove AgentPulse hooks from config.toml. Returns 'removed' or 'no_config_file'."""
    if not config_path.exists():
        return "no_config_file"

    backup_path = config_path.with_suffix(".toml.bak")
    shutil.copy2(config_path, backup_path)

    raw = config_path.read_text(encoding="utf-8")
    sections = _parse_toml_sections(raw)

    if "hooks" not in sections:
        return "removed"  # Nothing to remove

    lines = list(sections["hooks"])

    # Remove our 6 events
    for event in HOOK_EVENTS:
        start = _find_event_start(lines, event)
        while start is not None:
            end = _find_event_end(lines, start)
            lines = lines[:start] + lines[end:]
            start = _find_event_start(lines, event)

    # Remove trailing blank lines
    while lines and lines[-1].strip() == "":
        lines.pop()

    sections["hooks"] = lines

    # Reassemble
    output_lines = []

    # _top section
    if "_top" in sections and sections["_top"]:
        output_lines.extend(sections["_top"])

    # Other sections
    for name, lines_list in sections.items():
        if name == "_top":
            continue
        if name == "hooks" and not lines_list:
            # Don't write empty hooks section — use a comment
            continue
        output_lines.append(f"[{name}]")
        for line in lines_list:
            output_lines.append(line)
        output_lines.append("")

    result = "\n".join(output_lines).rstrip("\n")
    if result:
        result += "\n"
    config_path.write_text(result, encoding="utf-8")
    return "removed"


def status(config_path: Path) -> dict:
    """Return hook status dict for the given config path."""
    if not config_path.exists():
        return {event: False for event in HOOK_EVENTS}

    raw = config_path.read_text(encoding="utf-8")
    return {event: f"{event} =" in raw for event in HOOK_EVENTS}


def dry_run(config_path: Path) -> dict:
    """Return dict describing what install would do."""
    adapter_path = get_adapter_path()
    current_status = status(config_path)
    return {
        "hooks_to_install": [e for e, v in current_status.items() if not v],
        "hooks_already_installed": [e for e, v in current_status.items() if v],
        "monitor_script": adapter_path,
        "config_path": str(config_path),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Install/uninstall AgentPulse Codex hooks"
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--remove", action="store_true", help="Remove hooks")
    group.add_argument("--status", action="store_true", help="Show hook install status")
    group.add_argument("--dry-run", action="store_true", help="Preview changes without modifying")
    parser.add_argument("--force", action="store_true", help="Force overwrite existing hooks")
    parser.add_argument(
        "--config",
        type=str,
        default=str(DEFAULT_CONFIG_PATH),
        help="Path to config.toml",
    )
    args = parser.parse_args()

    config_path = Path(args.config)

    if args.remove:
        result = remove_hooks(config_path)
        if result == "no_config_file":
            print("No config.toml file found. Nothing to remove.")
        else:
            print(f"AgentPulse Codex hooks removed from {config_path}")
    elif args.status:
        s = status(config_path)
        for event in HOOK_EVENTS:
            mark = "[OK]" if s[event] else "[--]"
            print(f"  {mark} {event}")
    elif args.dry_run:
        info = dry_run(config_path)
        print(f"Config path: {info['config_path']}")
        print(f"Monitor script: {info['monitor_script']}")
        for event in info["hooks_already_installed"]:
            print(f"  [SKIP] {event} (already installed)")
        for event in info["hooks_to_install"]:
            print(f"  [WILL INSTALL] {event}")
    else:
        print("Installing AgentPulse Codex hooks...")
        print(f"Adapter: {get_adapter_path()}")
        print()
        result = install(config_path, force=args.force)
        if result == "already_installed":
            print("All Codex hooks already installed. Use --force to reinstall.")
        else:
            for event in HOOK_EVENTS:
                print(f"  [OK] {event}")
            print()
            print(f"Hooks installed to {config_path}")
            print("AgentPulse will now receive events from all Codex CLI sessions.")
            print("Make sure the AgentPulse desktop app is running.")


if __name__ == "__main__":
    main()
