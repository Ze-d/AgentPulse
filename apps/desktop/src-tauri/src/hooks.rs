//! Hook registration for AgentPulse → Claude Code integration.
//!
//! On every launch, this module extracts the bundled `monitor_hook.py` into
//! the app data directory and ensures `~/.claude/settings.json` contains the
//! 6 hook events that forward Claude Code lifecycle events to the AgentPulse
//! HTTP server.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use log;
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

// ---------------------------------------------------------------------------
// Locate monitor script
// ---------------------------------------------------------------------------

/// Locate `monitor_hook.py` on disk.
///
/// In bundled (release) mode the file lives in the resource directory. In dev
/// mode we fall back to the source tree under `adapters/claude-code/`.
pub fn find_monitor_script(resource_dir: &Path) -> Result<PathBuf, String> {
    let bundled = resource_dir.join("monitor_hook.py");
    if bundled.exists() {
        return Ok(bundled);
    }

    // Dev fallback: resolve relative to the Cargo manifest directory.
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..") // src-tauri/
        .join("..") // desktop/
        .join("..") // apps/
        .join("..") // repo root
        .join("adapters")
        .join("claude-code")
        .join("monitor_hook.py");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    Err(format!(
        "monitor_hook.py not found in resource_dir ({}) or dev path ({})",
        bundled.display(),
        dev_path.display()
    ))
}

// ---------------------------------------------------------------------------
// Extract script into persistent location
// ---------------------------------------------------------------------------

/// Copy `monitor_hook.py` into `app_data_dir`, overwriting only when the
/// source is newer. Returns the destination path.
pub fn extract_monitor_script(resource_dir: &Path, app_data_dir: &Path) -> Result<PathBuf, String> {
    let src = find_monitor_script(resource_dir)?;

    fs::create_dir_all(app_data_dir).map_err(|e| format!("create app_data_dir: {e}"))?;

    let dst = app_data_dir.join("monitor_hook.py");

    // Only copy if the source is newer (or destination missing).
    let should_copy = match (fs::metadata(&src), fs::metadata(&dst)) {
        (Ok(src_meta), Ok(dst_meta)) => src_meta.modified().ok() > dst_meta.modified().ok(),
        (Ok(_), Err(_)) => true, // destination missing
        _ => false,
    };

    if should_copy {
        fs::copy(&src, &dst).map_err(|e| format!("copy monitor_hook.py: {e}"))?;
        log::info!("monitor_hook.py extracted to {}", dst.display());
    }

    Ok(dst)
}

// ---------------------------------------------------------------------------
// Build hook configs
// ---------------------------------------------------------------------------

/// Build the `hooks` object for all 6 events pointing at `monitor_script`.
fn build_hook_configs(monitor_script: &str) -> Value {
    let mut hooks = serde_json::Map::new();
    let command = format!("python \"{monitor_script}\"");

    for event in &HOOK_EVENTS {
        let entry = json!([{
            "matcher": "",
            "hooks": [
                { "type": "command", "command": command }
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
            log::warn!("Failed to parse {}: {e}, treating as empty", path.display());
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
    fs::write(path, json_str).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

fn backup_settings(path: &Path) {
    if path.exists() {
        let bak = path.with_extension("json.bak");
        if let Err(e) = fs::copy(path, &bak) {
            log::warn!("Failed to backup settings.json: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Core public API
// ---------------------------------------------------------------------------

/// Ensure all 6 AgentPulse hooks are present and point to `monitor_script`.
///
/// Returns:
/// - `"already_ok"` — hooks are correct, nothing changed
/// - `"installed"` — hooks were missing and have been added
/// - `"updated"` — hooks existed but pointed to a stale path, now fixed
pub fn ensure_hooks_installed(
    settings_path: &Path,
    monitor_script: &str,
) -> Result<String, String> {
    let settings = load_settings(settings_path);
    let existing_hooks = settings.get("hooks").cloned().unwrap_or(json!({}));
    let our_config = build_hook_configs(monitor_script);

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
        log::info!("AgentPulse hooks updated (path changed)");
        Ok("updated".to_string())
    } else {
        log::info!("AgentPulse hooks installed to {}", settings_path.display());
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
        let config = build_hook_configs("/tmp/monitor_hook.py");
        for event in &HOOK_EVENTS {
            assert!(
                config.get(event).is_some(),
                "missing hook config for {event}"
            );
        }
    }

    #[test]
    fn test_build_hook_configs_contains_command() {
        let config = build_hook_configs("C:\\app\\monitor_hook.py");
        let session_start = &config["SessionStart"][0]["hooks"][0]["command"];
        let cmd = session_start.as_str().unwrap();
        assert!(cmd.contains("python"), "expected python in command");
        assert!(cmd.contains("monitor_hook.py"), "expected script path");
    }

    #[test]
    fn test_install_creates_settings_when_missing() {
        let dir = TempDir::new().unwrap();
        let settings_path = temp_settings_path(&dir);
        let monitor_script = "/app/monitor_hook.py";

        let result = ensure_hooks_installed(&settings_path, monitor_script);
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
        let monitor_script = "/app/monitor_hook.py";

        // First call installs.
        assert_eq!(
            ensure_hooks_installed(&settings_path, monitor_script).unwrap(),
            "installed"
        );
        // Second call is no-op.
        assert_eq!(
            ensure_hooks_installed(&settings_path, monitor_script).unwrap(),
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

        let result = ensure_hooks_installed(&settings_path, "/app/monitor_hook.py");
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
        ensure_hooks_installed(&settings_path, "/old/path/monitor_hook.py").unwrap();

        // Now call with new path — should update.
        let result = ensure_hooks_installed(&settings_path, "/new/path/monitor_hook.py");
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

        ensure_hooks_installed(&settings_path, "/app/monitor_hook.py").unwrap();

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
                    {"type": "command", "command": "python /x/monitor_hook.py"}
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
        ensure_hooks_installed(&settings_path, "/app/monitor_hook.py").unwrap();
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
        ensure_hooks_installed(&settings_path, "/app/monitor_hook.py").unwrap();
        assert!(!bak_path.exists(), "backup should not exist for new file");

        // Simulate update — backup should be created.
        let existing = json!({
            "hooks": {
                "SessionStart": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "python /old/monitor_hook.py"}
                ]}]
            }
        });
        save_settings(&settings_path, &existing).unwrap();

        ensure_hooks_installed(&settings_path, "/new/monitor_hook.py").unwrap();
        assert!(bak_path.exists(), "backup should exist after update");
    }
}
