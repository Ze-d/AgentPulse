//! Hook registration for AgentPulse → Claude Code integration.
//!
//! On every launch, this module extracts the bundled `agentpulse-hook` binary into
//! the app data directory and ensures `~/.claude/settings.json` contains the
//! 6 hook events that forward Claude Code lifecycle events to the AgentPulse
//! HTTP server.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
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
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")   // src-tauri/
        .join("..")   // desktop/
        .join("..")   // apps/
        .join("..")   // repo root
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
pub fn ensure_hooks_installed(
    settings_path: &Path,
    hook_binary: &str,
) -> Result<String, String> {
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
// Codex TOML types
// ---------------------------------------------------------------------------

/// Serializable TOML structure for a single Codex hook handler.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct CodexHookHandler {
    #[serde(rename = "type")]
    handler_type: String,
    command: String,
}

/// Serializable TOML structure for a Codex matcher group.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct CodexMatcherGroup {
    matcher: String,
    hooks: Vec<CodexHookHandler>,
}

/// Serializable TOML structure for the `[hooks]` section in `~/.codex/config.toml`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
struct CodexHooksToml {
    #[serde(rename = "SessionStart", default)]
    session_start: Vec<CodexMatcherGroup>,
    #[serde(rename = "PreToolUse", default)]
    pre_tool_use: Vec<CodexMatcherGroup>,
    #[serde(rename = "PostToolUse", default)]
    post_tool_use: Vec<CodexMatcherGroup>,
    #[serde(rename = "PermissionRequest", default)]
    permission_request: Vec<CodexMatcherGroup>,
    #[serde(rename = "UserPromptSubmit", default)]
    user_prompt_submit: Vec<CodexMatcherGroup>,
    #[serde(rename = "Stop", default)]
    stop: Vec<CodexMatcherGroup>,
}

/// Serializable TOML structure for the entire `~/.codex/config.toml` that we care about.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CodexConfigToml {
    #[serde(default)]
    hooks: Option<CodexHooksToml>,
}

// ---------------------------------------------------------------------------
// Codex TOML configuration management
// ---------------------------------------------------------------------------

/// Build the Codex hook configs object with the 6 events pointing at `hook_binary`.
fn build_codex_hook_configs(hook_binary: &str) -> CodexHooksToml {
    let escaped = hook_binary.replace('\\', "\\\\");
    let group = vec![CodexMatcherGroup {
        matcher: String::new(),
        hooks: vec![CodexHookHandler {
            handler_type: "command".to_string(),
            command: escaped.clone(),
        }],
    }];
    CodexHooksToml {
        session_start: group.clone(),
        pre_tool_use: group.clone(),
        post_tool_use: group.clone(),
        permission_request: group.clone(),
        user_prompt_submit: group.clone(),
        stop: group,
    }
}

/// Load a Codex config.toml, returning the parsed struct or an empty default.
fn load_codex_config(path: &Path) -> CodexConfigToml {
    match std::fs::read_to_string(path) {
        Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to parse codex config.toml, treating as empty"
            );
            CodexConfigToml::default()
        }),
        Err(_) => CodexConfigToml::default(),
    }
}

/// Save our Codex hooks to disk, preserving all existing TOML keys and
/// non-AgentPulse hook entries.
fn save_codex_config(path: &Path, hooks: &CodexHooksToml) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }

    // Read existing file as raw TOML value.
    let existing_raw: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or(toml::Value::Table(toml::Table::new()));

    let mut root_table = match existing_raw {
        toml::Value::Table(t) => t,
        other => {
            let mut t = toml::Table::new();
            t.insert("__previous_root".to_string(), other);
            t
        }
    };

    // Grab the existing [hooks] table, or start a new one.
    let existing_hooks = root_table
        .remove("hooks")
        .and_then(|v| match v {
            toml::Value::Table(t) => Some(t),
            _ => None,
        })
        .unwrap_or_default();

    // Merge: for each of our 6 events, set our entry, but keep others.
    let mut merged_hooks = existing_hooks;
    let hooks_str = toml::to_string(hooks).map_err(|e| format!("serialize hooks: {e}"))?;
    let our_value: toml::Value = toml::from_str(&hooks_str).map_err(|e| format!("parse hooks: {e}"))?;
    if let toml::Value::Table(our_table) = our_value {
        for (key, value) in our_table {
            let is_empty = matches!(&value, toml::Value::Array(arr) if arr.is_empty());
            if is_empty {
                merged_hooks.remove(&key);
            } else {
                merged_hooks.insert(key, value);
            }
        }
    }

    root_table.insert("hooks".to_string(), toml::Value::Table(merged_hooks));

    let new_raw = toml::Value::Table(root_table);
    let toml_str = toml::to_string_pretty(&new_raw).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, &toml_str).map_err(|e| format!("write {}: {e}", path.display()))?;
    tracing::debug!(path = %path.display(), len = toml_str.len(), "codex config.toml saved");
    Ok(())
}

