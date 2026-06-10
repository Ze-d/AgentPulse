//! Hook configuration management for Claude Code (JSON) and Codex (TOML).
//!
//! Supports: install, remove, status, dry-run.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CC_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
];

const CODEX_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "UserPromptSubmit",
    "Stop",
];

// ── Helpers ───────────────────────────────────────────────────

fn dirs_home() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

fn cc_default_path() -> PathBuf {
    dirs_home().join(".claude").join("settings.json")
}

fn codex_default_path() -> PathBuf {
    dirs_home().join(".codex").join("config.toml")
}

// ── Claude Code (JSON) ────────────────────────────────────────

fn build_cc_hook_configs(hook_binary: &str) -> serde_json::Value {
    let mut hooks = serde_json::Map::new();
    for event in CC_HOOK_EVENTS {
        let entry = serde_json::json!([{
            "matcher": "",
            "hooks": [
                { "type": "command", "command": hook_binary }
            ]
        }]);
        hooks.insert(event.to_string(), entry);
    }
    serde_json::Value::Object(hooks)
}

fn load_json(path: &Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}))
}

fn save_json(path: &Path, data: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let s = serde_json::to_string_pretty(data).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, &s).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

fn backup_json(path: &Path) {
    if path.exists() {
        let bak = path.with_extension("json.bak");
        let _ = std::fs::copy(path, &bak);
    }
}

fn cc_install(path: &Path, binary: &str, force: bool) -> String {
    let settings = load_json(path);
    let our = build_cc_hook_configs(binary);

    if !force {
        let existing = settings
            .get("hooks")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let all_ok = CC_HOOK_EVENTS.iter().all(|e| {
            let ours = our.get(e);
            let theirs = existing.get(e);
            ours.is_some() && ours == theirs
        });
        if all_ok {
            return "already_installed".to_string();
        }
    }

    backup_json(path);

    let existing_hooks = settings
        .get("hooks")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let mut merged = if let serde_json::Value::Object(m) = existing_hooks {
        m
    } else {
        serde_json::Map::new()
    };
    if let serde_json::Value::Object(our_map) = our {
        for (k, v) in our_map {
            merged.insert(k, v);
        }
    }

    let mut new_settings = settings;
    new_settings["hooks"] = serde_json::Value::Object(merged);
    let _ = save_json(path, &new_settings);

    "installed".to_string()
}

fn cc_remove(path: &Path) -> String {
    if !path.exists() {
        return "no_settings_file".to_string();
    }
    backup_json(path);

    let settings = load_json(path);
    if let serde_json::Value::Object(mut map) = settings {
        if let Some(hooks) = map.get_mut("hooks") {
            if let serde_json::Value::Object(hooks_map) = hooks {
                for event in CC_HOOK_EVENTS {
                    hooks_map.remove(*event);
                }
            }
        }
        let _ = save_json(path, &serde_json::Value::Object(map));
    }
    "removed".to_string()
}

fn cc_status_output(path: &Path) -> String {
    let settings = load_json(path);
    let hooks = settings
        .get("hooks")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let mut out = String::new();
    for event in CC_HOOK_EVENTS {
        let mark = if hooks.get(event).is_some() {
            "[OK]"
        } else {
            "[--]"
        };
        out.push_str(&format!("  {mark} {event}\n"));
    }
    out
}

fn cc_dry_run_output(path: &Path, binary: &str) -> String {
    let settings = load_json(path);
    let hooks = settings
        .get("hooks")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let mut out = format!("Monitor binary: {binary}\n");
    for event in CC_HOOK_EVENTS {
        if hooks.get(event).is_some() {
            out.push_str(&format!("  [SKIP] {event} (already installed)\n"));
        } else {
            out.push_str(&format!("  [WILL INSTALL] {event}\n"));
        }
    }
    out
}

// ── Codex (TOML) ──────────────────────────────────────────────

fn build_codex_hook_entry(event: &str, command: &str) -> String {
    format!(
        "{event} = [\n  {{ matcher = \"\", hooks = [\n    {{ type = \"command\", command = '{command}' }}\n  ] }}\n]"
    )
}

/// Minimal TOML section parser — handles the subset used by Codex config.toml.
fn parse_toml_sections(raw: &str) -> BTreeMap<String, Vec<String>> {
    let mut sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    sections.insert("_top".to_string(), vec![]);
    let mut current: String = "_top".to_string();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && !trimmed.starts_with("[[")
        {
            current = trimmed[1..trimmed.len() - 1].to_string();
            sections.entry(current.clone()).or_default();
        } else {
            sections
                .entry(current.clone())
                .or_default()
                .push(line.to_string());
        }
    }
    sections
}

