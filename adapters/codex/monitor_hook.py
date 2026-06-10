#!/usr/bin/env python3
"""
Codex CLI monitor hook for AgentPulse.

Reads hook JSON from stdin (Codex CLI passes hook data via stdin),
injects `agent_source: "codex"` and `process_pid`, then POSTs to the
local AgentPulse event server.

The hook JSON format from Codex is structurally identical to Claude Code:
  - session_id, cwd, hook_event_name, transcript_path
  - Plus Codex-specific fields: model, permission_mode, turn_id

Usage in ~/.codex/config.toml:
  [hooks]
  SessionStart = [
    { matcher = "", hooks = [
      { type = "command", command = "python /path/to/monitor_hook.py" }
    ]}
  ]

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
    format="%(asctime)s [AgentPulse:Codex] %(levelname)s %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)


# Shell process names that sit between the agent and our hook script.
_SHELL_NAMES = frozenset({
    "cmd.exe", "powershell.exe", "pwsh.exe",
    "sh.exe", "bash.exe", "conhost.exe",
})

# Recognised agent binary names (used for PID detection fallback).
_AGENT_BINARIES = {
    "node.exe": "claude-code",
    "codex.exe": "codex",
    "codex": "codex",
}


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


def _detect_agent_info() -> tuple[int, str]:
    """Walk up the parent chain to find the agent PID and determine its source.

    Returns (pid, agent_source). Falls back to (ppid, "codex").
    """
    if sys.platform != "win32":
        return os.getppid(), "codex"

    try:
        pid_to_parent, pid_to_name = _snapshot_processes()

        cur = os.getpid()
        last_non_shell = cur
        detected_source = "codex"  # default for this adapter
        for _ in range(5):
            parent = pid_to_parent.get(cur)
            if parent is None:
                break
            name = pid_to_name.get(parent, "").lower()
            if name not in _SHELL_NAMES:
                last_non_shell = parent
                if name in _AGENT_BINARIES:
                    return parent, _AGENT_BINARIES[name]
            cur = parent

        return last_non_shell, detected_source
    except Exception:
        return os.getppid(), "codex"


def _snapshot_processes() -> tuple[dict[int, int], dict[int, str]]:
    """Take a process snapshot and return (pid->parent_pid, pid->name)."""
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
        description="AgentPulse Codex CLI monitor hook"
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

    # Walk up the process tree to find the agent PID and source.
    process_pid, agent_source = _detect_agent_info()
    hook_data["process_pid"] = process_pid
    hook_data["agent_source"] = agent_source

    logger.debug("detected agent: source=%s pid=%d", agent_source, process_pid)

    if args.test:
        print(json.dumps(hook_data, indent=2))
        sys.exit(0)

    status = send_event(AGENTPULSE_URL, hook_data, DEFAULT_TIMEOUT)
    if status < 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