/// Check if our hook definitions match the existing ones exactly.
fn hooks_are_equal(existing: &CodexHooksToml, ours: &CodexHooksToml) -> bool {
    get_codex_event_groups(existing, "SessionStart") == get_codex_event_groups(ours, "SessionStart")
        && get_codex_event_groups(existing, "PreToolUse") == get_codex_event_groups(ours, "PreToolUse")
        && get_codex_event_groups(existing, "PostToolUse") == get_codex_event_groups(ours, "PostToolUse")
        && get_codex_event_groups(existing, "PermissionRequest") == get_codex_event_groups(ours, "PermissionRequest")
        && get_codex_event_groups(existing, "UserPromptSubmit") == get_codex_event_groups(ours, "UserPromptSubmit")
        && get_codex_event_groups(existing, "Stop") == get_codex_event_groups(ours, "Stop")
}

/// Get the matcher groups for a specific Codex event by name.
fn get_codex_event_groups(hooks: &CodexHooksToml, event: &str) -> Vec<CodexMatcherGroup> {
    match event {
        "SessionStart" => hooks.session_start.clone(),
        "PreToolUse" => hooks.pre_tool_use.clone(),
        "PostToolUse" => hooks.post_tool_use.clone(),
        "PermissionRequest" => hooks.permission_request.clone(),
        "UserPromptSubmit" => hooks.user_prompt_submit.clone(),
        "Stop" => hooks.stop.clone(),
        _ => vec![],
    }
}

/// Check whether a Codex event has any registered hooks.
fn has_codex_event(hooks: &CodexHooksToml, event: &str) -> bool {
    !get_codex_event_groups(hooks, event).is_empty()
}

/// Merge our hook definitions into the existing ones.  For events we manage,
/// our definitions win; other events are left untouched.
fn merge_codex_hooks(existing: &CodexHooksToml, ours: &CodexHooksToml) -> CodexHooksToml {
    CodexHooksToml {
        session_start: if ours.session_start.is_empty() { existing.session_start.clone() } else { ours.session_start.clone() },
        pre_tool_use: if ours.pre_tool_use.is_empty() { existing.pre_tool_use.clone() } else { ours.pre_tool_use.clone() },
        post_tool_use: if ours.post_tool_use.is_empty() { existing.post_tool_use.clone() } else { ours.post_tool_use.clone() },
        permission_request: if ours.permission_request.is_empty() { existing.permission_request.clone() } else { ours.permission_request.clone() },
        user_prompt_submit: if ours.user_prompt_submit.is_empty() { existing.user_prompt_submit.clone() } else { ours.user_prompt_submit.clone() },
        stop: if ours.stop.is_empty() { existing.stop.clone() } else { ours.stop.clone() },
    }
}

/// Ensure all 6 AgentPulse Codex hooks are present and point to `hook_binary`.
///
/// Returns `"already_ok"`, `"installed"`, or `"updated"`.
pub fn ensure_codex_hooks_installed(
    config_path: &Path,
    hook_binary: &str,
) -> Result<String, String> {
    let config = load_codex_config(config_path);
    let our_hooks = build_codex_hook_configs(hook_binary);

    let existing = config.hooks.clone().unwrap_or_default();

    if hooks_are_equal(&existing, &our_hooks) {
        return Ok("already_ok".to_string());
    }

    let merged = merge_codex_hooks(&existing, &our_hooks);

    save_codex_config(config_path, &merged)?;

    let had_any = CODEX_HOOK_EVENTS.iter().any(|e| has_codex_event(&existing, e));

    if had_any {
        tracing::info!(path = %config_path.display(), "Codex AgentPulse hooks updated");
        Ok("updated".to_string())
    } else {
        tracing::info!(path = %config_path.display(), "Codex AgentPulse hooks installed");
        Ok("installed".to_string())
    }
}

