from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"


def test_ci_installs_linux_tauri_system_dependencies_before_cargo():
    content = CI_WORKFLOW.read_text(encoding="utf-8")

    linux_deps_step = "Install Linux dependencies"
    cargo_clippy_step = "Cargo clippy"
    assert linux_deps_step in content
    assert content.index(linux_deps_step) < content.index(cargo_clippy_step)

    assert "if: runner.os == 'Linux'" in content
    for package in ("pkg-config", "libglib2.0-dev", "libgtk-3-dev", "libwebkit2gtk-4.1-dev"):
        assert package in content
