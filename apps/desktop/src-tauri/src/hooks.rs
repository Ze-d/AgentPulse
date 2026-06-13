//! Hook registration for AgentPulse → Claude Code integration.
//!
//! On every launch, this module extracts the bundled `agentpulse-hook` binary into
//! the app data directory and ensures `~/.claude/settings.json` contains the
//! 6 hook events that forward Claude Code lifecycle events to the AgentPulse
//! HTTP server.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// The 6 Claude Code hook events that AgentPulse subscribes to.
const HOOK_EVENTS: [&str; 6] = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
];

/// The 6 Codex hook events that AgentPulse subscribes to.
const CODEX_HOOK_EVENTS: [&str; 6] = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "UserPromptSubmit",
    "Stop",
];

// ---------------------------------------------------------------------------
// Locate monitor script
// ---------------------------------------------------------------------------

/// Locate `agentpulse-hook` binary on disk.
///
/// In bundled (release) mode the binary lives in the resource directory. In
/// dev mode we fall back to the hook-adapter build output.
pub fn find_hook_binary(resource_dir: &Path) -> Result<PathBuf, String> {
    let bin_name = if cfg!(target_os = "windows") {
        "agentpulse-hook.exe"
    } else {
        "agentpulse-hook"
    };

    let bundled = resource_dir.join(bin_name);
    if bundled.exists() {
        tracing::debug!(path = %bundled.display(), "hook binary found in resource dir");
        return Ok(bundled);
    }

    // Dev fallback: look in hook-adapter target directory.
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..") // src-tauri/
        .join("..") // desktop/
        .join("..") // apps/
        .join("..") // repo root
        .join("adapters")
        .join("hook-adapter")
        .join("target")
        .join(profile)
        .join(bin_name);
    if dev_path.exists() {
        tracing::debug!(path = %dev_path.display(), "hook binary found in dev path");
        return Ok(dev_path);
    }

    Err(format!(
        "agentpulse-hook binary not found in resource_dir ({}) or dev path ({})",
        bundled.display(),
        dev_path.display()
    ))
}

// ---------------------------------------------------------------------------
// Extract script into persistent location
// ---------------------------------------------------------------------------

/// Copy `agentpulse-hook` binary into `app_data_dir`, overwriting only when
/// the source is newer. Returns the destination path.
pub fn extract_hook_binary(resource_dir: &Path, app_data_dir: &Path) -> Result<PathBuf, String> {
    let src = find_hook_binary(resource_dir)?;

    fs::create_dir_all(app_data_dir).map_err(|e| format!("create app_data_dir: {e}"))?;

    let bin_name = if cfg!(target_os = "windows") {
        "agentpulse-hook.exe"
    } else {
        "agentpulse-hook"
    };
    let dst = app_data_dir.join(bin_name);

    // Only copy if the source is newer (or destination missing).
    let should_copy = match (fs::metadata(&src), fs::metadata(&dst)) {
        (Ok(src_meta), Ok(dst_meta)) => src_meta.modified().ok() > dst_meta.modified().ok(),
        (Ok(_), Err(_)) => true, // destination missing
        _ => false,
    };

    if should_copy {
        fs::copy(&src, &dst).map_err(|e| format!("copy agentpulse-hook: {e}"))?;
        tracing::info!(src = %src.display(), dst = %dst.display(), "agentpulse-hook extracted");

        // Ensure executable permissions on Unix.
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&dst) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&dst, perms);
            }
        }
    }

    Ok(dst)
}

// ---------------------------------------------------------------------------
// Build hook configs
// ---------------------------------------------------------------------------

/// Build the `hooks` object for all 6 events pointing at `hook_binary`.
fn build_hook_configs(hook_binary: &str) -> Value {
    let mut hooks = serde_json::Map::new();
    let escaped = hook_binary.replace('\\', "\\\\");

    for event in &HOOK_EVENTS {
        let entry = json!([{
            "matcher": "",
            "hooks": [
                { "type": "command", "command": escaped }
            ]
        }]);
        hooks.insert(event.to_string(), entry);
    }

    Value::Object(hooks)
}

// ---------------------------------------------------------------------------
// Settings file read / write
// ---------------------------------------------------------------------------

fn load_settings(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to parse settings.json, treating as empty"
            );
            json!({})
        }),
        Err(_) => json!({}),
    }
}