/// Remove the 6 AgentPulse Codex hook events from config.toml.
pub fn unregister_codex_hooks(config_path: &Path) -> Result<String, String> {
    if !config_path.exists() {
        return Ok("no_config_file".to_string());
    }

    // Read and manipulate at the TOML level to cleanly remove our keys.
    let raw = std::fs::read_to_string(config_path)
        .map_err(|e| format!("read {}: {e}", config_path.display()))?;
    let mut root: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("parse {}: {e}", config_path.display()))?;

    if let toml::Value::Table(ref mut root_table) = root {
        if let Some(toml::Value::Table(ref mut hooks_table)) = root_table.get_mut("hooks") {
            for event in &CODEX_HOOK_EVENTS {
                hooks_table.remove(*event);
            }
        }
    }

    let toml_str = toml::to_string_pretty(&root).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(config_path, &toml_str)
        .map_err(|e| format!("write {}: {e}", config_path.display()))?;

    Ok("removed".to_string())
}

/// Return `{event_name: bool}` indicating which Codex hooks are installed.
pub fn get_codex_hook_status(config_path: &Path) -> Result<HashMap<String, bool>, String> {
    let config = load_codex_config(config_path);
    let hooks = config.hooks.unwrap_or_default();

    let mut status = HashMap::new();
    for event in &CODEX_HOOK_EVENTS {
        status.insert(event.to_string(), has_codex_event(&hooks, event));
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
        assert!(cmd.contains("agentpulse-hook"), "expected binary in command");
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

    // ── Codex TOML tests ──

    #[test]
    fn test_codex_hook_config_serializes_to_toml() {
        let config = build_codex_hook_configs("/usr/local/bin/agentpulse-hook");
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("SessionStart"));
        assert!(toml_str.contains("PreToolUse"));
        assert!(toml_str.contains("PostToolUse"));
        assert!(toml_str.contains("PermissionRequest"));
        assert!(toml_str.contains("UserPromptSubmit"));
        assert!(toml_str.contains("Stop"));
        assert!(toml_str.contains("agentpulse-hook"));
    }

    #[test]
    fn test_codex_install_merges_with_existing_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        let existing = r#"
model = "gpt-5"

[hooks]
SomeOtherEvent = [
    { matcher = "", hooks = [{ type = "command", command = "echo hi" }] }
]
"#;
        std::fs::write(&config_path, existing).unwrap();

        let result = ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook");
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("SomeOtherEvent"));
        assert!(content.contains("SessionStart"));
        assert!(content.contains("model"));
    }

    #[test]
    fn test_codex_install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        assert_eq!(
            ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook").unwrap(),
            "installed"
        );
        assert_eq!(
            ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook").unwrap(),
            "already_ok"
        );
    }

    #[test]
    fn test_codex_get_hook_status() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        // Empty — all false
        let status = get_codex_hook_status(&config_path).unwrap();
        assert_eq!(status.len(), 6);
        for v in status.values() {
            assert!(!v);
        }

        // After install — all true
        ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook").unwrap();
        let status = get_codex_hook_status(&config_path).unwrap();
        for v in status.values() {
            assert!(v);
        }
    }

    #[test]
    fn test_codex_unregister_removes_only_our_hooks() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        let existing = r#"
[hooks]
CustomEvent = [{ matcher = "", hooks = [{ type = "command", command = "echo hi" }] }]
"#;
        std::fs::write(&config_path, existing).unwrap();

        ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook").unwrap();

        let result = unregister_codex_hooks(&config_path);
        assert_eq!(result.unwrap(), "removed");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("CustomEvent"));
        assert!(!content.contains("SessionStart"));
    }

    #[test]
    fn test_codex_unregister_missing_file() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("nonexistent.toml");
        let result = unregister_codex_hooks(&config_path);
        assert_eq!(result.unwrap(), "no_config_file");
    }
}
