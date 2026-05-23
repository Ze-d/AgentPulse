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
