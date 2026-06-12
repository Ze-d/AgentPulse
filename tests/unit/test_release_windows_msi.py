from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"


def test_release_windows_build_explicitly_requests_msi_bundle():
    content = RELEASE_WORKFLOW.read_text(encoding="utf-8")

    windows_target = "target: 'x86_64-pc-windows-msvc'"
    assert windows_target in content

    windows_entry = content[content.index(windows_target) :]
    next_platform = windows_entry.find("\n          - platform:", 1)
    if next_platform != -1:
        windows_entry = windows_entry[:next_platform]

    assert "bundles: 'nsis,msi'" in windows_entry
    assert "--bundles ${{ matrix.bundles }}" in content


def test_release_workflow_fails_when_windows_msi_is_missing():
    content = RELEASE_WORKFLOW.read_text(encoding="utf-8")

    assert "uses: tauri-apps/tauri-action@v0" in content
    assert "name: Verify Windows MSI bundle" in content
    assert "target/*/release/bundle/msi/*.msi" in content
