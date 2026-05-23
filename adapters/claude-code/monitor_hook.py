#!/usr/bin/env python3
"""
Claude Code hook adapter for AgentPulse.
Reads hook JSON from stdin (Claude Code passes hook data via stdin),
normalizes it, and POSTs to the local AgentPulse event server.

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
"""
import json
import sys
import os
import urllib.request
import urllib.error

AGENTPULSE_URL = os.environ.get("AGENTPULSE_URL", "http://127.0.0.1:17878/api/events")


def post_event(data: dict) -> bool:
    """POST the event JSON to AgentPulse server."""
    body = json.dumps(data).encode("utf-8")
    req = urllib.request.Request(
        AGENTPULSE_URL,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status == 201
    except urllib.error.URLError as e:
        print(f"AgentPulse: failed to send event: {e}", file=sys.stderr)
        return False


def main():
    # Claude Code passes hook data via stdin as JSON
    raw_input = sys.stdin.read().strip()
    if not raw_input:
        print("AgentPulse: no stdin data, skipping", file=sys.stderr)
        sys.exit(0)

    hook_data = json.loads(raw_input)

    # The hook data is already the event we need -- just forward it
    # Claude Code hooks provide: session_id, cwd, hook_event_name, transcript_path, etc.
    success = post_event(hook_data)

    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
