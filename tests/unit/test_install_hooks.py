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
            assert command == 'python "/path/to/monitor.py"'


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
