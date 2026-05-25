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
import ctypes
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


# Shell process names that sit between Claude Code and our hook script.
_SHELL_NAMES = frozenset({"cmd.exe", "powershell.exe", "pwsh.exe", "sh.exe", "bash.exe", "conhost.exe"})


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


def _walk_process_tree_to_cc() -> int:
    """Walk up the parent chain to find the Claude Code (node.exe) process.

    Claude Code spawns hook commands through a shell (cmd.exe / powershell.exe),
    so ``os.getppid()`` returns the shell PID which exits instantly. We walk
    upward until we find a non-shell process, which should be the CC node process.
    Falls back to ``os.getppid()`` on error or non-Windows platforms.
    """
    if sys.platform != "win32":
        return os.getppid()

    try:
        pid_to_parent, pid_to_name = _snapshot_processes()

        # Walk up from the current process, skipping known shell wrappers.
        cur = os.getpid()
        for _ in range(5):  # safety limit
            parent = pid_to_parent.get(cur)
            if parent is None:
                break
            name = pid_to_name.get(parent, "").lower()
            if name not in _SHELL_NAMES:
                return parent
            cur = parent

        # Fallback: return the last parent we found, or PPID.
        return os.getppid()
    except Exception:
        return os.getppid()


def _snapshot_processes() -> tuple[dict[int, int], dict[int, str]]:
    """Take a process snapshot and return (pid→parent_pid, pid→name)."""
    TH32CS_SNAPPROCESS = 0x00000002
    INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value

    class PROCESSENTRY32(ctypes.Structure):
        _fields_ = [
            ("dwSize", ctypes.c_ulong),
            ("cntUsage", ctypes.c_ulong),
            ("th32ProcessID", ctypes.c_ulong),
            ("th32DefaultHeapID", ctypes.POINTER(ctypes.c_ulong)),
            ("th32ModuleID", ctypes.c_ulong),
            ("cntThreads", ctypes.c_ulong),
            ("th32ParentProcessID", ctypes.c_ulong),
            ("pcPriClassBase", ctypes.c_long),
            ("dwFlags", ctypes.c_ulong),
            ("szExeFile", ctypes.c_char * 260),
        ]

    kernel32 = ctypes.windll.kernel32
    snapshot = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snapshot == INVALID_HANDLE_VALUE:
        return {}, {}

    pid_to_parent: dict[int, int] = {}
    pid_to_name: dict[int, str] = {}

    entry = PROCESSENTRY32()
    entry.dwSize = ctypes.sizeof(PROCESSENTRY32)

    if kernel32.Process32First(snapshot, ctypes.byref(entry)):
        while True:
            pid = entry.th32ProcessID
            pid_to_parent[pid] = entry.th32ParentProcessID
            pid_to_name[pid] = entry.szExeFile.decode("utf-8", errors="replace")
            if not kernel32.Process32Next(snapshot, ctypes.byref(entry)):
                break

    kernel32.CloseHandle(snapshot)
    return pid_to_parent, pid_to_name
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

    # Walk up the process tree to find the Claude Code (node.exe) PID.
    # os.getppid() would give us the shell that spawned us — that exits
    # instantly, so we walk past shell wrappers to the real CC process.
    hook_data["process_pid"] = _walk_process_tree_to_cc()

    if args.test:
        print(json.dumps(hook_data, indent=2))
        sys.exit(0)

    status = send_event(AGENTPULSE_URL, hook_data, DEFAULT_TIMEOUT)
    if status < 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