fn find_event_start(lines: &[String], event: &str) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.trim().starts_with(&format!("{event} =")))
}

fn find_event_end(lines: &[String], start: usize) -> usize {
    for i in (start + 1)..lines.len() {
        let s = lines[i].trim();
        if s.contains('=') && !s.starts_with(&[' ', '\t', '{', ']'][..]) {
            return i;
        }
    }
    lines.len()
}

fn reassemble_toml(sections: &BTreeMap<String, Vec<String>>) -> String {
    let mut out = String::new();
    if let Some(top) = sections.get("_top") {
        for line in top {
            if !line.trim().is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (name, lines) in sections {
        if name == "_top" || lines.is_empty() {
            continue;
        }
        out.push_str(&format!("[{name}]\n"));
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn codex_install(path: &Path, binary: &str, force: bool) -> String {
    let existing_raw = if path.exists() {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };

    if !force {
        let all_ok = CODEX_HOOK_EVENTS
            .iter()
            .all(|e| existing_raw.contains(&format!("{e} =")));
        if all_ok {
            return "already_installed".to_string();
        }
    }

    // Backup
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("toml.bak"));
    }

    let escaped = binary.replace('\\', "\\\\").replace('\'', "\\'");
    let mut sections = parse_toml_sections(&existing_raw);

    let our_entries: Vec<(String, String)> = CODEX_HOOK_EVENTS
        .iter()
        .map(|e| ((*e).to_string(), build_codex_hook_entry(e, &escaped)))
        .collect();

    let hooks_lines = sections.entry("hooks".to_string()).or_default();

    // Remove existing entries for our events
    for (event, _) in &our_entries {
        while let Some(start) = find_event_start(hooks_lines, event) {
            let end = find_event_end(hooks_lines, start);
            hooks_lines.drain(start..end);
        }
    }

    // Append our entries
    for (_event, entry) in &our_entries {
        hooks_lines.push(entry.clone());
    }

    let out = reassemble_toml(&sections);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, &out).unwrap();
    "installed".to_string()
}

fn codex_remove(path: &Path) -> String {
    if !path.exists() {
        return "no_config_file".to_string();
    }
    let _ = std::fs::copy(path, path.with_extension("toml.bak"));
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut sections = parse_toml_sections(&raw);

    if let Some(lines) = sections.get_mut("hooks") {
        for event in CODEX_HOOK_EVENTS {
            while let Some(start) = find_event_start(lines, event) {
                let end = find_event_end(lines, start);
                lines.drain(start..end);
            }
        }
        // Remove trailing blank lines
        while lines.last().map_or(false, |l| l.trim().is_empty()) {
            lines.pop();
        }
    }

    let out = reassemble_toml(&sections);
    std::fs::write(path, &out).unwrap();
    "removed".to_string()
}

fn codex_status_output(path: &Path) -> String {
    let mut out = String::new();
    if !path.exists() {
        for event in CODEX_HOOK_EVENTS {
            out.push_str(&format!("  [--] {event}\n"));
        }
        return out;
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    for event in CODEX_HOOK_EVENTS {
        let mark = if raw.contains(&format!("{event} =")) {
            "[OK]"
        } else {
            "[--]"
        };
        out.push_str(&format!("  {mark} {event}\n"));
    }
    out
}

fn codex_dry_run_output(path: &Path, binary: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("Config path: {}\n", path.display()));
    out.push_str(&format!("Monitor binary: {binary}\n"));
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    for event in CODEX_HOOK_EVENTS {
        if raw.contains(&format!("{event} =")) {
            out.push_str(&format!("  [SKIP] {event} (already installed)\n"));
        } else {
            out.push_str(&format!("  [WILL INSTALL] {event}\n"));
        }
    }
    out
}

// ── Public API ────────────────────────────────────────────────

pub fn install(agent: &str, path: Option<&str>, force: bool) {
    let binary_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "agentpulse-hook".to_string());

    match agent {
        "codex" => {
            let p = path.map(PathBuf::from).unwrap_or_else(codex_default_path);
            let result = codex_install(&p, &binary_path, force);
            println!("Codex hooks {result} at {}", p.display());
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            let result = cc_install(&p, &binary_path, force);
            if result == "already_installed" {
                println!("All hooks already installed. Use --force to reinstall.");
            } else {
                for event in CC_HOOK_EVENTS {
                    println!("  [OK] {event}");
                }
                println!();
                println!("Hooks installed to {}", p.display());
            }
        }
    }
}

