from pathlib import Path


RELEASE_WORKFLOW = (
    Path(__file__).resolve().parents[2] / ".github" / "workflows" / "release.yml"
)


def test_release_workflow_uses_current_tauri_action_major_tag():
    workflow = RELEASE_WORKFLOW.read_text(encoding="utf-8")

    assert "tauri-apps/tauri-action@v1" in workflow
    assert "tauri-apps/tauri-action@v2" not in workflow
