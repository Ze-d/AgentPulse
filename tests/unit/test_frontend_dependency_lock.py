import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DESKTOP_LOCK = ROOT / "apps" / "desktop" / "package-lock.json"
WORKFLOWS = [
    ROOT / ".github" / "workflows" / "ci.yml",
    ROOT / ".github" / "workflows" / "release.yml",
]


def test_vitest_mocker_estree_walker_is_locked():
    lock = json.loads(DESKTOP_LOCK.read_text(encoding="utf-8"))

    assert "node_modules/@vitest/mocker/node_modules/estree-walker" in lock["packages"]


def test_github_workflows_use_node_20_for_frontend_dependencies():
    for workflow in WORKFLOWS:
        content = workflow.read_text(encoding="utf-8")

        assert "node-version: 18" not in content
        assert "node-version: 20" in content