pub fn remove(agent: &str, path: Option<&str>) {
    match agent {
        "codex" => {
            let p = path.map(PathBuf::from).unwrap_or_else(codex_default_path);
            let result = codex_remove(&p);
            if result == "no_config_file" {
                println!("No config.toml file found. Nothing to remove.");
            } else {
                println!("AgentPulse Codex hooks removed from {}", p.display());
            }
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            let result = cc_remove(&p);
            if result == "no_settings_file" {
                println!("No settings file found. Nothing to remove.");
            } else {
                println!("AgentPulse hooks removed from {}", p.display());
            }
        }
    }
}

pub fn status(agent: &str, path: Option<&str>) {
    match agent {
        "codex" => {
            let p = path.map(PathBuf::from).unwrap_or_else(codex_default_path);
            print!("{}", codex_status_output(&p));
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            print!("{}", cc_status_output(&p));
        }
    }
}

pub fn dry_run(agent: &str, path: Option<&str>) {
    let binary_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "agentpulse-hook".to_string());

    match agent {
        "codex" => {
            let p = path.map(PathBuf::from).unwrap_or_else(codex_default_path);
            print!("{}", codex_dry_run_output(&p, &binary_path));
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            print!("{}", cc_dry_run_output(&p, &binary_path));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Claude Code tests ──

    #[test]
    fn test_cc_install_creates_file() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("settings.json");
        let result = cc_install(&p, "/usr/bin/agentpulse-hook", false);
        assert_eq!(result, "installed");
        assert!(p.exists());
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("SessionStart"));
        assert!(content.contains("agentpulse-hook"));
    }

    #[test]
    fn test_cc_install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("settings.json");
        assert_eq!(
            cc_install(&p, "/usr/bin/agentpulse-hook", false),
            "installed"
        );
        assert_eq!(
            cc_install(&p, "/usr/bin/agentpulse-hook", false),
            "already_installed"
        );
    }

    #[test]
    fn test_cc_install_preserves_other_keys() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("settings.json");
        let existing = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "permissions": {"allow": ["Read"]}
        });
        save_json(&p, &existing).unwrap();

        cc_install(&p, "/usr/bin/agentpulse-hook", false);

        let settings = load_json(&p);
        assert_eq!(settings["model"], "claude-sonnet-4-6");
        assert_eq!(settings["permissions"]["allow"][0], "Read");
        assert!(settings["hooks"]["SessionStart"].is_array());
    }

    #[test]
    fn test_cc_remove_cleans_only_our_hooks() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("settings.json");
        let existing = serde_json::json!({
            "hooks": {
                "CustomEvent": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "echo hi"}
                ]}],
                "SessionStart": [{"matcher": "", "hooks": [
                    {"type": "command", "command": "agentpulse-hook"}
                ]}]
            }
        });
        save_json(&p, &existing).unwrap();

        let result = cc_remove(&p);
        assert_eq!(result, "removed");

        let settings = load_json(&p);
        let hooks = settings["hooks"].as_object().unwrap();
        assert!(hooks.contains_key("CustomEvent"));
        assert!(!hooks.contains_key("SessionStart"));
    }

    #[test]
    fn test_cc_remove_missing_file() {
        let p = PathBuf::from("/nonexistent/settings.json");
        let result = cc_remove(&p);
        assert_eq!(result, "no_settings_file");
    }

    // ── Codex tests ──

    #[test]
    fn test_codex_install_creates_file() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("config.toml");
        let result = codex_install(&p, "/usr/bin/agentpulse-hook", false);
        assert_eq!(result, "installed");
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("SessionStart"));
        assert!(content.contains("agentpulse-hook"));
    }

    #[test]
    fn test_codex_install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("config.toml");
        assert_eq!(
            codex_install(&p, "/usr/bin/agentpulse-hook", false),
            "installed"
        );
        assert_eq!(
            codex_install(&p, "/usr/bin/agentpulse-hook", false),
            "already_installed"
        );
    }

    #[test]
    fn test_codex_install_merges_with_existing() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(
            &p,
            "model = \"gpt-5\"\n\n[hooks]\nCustomEvent = [{ matcher = \"\", hooks = [{ type = \"command\", command = \"echo hi\" }] }]\n",
        )
        .unwrap();

        codex_install(&p, "/usr/bin/agentpulse-hook", false);

        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("CustomEvent"));
        assert!(content.contains("SessionStart"));
        assert!(content.contains("model"));
    }
}
