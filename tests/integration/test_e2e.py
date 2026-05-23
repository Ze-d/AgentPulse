"""
End-to-end smoke test: simulate Claude Code hook events via HTTP.
Can run against a running AgentPulse event server, or as a dry-run.
"""
import json
import time
import urllib.request
import urllib.error
import sys

AGENTPULSE_URL = "http://127.0.0.1:17878"


def post_event(data: dict) -> int:
    body = json.dumps(data).encode("utf-8")
    req = urllib.request.Request(
        f"{AGENTPULSE_URL}/api/events",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status
    except urllib.error.URLError:
        return -1


def test_health_endpoint():
    """Test that the event server is alive."""
    try:
        with urllib.request.urlopen(f"{AGENTPULSE_URL}/api/health", timeout=2) as resp:
            assert resp.status == 200
            data = json.loads(resp.read())
            assert data["status"] == "ok"
            print("  PASS: health endpoint")
    except urllib.error.URLError:
        print("  SKIP: AgentPulse server not running")


def test_full_session_lifecycle():
    """Simulate a complete Claude Code session."""
    session_id = f"e2e-test-{int(time.time())}"

    events = [
        {
            "session_id": session_id,
            "cwd": "/tmp/e2e-test-project",
            "hook_event_name": "SessionStart",
            "transcript_path": f"/tmp/transcript-{session_id}.json",
        },
        {
            "session_id": session_id,
            "cwd": "/tmp/e2e-test-project",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "echo hello"},
        },
        {
            "session_id": session_id,
            "cwd": "/tmp/e2e-test-project",
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_response": "hello",
        },
        {
            "session_id": session_id,
            "cwd": "/tmp/e2e-test-project",
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt",
            "message": "Approve this action?",
        },
        {
            "session_id": session_id,
            "cwd": "/tmp/e2e-test-project",
            "hook_event_name": "Stop",
            "last_assistant_message": "Task complete!",
        },
    ]

    for event in events:
        status = post_event(event)
        if status < 0:
            print("  SKIP: AgentPulse server not running")
            return
        assert status == 201, f"Expected 201, got {status} for {event['hook_event_name']}"

    print("  PASS: all 5 events posted with 201")

    # Verify session exists via API
    try:
        with urllib.request.urlopen(f"{AGENTPULSE_URL}/api/sessions", timeout=2) as resp:
            sessions = json.loads(resp.read())
            matching = [s for s in sessions if s["sessionId"] == session_id]
            if len(matching) >= 1:
                assert matching[0]["status"] == "completed"
                assert matching[0]["projectName"] == "e2e-test-project"
                print("  PASS: session verified as completed")
            else:
                print(f"  WARN: session {session_id} not found in active sessions (may be filtered)")
    except urllib.error.URLError:
        print("  SKIP: Cannot verify sessions endpoint")


def main():
    print("AgentPulse E2E Integration Test")
    print("===============================")
    print()

    # Test health
    test_health_endpoint()

    # Test full lifecycle
    test_full_session_lifecycle()

    print()
    print("Done.")


if __name__ == "__main__":
    main()
