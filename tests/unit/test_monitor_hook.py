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
