import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
TAURI_DIR = REPO_ROOT / "apps" / "desktop" / "src-tauri"


def test_tauri_bundles_generated_hook_resources():
    config = json.loads((TAURI_DIR / "tauri.conf.json").read_text(encoding="utf-8"))

    resources = config["bundle"]["resources"]

    assert resources == {
        "resources/agentpulse-hook": "agentpulse-hook",
        "resources/agentpulse-hook.exe": "agentpulse-hook.exe",
    }


def test_build_script_prepares_hook_resources_before_tauri_build():
    build_script = (TAURI_DIR / "build.rs").read_text(encoding="utf-8")

    prepare_call = build_script.index("prepare_hook_resources")
    tauri_call = build_script.index("tauri_build::build")

    assert prepare_call < tauri_call