fn save_settings(path: &Path, data: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let json_str = serde_json::to_string_pretty(data).map_err(|e| format!("serialize: {e}"))?;
    fs::write(path, &json_str).map_err(|e| format!("write {}: {e}", path.display()))?;
    tracing::debug!(path = %path.display(), len = json_str.len(), "settings saved");
    Ok(())
}

fn backup_settings(path: &Path) {
    if path.exists() {
        let bak = path.with_extension("json.bak");
        if let Err(e) = fs::copy(path, &bak) {
            tracing::warn!(
                path = %path.display(),
                backup = %bak.display(),
                error = %e,
                "failed to backup settings.json"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Core public API
// ---------------------------------------------------------------------------

/// Ensure all 6 AgentPulse hooks are present and point to `hook_binary`.
///
/// Returns:
/// - `"already_ok"` — hooks are correct, nothing changed
/// - `"installed"` — hooks were missing and have been added
/// - `"updated"` — hooks existed but pointed to a stale path, now fixed
pub fn ensure_hooks_installed(settings_path: &Path, hook_binary: &str) -> Result<String, String> {
    let settings = load_settings(settings_path);
    let existing_hooks = settings.get("hooks").cloned().unwrap_or(json!({}));
    let our_config = build_hook_configs(hook_binary);

    // Check whether all 6 events already have the correct command path.
    let mut all_ok = true;
    for event in &HOOK_EVENTS {
        let ours = our_config.get(event);
        let theirs = existing_hooks.get(event);

        let ok = match (ours, theirs) {
            (Some(o), Some(t)) => o == t,
            _ => false,
        };

        if !ok {
            all_ok = false;
            break;
        }
    }

    if all_ok {
        return Ok("already_ok".to_string());
    }

    // Merge: keep all non-AgentPulse hooks, overwrite / insert our 6 events.
    backup_settings(settings_path);

    let mut merged_hooks = if let Value::Object(ref map) = existing_hooks {
        map.clone()
    } else {
        serde_json::Map::new()
    };

    if let Value::Object(our_map) = our_config {
        for (k, v) in our_map {
            merged_hooks.insert(k, v);
        }
    }

    let mut new_settings = settings.clone();
    new_settings["hooks"] = Value::Object(merged_hooks);

    save_settings(settings_path, &new_settings)?;

    let had_any = HOOK_EVENTS.iter().any(|e| existing_hooks.get(e).is_some());
    if had_any {
        tracing::info!(path = %settings_path.display(), "AgentPulse hooks updated (path changed)");
        Ok("updated".to_string())
    } else {
        tracing::info!(path = %settings_path.display(), "AgentPulse hooks installed");
        Ok("installed".to_string())
    }
}

/// Remove the 6 AgentPulse hook events from settings.json, preserving
/// everything else.
pub fn unregister_hooks(settings_path: &Path) -> Result<String, String> {
    if !settings_path.exists() {
        return Ok("no_settings_file".to_string());
    }

    backup_settings(settings_path);

    let settings = load_settings(settings_path);
    let existing_hooks = settings.get("hooks").cloned().unwrap_or(json!({}));

    if let Value::Object(mut map) = existing_hooks {
        for event in &HOOK_EVENTS {
            map.remove(*event);
        }
        let mut new_settings = settings.clone();
        new_settings["hooks"] = Value::Object(map);
        save_settings(settings_path, &new_settings)?;
    }

    Ok("removed".to_string())
}

/// Return `{event_name: bool}` indicating which hooks are installed with the
/// correct monitor script path.
pub fn get_hook_status(settings_path: &Path) -> Result<HashMap<String, bool>, String> {
    let settings = load_settings(settings_path);
    let hooks = settings.get("hooks").cloned().unwrap_or(json!({}));

    let mut status = HashMap::new();
    for event in &HOOK_EVENTS {
        status.insert(event.to_string(), hooks.get(event).is_some());
    }
    Ok(status)
}

// ---------------------------------------------------------------------------
// Codex TOML configuration management (delegated to agentpulse-hook binary)
// ---------------------------------------------------------------------------

/// Run `agentpulse-hook --agent codex <subcommand>` and return (exit_code, stdout, stderr).
fn run_codex_hook_cmd(
    hook_binary: &str,
    config_path: Option<&Path>,
    subcommand: &str,
) -> Result<(i32, String, String), String> {
    let mut cmd = std::process::Command::new(hook_binary);
    cmd.args(["--agent", "codex"]);
    if let Some(p) = config_path {
        cmd.args(["--path", &p.to_string_lossy()]);
    }
    cmd.arg(subcommand);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run agentpulse-hook: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    tracing::debug!(
        exit_code,
        subcommand,
        stdout = %stdout.trim(),
        stderr = %stderr.trim(),
        "agentpulse-hook codex command completed"
    );

    Ok((exit_code, stdout, stderr))
}

/// Ensure all 6 AgentPulse Codex hooks are present and point to `hook_binary`.
///
/// Delegates to `agentpulse-hook --agent codex install`, which is idempotent
/// and preserves existing non-AgentPulse hooks.  Returns `"installed"` or an
/// error message.
pub fn ensure_codex_hooks_installed(
    config_path: &Path,
    hook_binary: &str,
) -> Result<String, String> {
    let (code, stdout, stderr) =
        run_codex_hook_cmd(hook_binary, Some(config_path), "install")?;

    if code == 0 {
        tracing::info!(path = %config_path.display(), "Codex AgentPulse hooks ensured");
        Ok(stdout.trim().to_string())
    } else {
        let msg = format!("codex install failed (exit {code}): {stderr}");
        tracing::error!(%msg);
        Err(msg)
    }
}

/// Remove the 6 AgentPulse Codex hook events from config.toml.
///
/// Delegates to `agentpulse-hook --agent codex remove`.
pub fn unregister_codex_hooks(config_path: &Path, hook_binary: &str) -> Result<String, String> {
    let (code, stdout, stderr) =
        run_codex_hook_cmd(hook_binary, Some(config_path), "remove")?;

    if code == 0 {
        tracing::info!(path = %config_path.display(), "Codex AgentPulse hooks removed");
        Ok(stdout.trim().to_string())
    } else {
        let msg = format!("codex remove failed (exit {code}): {stderr}");
        tracing::error!(%msg);
        Err(msg)
    }
}

/// Return `{event_name: bool}` indicating which Codex hooks are installed.
///
/// Delegates to `agentpulse-hook --agent codex status` and parses the output.
pub fn get_codex_hook_status(
    config_path: &Path,
    hook_binary: &str,
) -> Result<HashMap<String, bool>, String> {
    let (code, stdout, stderr) =
        run_codex_hook_cmd(hook_binary, Some(config_path), "status")?;

    if code != 0 {
        return Err(format!("codex status failed (exit {code}): {stderr}"));
    }

    let mut status = HashMap::new();
    for event in &CODEX_HOOK_EVENTS {
        status.insert(event.to_string(), false);
    }

    // Parse lines like: "  [OK] SessionStart" or "  [--] PreToolUse"
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("[OK]") {
            if let Some(event) = line.strip_prefix("[OK]").map(|s| s.trim()) {
                status.insert(event.to_string(), true);
            }
        } else if line.starts_with("[--]") {
            if let Some(event) = line.strip_prefix("[--]").map(|s| s.trim()) {
                status.insert(event.to_string(), false);
            }
        }
    }

    Ok(status)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_settings_path(dir: &TempDir) -> PathBuf {
        dir.path().join("settings.json")
    }

    #[test]
    fn test_build_hook_configs_has_all_events() {
        let config = build_hook_configs("/tmp/agentpulse-hook");
        for event in &HOOK_EVENTS {
            assert!(
                config.get(event).is_some(),
                "missing hook config for {event}"
            );
        }
    }

    #[test]
    fn test_build_hook_configs_contains_command() {
        let config = build_hook_configs("C:\\app\\agentpulse-hook.exe");
        let session_start = &config["SessionStart"][0]["hooks"][0]["command"];
        let cmd = session_start.as_str().unwrap();
        assert!(
            cmd.contains("agentpulse-hook"),
            "expected binary in command"
        );
    }

    #[test]
    fn test_install_creates_settings_when_missing() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);
        let hook_binary = "/app/agentpulse-hook";

        let result = ensure_hooks_installed(&settings_path, hook_binary);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status, "installed");

        let settings = load_settings(&settings_path);
        let hooks = settings["hooks"].as_object().unwrap();
        assert_eq!(hooks.len(), 6);
    }

    #[test]
    fn test_install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);
        let hook_binary = "/app/agentpulse-hook";

        // First call installs.
        assert_eq!(
            ensure_hooks_installed(&settings_path, hook_binary).unwrap(),
            "installed"
        );
        // Second call is no-op.
        assert_eq!(
            ensure_hooks_installed(&settings_path, hook_binary).unwrap(),
            "already_ok"
        );
    }

    #[test]
    fn test_install_preserves_other_hooks() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);

        // Pre-populate with a custom hook.
        let existing = json!({
            "hooks": {
                "CustomEvent": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "echo hello"}
                ]}]
            }
        });
        save_settings(&settings_path, &existing).unwrap();

        let result = ensure_hooks_installed(&settings_path, "/app/agentpulse-hook");
        assert!(result.is_ok());

        let settings = load_settings(&settings_path);
        let hooks = settings["hooks"].as_object().unwrap();
        // 6 ours + 1 custom = 7
        assert_eq!(hooks.len(), 7);
        assert!(hooks.contains_key("CustomEvent"));
        assert!(hooks.contains_key("SessionStart"));
    }

    #[test]
    fn test_install_updates_stale_path() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);

        // Install with old path.
        ensure_hooks_installed(&settings_path, "/old/path/agentpulse-hook").unwrap();

        // Now call with new path — should update.
        let result = ensure_hooks_installed(&settings_path, "/new/path/agentpulse-hook");
        assert_eq!(result.unwrap(), "updated");

        let settings = load_settings(&settings_path);
        let cmd = &settings["hooks"]["SessionStart"][0]["hooks"][0]["command"];
        assert!(cmd.as_str().unwrap().contains("/new/path/"));
    }

    #[test]
    fn test_install_preserves_non_hooks_keys() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);

        let existing = json!({
            "permissions": { "allow": ["Read"] },
            "model": "claude-sonnet-4-6"
        });
        save_settings(&settings_path, &existing).unwrap();

        ensure_hooks_installed(&settings_path, "/app/agentpulse-hook").unwrap();

        let settings = load_settings(&settings_path);
        assert_eq!(settings["model"], "claude-sonnet-4-6");
        let perms = &settings["permissions"]["allow"][0];
        assert_eq!(perms, "Read");
        assert!(settings["hooks"]
            .as_object()
            .unwrap()
            .contains_key("SessionStart"));
    }

    #[test]
    fn test_unregister_removes_only_our_hooks() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);

        let existing = json!({
            "hooks": {
                "CustomEvent": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "echo hello"}
                ]}],
                "SessionStart": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "/x/agentpulse-hook"}
                ]}]
            }
        });
        save_settings(&settings_path, &existing).unwrap();

        let result = unregister_hooks(&settings_path);
        assert_eq!(result.unwrap(), "removed");

        let settings = load_settings(&settings_path);
        let hooks = settings["hooks"].as_object().unwrap();
        assert_eq!(hooks.len(), 1);
        assert!(hooks.contains_key("CustomEvent"));
        assert!(!hooks.contains_key("SessionStart"));
    }

    #[test]
    fn test_unregister_missing_file() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);
        let result = unregister_hooks(&settings_path);
        assert_eq!(result.unwrap(), "no_settings_file");
    }

    #[test]
    fn test_get_hook_status() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);

        // Empty — all false.
        let status = get_hook_status(&settings_path).unwrap();
        assert_eq!(status.len(), 6);
        for v in status.values() {
            assert!(!v);
        }

        // After install — all true.
        ensure_hooks_installed(&settings_path, "/app/agentpulse-hook").unwrap();
        let status = get_hook_status(&settings_path).unwrap();
        for v in status.values() {
            assert!(v);
        }
    }

    #[test]
    fn test_backup_created() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);
        let bak_path = settings_path.with_extension("json.bak");

        // No backup if file didn't exist.
        ensure_hooks_installed(&settings_path, "/app/agentpulse-hook").unwrap();
        assert!(!bak_path.exists(), "backup should not exist for new file");

        // Simulate update — backup should be created.
        let existing = json!({
            "hooks": {
                "SessionStart": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "/old/agentpulse-hook"}
                ]}]
            }
        });
        save_settings(&settings_path, &existing).unwrap();

        ensure_hooks_installed(&settings_path, "/new/agentpulse-hook").unwrap();
        assert!(bak_path.exists(), "backup should exist after update");
    }

    // Codex TOML management now delegates to the agentpulse-hook binary.
    // Integration tests live in adapters/hook-adapter/src/installer.rs.
}
