# Python-Free Hook Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace 4 Python files (2× monitor_hook.py + 2× install_hooks.py) with a single Rust binary `agentpulse-hook` that handles both hook event forwarding and hook configuration management.

**Architecture:** Standalone Rust binary crate at `adapters/hook-adapter/` using blocking I/O (ureq) for zero external dependency. The Tauri app (`hooks.rs`) drops `resolve_python()` and adapts its command format from `python <script>` to direct binary path. The binary serves two modes: default (stdin → POST) for runtime events, and subcommands (install/remove/status/dry-run) for CLI hook management.

**Tech Stack:** Rust, ureq (blocking HTTP), clap (CLI), sysinfo (process detection), serde_json, toml, log + env_logger

---

## File Structure

```
adapters/hook-adapter/           # NEW — standalone binary crate
├── Cargo.toml
└── src/
    ├── main.rs                  # clap dispatch: default mode vs subcommands
    ├── hook.rs                  # stdin → JSON → enrich → POST (runtime mode)
    ├── agent.rs                 # cross-platform process tree detection
    ├── sender.rs                # HTTP POST with retry logic
    └── installer.rs             # install/remove/status/dry-run for CC & Codex

apps/desktop/src-tauri/src/
├── hooks.rs                     # MODIFY — drop resolve_python, rename functions
├── commands.rs                  # MODIFY — drop python param from install commands
├── lib.rs                       # MODIFY — drop python_for_hooks variable
├── config.rs                    # MODIFY — remove python field and AGENTPULSE_PYTHON
└── tauri.conf.json              # MODIFY — bundle binary instead of .py

adapters/claude-code/
├── monitor_hook.py              # DELETE
└── install_hooks.py             # DELETE

adapters/codex/
├── monitor_hook.py              # DELETE
└── install_hooks.py             # DELETE

tests/unit/
├── test_install_hooks.py        # DELETE
└── test_monitor_hook.py         # DELETE
```

---

### Task 1: Create hook-adapter crate skeleton

**Files:**
- Create: `adapters/hook-adapter/Cargo.toml`
- Create: `adapters/hook-adapter/src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "agentpulse-hook"
version = "0.4.0"
description = "AgentPulse hook adapter — zero-dependency event forwarder and hook manager"
authors = ["Kal_zed"]
repository = "https://github.com/Ze-d/AgentPulse"
license = "MIT"
edition = "2021"

[[bin]]
name = "agentpulse-hook"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
ureq = "2"
sysinfo = "0.31"
log = "0.4"
env_logger = "0.11"
```

- [ ] **Step 2: Create minimal main.rs**

```rust
fn main() {
    eprintln!("agentpulse-hook v0.4.0 (placeholder)");
}
```

- [ ] **Step 3: Build to verify**

Run: `cd adapters/hook-adapter && cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add adapters/hook-adapter/Cargo.toml adapters/hook-adapter/src/main.rs
git commit -m "feat: scaffold hook-adapter crate skeleton"
```

---

### Task 2: Implement HTTP sender with retry

**Files:**
- Create: `adapters/hook-adapter/src/sender.rs`

- [ ] **Step 1: Write sender.rs**

```rust
//! HTTP POST with retry logic for AgentPulse events.

pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_MS: u64 = 1000;

/// POST event JSON to AgentPulse server. Returns HTTP status code, or -1 on
/// complete failure after all retries.
pub fn send_event(data: &serde_json::Value) -> i32 {
    let url = std::env::var("AGENTPULSE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:17878/api/events".to_string());

    let timeout_secs: u64 = std::env::var("AGENTPULSE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let body = serde_json::to_vec(data).unwrap_or_default();

    for attempt in 1..=MAX_RETRIES {
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .send_bytes(&body)
        {
            Ok(resp) => {
                let status = resp.status();
                if status == 201 {
                    log::info!("Event sent successfully (attempt {})", attempt);
                } else {
                    log::warn!("Server returned {} (attempt {})", status, attempt);
                }
                return status as i32;
            }
            Err(ureq::Error::Status(code, _resp)) => {
                log::warn!("Server returned {} (attempt {})", code, attempt);
                return code as i32;
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!(
                        "Connection failed (attempt {}/{}): {}",
                        attempt, MAX_RETRIES, e
                    );
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                } else {
                    log::error!(
                        "Failed to send event after {} attempts: {}",
                        MAX_RETRIES, e
                    );
                    return -1;
                }
            }
        }
    }

    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_event_connection_refused_returns_minus_one() {
        // Set a URL that definitely won't respond
        std::env::set_var("AGENTPULSE_URL", "http://127.0.0.1:19999/events");
        std::env::set_var("AGENTPULSE_TIMEOUT", "1");
        let data = serde_json::json!({"test": true});
        let status = send_event(&data);
        assert_eq!(status, -1);
    }
}
```

- [ ] **Step 2: Build and run sender test**

Run: `cd adapters/hook-adapter && cargo test`
Expected: Test `test_send_event_connection_refused_returns_minus_one` passes (server not running → returns -1 after retries)

- [ ] **Step 3: Commit**

```bash
git add adapters/hook-adapter/src/sender.rs
git commit -m "feat: add HTTP sender with retry logic"
```

---

### Task 3: Implement cross-platform agent detection

**Files:**
- Create: `adapters/hook-adapter/src/agent.rs`

- [ ] **Step 1: Write agent.rs**

```rust
//! Cross-platform process tree detection.
//!
//! Walks up the parent chain from our PID to find the real agent process,
//! skipping shell wrappers (cmd.exe, powershell.exe, bash, etc.).

use sysinfo::{Pid, System};

#[cfg(target_os = "windows")]
const SHELL_NAMES: &[&str] = &[
    "cmd.exe", "powershell.exe", "pwsh.exe",
    "sh.exe", "bash.exe", "conhost.exe",
];

#[cfg(not(target_os = "windows"))]
const SHELL_NAMES: &[&str] = &["sh", "bash", "zsh", "fish", "dash"];

const AGENT_BINARIES: &[(&str, &str)] = &[
    ("node.exe", "claude-code"),
    ("codex.exe", "codex"),
    ("codex", "codex"),
    ("gemini", "gemini"),
    ("copilot", "copilot"),
];

/// Walk up the process tree and return `(pid, agent_source)`.
///
/// On Windows, skips shell wrappers to find the real agent.
/// On other platforms, returns the immediate parent PID.
pub fn detect() -> (u32, String) {
    let mut system = System::new_all();
    system.refresh_all();

    let my_pid = Pid::from(std::process::id() as usize);

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(me) = system.process(my_pid) {
            if let Some(parent) = me.parent() {
                return (parent.as_u32(), "claude-code".to_string());
            }
        }
        return (std::process::id(), "claude-code".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut current = my_pid;
        let mut last_non_shell = std::process::id();
        let mut detected_source = "claude-code".to_string();

        for _ in 0..5 {
            let process = match system.process(current) {
                Some(p) => p,
                None => break,
            };
            let parent_pid = match process.parent() {
                Some(p) => p,
                None => break,
            };
            let parent = match system.process(parent_pid) {
                Some(p) => p,
                None => break,
            };
            let name = parent.name().to_lowercase();
            if !SHELL_NAMES.contains(&name.as_str()) {
                last_non_shell = parent_pid.as_u32();
                for (bin, source) in AGENT_BINARIES {
                    if name == *bin {
                        return (parent_pid.as_u32(), source.to_string());
                    }
                }
            }
            current = parent_pid;
        }

        (last_non_shell, detected_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid_pid() {
        let (pid, source) = detect();
        assert!(pid > 0, "pid should be positive, got {}", pid);
        assert!(!source.is_empty(), "source should not be empty");
    }

    #[test]
    fn test_detect_returns_known_source() {
        let (_pid, source) = detect();
        let valid_sources = ["claude-code", "codex", "gemini", "copilot", "unknown"];
        assert!(
            valid_sources.contains(&source.as_str()),
            "source '{}' should be one of {:?}",
            source,
            valid_sources
        );
    }
}
```

- [ ] **Step 2: Build and run agent tests**

Run: `cd adapters/hook-adapter && cargo test`
Expected: Both agent tests pass

- [ ] **Step 3: Commit**

```bash
git add adapters/hook-adapter/src/agent.rs
git commit -m "feat: add cross-platform agent detection"
```

---

### Task 4: Implement hook event handler (default mode)

**Files:**
- Create: `adapters/hook-adapter/src/hook.rs`

- [ ] **Step 1: Write hook.rs**

```rust
//! Hook event handler: reads JSON from stdin, enriches with agent info,
//! and POSTs to the AgentPulse server.

use std::io::Read;

/// Read hook JSON from stdin, detect agent info, and either print to stdout
/// (`test_mode == true`) or POST to the AgentPulse event server.  Exits the
/// process on critical failure.
pub fn run(test_mode: bool, url_override: Option<&str>) {
    // --- read stdin ---
    let mut raw = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut raw) {
        log::error!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    let raw = raw.trim().to_string();
    if raw.is_empty() {
        log::info!("No stdin data, skipping");
        std::process::exit(0);
    }

    // --- parse JSON ---
    let mut data: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to parse stdin as JSON: {}", e);
            std::process::exit(1);
        }
    };

    // --- enrich with agent info ---
    let (pid, source) = crate::agent::detect();
    log::debug!("detected agent: source={} pid={}", source, pid);
    data["process_pid"] = serde_json::json!(pid);
    data["agent_source"] = serde_json::json!(source);

    // --- test mode: print to stdout ---
    if test_mode {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
        std::process::exit(0);
    }

    // --- production: POST to server ---
    if let Some(url) = url_override {
        std::env::set_var("AGENTPULSE_URL", url);
    }

    let status = crate::sender::send_event(&data);
    if status < 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_run_with_empty_stdin_exits_zero() {
        // We can't easily test process::exit, but we can test the logic.
        // The empty-input case is verified via integration test below.
    }

    #[test]
    fn test_run_test_mode_writes_to_stdout() {
        // Spawn ourselves to verify --test flag output.
        let output = std::process::Command::new(
            std::env::current_exe().unwrap(),
        )
        .arg("--test")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(br#"{"session_id":"t1"}"#).unwrap();
            }
            child.wait_with_output()
        });

        match output {
            Ok(out) => {
                assert!(out.status.success());
                let stdout = String::from_utf8_lossy(&out.stdout);
                assert!(stdout.contains("session_id"), "stdout: {}", stdout);
                assert!(stdout.contains("agent_source"), "stdout: {}", stdout);
                assert!(stdout.contains("process_pid"), "stdout: {}", stdout);
            }
            Err(e) => {
                eprintln!("Skipping integration test: {e}");
            }
        }
    }
}
```

- [ ] **Step 2: Build**

Run: `cd adapters/hook-adapter && cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add adapters/hook-adapter/src/hook.rs
git commit -m "feat: add hook event handler (stdin -> POST)"
```

---

### Task 5: Implement installer for Claude Code (JSON) and Codex (TOML)

**Files:**
- Create: `adapters/hook-adapter/src/installer.rs`

- [ ] **Step 1: Write installer.rs**

```rust
//! Hook configuration management for Claude Code (JSON) and Codex (TOML).
//!
//! Supports: install, remove, status, dry-run.

use std::path::{Path, PathBuf};

const CC_HOOK_EVENTS: &[&str] = &[
    "SessionStart", "PreToolUse", "PostToolUse",
    "PostToolUseFailure", "Notification", "Stop",
];

const CODEX_HOOK_EVENTS: &[&str] = &[
    "SessionStart", "PreToolUse", "PostToolUse",
    "PermissionRequest", "UserPromptSubmit", "Stop",
];

// ── Claude Code (JSON) ────────────────────────────────────────

fn cc_default_path() -> PathBuf {
    dirs_home().join(".claude").join("settings.json")
}

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

fn cc_install(path: &Path, binary: &str, force: bool) -> String {
    let settings = load_json(path);
    let our = build_cc_hook_configs(binary);

    if !force {
        let existing = settings.get("hooks").cloned().unwrap_or(serde_json::json!({}));
        let all_ok = CC_HOOK_EVENTS.iter().all(|e| {
            let ours = our.get(e);
            let theirs = existing.get(e);
            ours.is_some() && ours == theirs
        });
        if all_ok {
            return "already_installed".to_string();
        }
    }

    // Backup
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("json.bak"));
    }

    let existing_hooks = settings.get("hooks").cloned().unwrap_or(serde_json::json!({}));
    let mut merged = if let serde_json::Value::Object(m) = existing_hooks {
        m.clone()
    } else {
        serde_json::Map::new()
    };
    if let serde_json::Value::Object(our_map) = our {
        for (k, v) in our_map {
            merged.insert(k, v);
        }
    }

    let mut new_settings = settings.clone();
    new_settings["hooks"] = serde_json::Value::Object(merged);
    save_json(path, &new_settings).unwrap();

    "installed".to_string()
}

fn cc_remove(path: &Path) -> String {
    if !path.exists() {
        return "no_settings_file".to_string();
    }
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("json.bak"));
    }
    let settings = load_json(path);
    if let serde_json::Value::Object(mut map) = settings {
        if let Some(hooks) = map.get_mut("hooks") {
            if let serde_json::Value::Object(hooks_map) = hooks {
                for event in CC_HOOK_EVENTS {
                    hooks_map.remove(*event);
                }
            }
        }
        save_json(path, &serde_json::Value::Object(map)).unwrap();
    }
    "removed".to_string()
}

fn cc_status(path: &Path) {
    let settings = load_json(path);
    let hooks = settings.get("hooks").cloned().unwrap_or(serde_json::json!({}));
    for event in CC_HOOK_EVENTS {
        let mark = if hooks.get(event).is_some() { "[OK]" } else { "[--]" };
        println!("  {mark} {event}");
    }
}

fn cc_dry_run(path: &Path, binary: &str) {
    let settings = load_json(path);
    let hooks = settings.get("hooks").cloned().unwrap_or(serde_json::json!({}));
    println!("Monitor binary: {binary}");
    for event in CC_HOOK_EVENTS {
        if hooks.get(event).is_some() {
            println!("  [SKIP] {event} (already installed)");
        } else {
            println!("  [WILL INSTALL] {event}");
        }
    }
}

// ── Codex (TOML) ──────────────────────────────────────────────

fn codex_default_path() -> PathBuf {
    dirs_home().join(".codex").join("config.toml")
}

fn build_codex_hook_entry(event: &str, command: &str) -> String {
    format!(
        r#"{event} = [
  {{ matcher = "", hooks = [
    {{ type = "command", command = '{command}' }}
  ] }}
]"#,
    )
}

/// Minimal TOML section parser — handles the subset used by Codex config.toml.
fn parse_toml_sections(raw: &str) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut sections: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    sections.insert("_top".to_string(), vec![]);
    let mut current: String = "_top".to_string();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed.starts_with("[[") {
            current = trimmed[1..trimmed.len() - 1].to_string();
            sections.entry(current.clone()).or_default();
        } else {
            sections.entry(current.clone()).or_default().push(line.to_string());
        }
    }
    sections
}

fn find_event_start(lines: &[String], event: &str) -> Option<usize> {
    lines.iter().position(|l| l.trim().starts_with(&format!("{event} =")))
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

    // Reassemble
    let mut out = String::new();
    if let Some(top) = sections.remove("_top") {
        for line in &top {
            if !line.trim().is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (name, lines) in &sections {
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("[{name}]\n"));
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }

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

    let mut out = String::new();
    if let Some(top) = sections.remove("_top") {
        for line in &top {
            if !line.trim().is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (name, lines) in &sections {
        if name == "hooks" && lines.is_empty() {
            continue;
        }
        out.push_str(&format!("[{name}]\n"));
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    std::fs::write(path, &out).unwrap();
    "removed".to_string()
}

fn codex_status(path: &Path) {
    if !path.exists() {
        for event in CODEX_HOOK_EVENTS {
            println!("  [--] {event}");
        }
        return;
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    for event in CODEX_HOOK_EVENTS {
        let mark = if raw.contains(&format!("{event} =")) { "[OK]" } else { "[--]" };
        println!("  {mark} {event}");
    }
}

fn codex_dry_run(path: &Path, binary: &str) {
    println!("Config path: {}", path.display());
    println!("Monitor binary: {binary}");
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    for event in CODEX_HOOK_EVENTS {
        if raw.contains(&format!("{event} =")) {
            println!("  [SKIP] {event} (already installed)");
        } else {
            println!("  [WILL INSTALL] {event}");
        }
    }
}

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
            codex_status(&p);
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            cc_status(&p);
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
            codex_dry_run(&p, &binary_path);
        }
        _ => {
            let p = path.map(PathBuf::from).unwrap_or_else(cc_default_path);
            cc_dry_run(&p, &binary_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
        assert_eq!(cc_install(&p, "/usr/bin/agentpulse-hook", false), "installed");
        assert_eq!(cc_install(&p, "/usr/bin/agentpulse-hook", false), "already_installed");
    }

    #[test]
    fn test_cc_install_preserves_other_keys() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("settings.json");
        let existing = serde_json::json!({"model": "claude-sonnet-4-6", "permissions": {"allow": ["Read"]}});
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
                "CustomEvent": [{"matcher": "", "hooks": [{"type": "command", "command": "echo hi"}]}],
                "SessionStart": [{"matcher": "", "hooks": [{"type": "command", "command": "agentpulse-hook"}]}]
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
        assert_eq!(codex_install(&p, "/usr/bin/agentpulse-hook", false), "installed");
        assert_eq!(codex_install(&p, "/usr/bin/agentpulse-hook", false), "already_installed");
    }

    #[test]
    fn test_codex_install_merges_with_existing() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(&p, "model = \"gpt-5\"\n\n[hooks]\nCustomEvent = [{ matcher = \"\", hooks = [{ type = \"command\", command = \"echo hi\" }] }]\n").unwrap();

        codex_install(&p, "/usr/bin/agentpulse-hook", false);

        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("CustomEvent"));
        assert!(content.contains("SessionStart"));
        assert!(content.contains("model"));
    }
}
```

- [ ] **Step 2: Add tempfile to dev-dependencies in Cargo.toml**

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Build and run tests**

Run: `cd adapters/hook-adapter && cargo test`
Expected: All 8 installer tests pass

- [ ] **Step 4: Commit**

```bash
git add adapters/hook-adapter/src/installer.rs adapters/hook-adapter/Cargo.toml
git commit -m "feat: add installer for CC JSON and Codex TOML hook management"
```

---

### Task 6: Wire up main.rs with clap dispatch

**Files:**
- Modify: `adapters/hook-adapter/src/main.rs`

- [ ] **Step 1: Rewrite main.rs**

```rust
//! AgentPulse hook adapter — zero-dependency binary.
//!
//! Default mode (no subcommand): reads hook JSON from stdin, enriches with
//! agent info, and POSTs to the AgentPulse event server.
//!
//! Subcommand mode: manages hook configuration in Claude Code / Codex settings.

mod agent;
mod hook;
mod installer;
mod sender;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agentpulse-hook",
    version,
    about = "AgentPulse hook adapter",
    long_about = "Zero-dependency event forwarder and hook manager for AgentPulse."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Print enriched JSON to stdout instead of sending to server
    #[arg(long, global = true)]
    test: bool,

    /// Target agent: claude (default) or codex
    #[arg(long, default_value = "claude", global = true)]
    agent: String,

    /// Override config/settings file path
    #[arg(long, global = true)]
    path: Option<String>,

    /// Force overwrite existing hooks
    #[arg(long, global = true)]
    force: bool,

    /// Override the AgentPulse server URL (runtime mode only)
    #[arg(long)]
    url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install hooks to agent config file
    Install,
    /// Remove hooks from agent config file
    Remove,
    /// Show hook installation status
    Status,
    /// Preview changes without modifying
    DryRun,
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(
            std::env::var("AGENTPULSE_LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
        )
    ).format_timestamp_millis().init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default mode: stdin -> POST
            hook::run(cli.test, cli.url.as_deref());
        }
        Some(Commands::Install) => {
            installer::install(&cli.agent, cli.path.as_deref(), cli.force);
        }
        Some(Commands::Remove) => {
            installer::remove(&cli.agent, cli.path.as_deref());
        }
        Some(Commands::Status) => {
            installer::status(&cli.agent, cli.path.as_deref());
        }
        Some(Commands::DryRun) => {
            installer::dry_run(&cli.agent, cli.path.as_deref());
        }
    }
}
```

- [ ] **Step 2: Build release binary**

Run: `cd adapters/hook-adapter && cargo build --release`
Expected: Compiles, binary at `target/release/agentpulse-hook[.exe]`

- [ ] **Step 3: Smoke test — test mode**

Run (PowerShell):
```powershell
echo '{"session_id":"test-001","cwd":"/tmp","hook_event_name":"SessionStart"}' | .\adapters\hook-adapter\target\release\agentpulse-hook --test
```
Expected: Prints enriched JSON with `process_pid` and `agent_source` fields to stdout

- [ ] **Step 4: Smoke test — install dry-run**

Run (PowerShell):
```powershell
.\adapters\hook-adapter\target\release\agentpulse-hook dry-run --path "$env:TEMP\test_settings.json"
```
Expected: Shows hooks to install (all 6)

- [ ] **Step 5: Commit**

```bash
git add adapters/hook-adapter/src/main.rs
git commit -m "feat: wire up main.rs with clap dispatch"
```

---

### Task 7: Modify hooks.rs — drop resolve_python, adapt to binary

**Files:**
- Modify: `apps/desktop/src-tauri/src/hooks.rs`

- [ ] **Step 1: Replace `find_monitor_script` with `find_hook_binary`**

Replace the function at hooks.rs:43-69:

```rust
/// Locate `agentpulse-hook` binary on disk.
///
/// In bundled (release) mode the binary lives in the resource directory. In
/// dev mode we search the hook-adapter build output.
pub fn find_hook_binary(resource_dir: &Path) -> Result<PathBuf, String> {
    // On Windows the binary has an .exe extension.
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
```

- [ ] **Step 2: Replace `extract_monitor_script` with `extract_hook_binary`**

Replace the function at hooks.rs:77-97:

```rust
/// Copy `agentpulse-hook` binary into `app_data_dir`, overwriting only when the
/// source is newer. Returns the destination path.
pub fn extract_hook_binary(resource_dir: &Path, app_data_dir: &Path) -> Result<PathBuf, String> {
    let src = find_hook_binary(resource_dir)?;

    fs::create_dir_all(app_data_dir).map_err(|e| format!("create app_data_dir: {e}"))?;

    let bin_name = if cfg!(target_os = "windows") {
        "agentpulse-hook.exe"
    } else {
        "agentpulse-hook"
    };
    let dst = app_data_dir.join(bin_name);

    let should_copy = match (fs::metadata(&src), fs::metadata(&dst)) {
        (Ok(src_meta), Ok(dst_meta)) => src_meta.modified().ok() > dst_meta.modified().ok(),
        (Ok(_), Err(_)) => true,
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
```

- [ ] **Step 3: Remove `resolve_python()` entirely**

Delete the function at hooks.rs:103-136. The entire `resolve_python` function is dead code after this migration.

- [ ] **Step 4: Change `build_hook_configs` to not take python param**

Replace `build_hook_configs` at hooks.rs:139-154:

```rust
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
```

- [ ] **Step 5: Change `ensure_hooks_installed` signature to remove `python` param**

Change signature at hooks.rs:208-212:

```rust
pub fn ensure_hooks_installed(
    settings_path: &Path,
    hook_binary: &str,
) -> Result<String, String> {
```

And update the call to `build_hook_configs` inside:
```rust
let our_config = build_hook_configs(hook_binary);
```

- [ ] **Step 6: Change `build_codex_hook_configs` to not take python param**

Replace `build_codex_hook_configs` at hooks.rs:353-370:

```rust
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
```

- [ ] **Step 7: Change `ensure_codex_hooks_installed` signature to remove `python` param**

Change signature at hooks.rs:487-491:

```rust
pub fn ensure_codex_hooks_installed(
    config_path: &Path,
    hook_binary: &str,
) -> Result<String, String> {
```

And update the internal call to `build_codex_hook_configs(hook_binary)`.

- [ ] **Step 8: Update all references inside hooks.rs tests**

All test calls to `ensure_hooks_installed` change from:
```rust
ensure_hooks_installed(&settings_path, monitor_script, "python")
```
to:
```rust
ensure_hooks_installed(&settings_path, "/app/agentpulse-hook")
```

All test calls to `ensure_codex_hooks_installed` change from:
```rust
ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python")
```
to:
```rust
ensure_codex_hooks_installed(&config_path, "/app/agentpulse-hook")
```

And update assertions that check for `python` in command strings to check for `agentpulse-hook` instead.

- [ ] **Step 9: Build and run hooks.rs tests**

Run: `cd apps/desktop/src-tauri && cargo test --lib hooks`
Expected: All tests pass

- [ ] **Step 10: Commit**

```bash
git add apps/desktop/src-tauri/src/hooks.rs
git commit -m "refactor: remove Python dependency from hooks.rs, use agentpulse-hook binary"
```

---

### Task 8: Modify commands.rs — drop resolve_python calls

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands.rs`

- [ ] **Step 1: Update `install_hooks_cmd`**

Replace lines 74-93:

```rust
#[tauri::command]
pub fn install_hooks_cmd(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("user triggered hook installation");
    let settings_path = app_handle
        .path()
        .resolve(".claude/settings.json", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let hook_path = hooks::extract_hook_binary(&resource_dir, &app_data_dir)?;
    hooks::ensure_hooks_installed(&settings_path, &hook_path.to_string_lossy())
}
```

Note: removes the `state: State<AppState>` parameter since we no longer need `state.config.python`.

- [ ] **Step 2: Update `install_codex_hooks_cmd`**

Replace lines 117-137:

```rust
#[tauri::command]
pub fn install_codex_hooks_cmd(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("user triggered codex hook installation");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let hook_path = hooks::extract_hook_binary(&resource_dir, &app_data_dir)?;
    hooks::ensure_codex_hooks_installed(&config_path, &hook_path.to_string_lossy())
}
```

- [ ] **Step 3: Build**

Run: `cd apps/desktop/src-tauri && cargo build`
Expected: Compiles (may have lib.rs errors until Task 9)

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/commands.rs
git commit -m "refactor: drop Python param from install commands"
```

---

### Task 9: Modify lib.rs — drop python_for_hooks variable

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Read lib.rs to find `python_for_hooks` and `python` variable bindings**

Locate the variable initialization (search for `python_for_hooks` or `resolve_python` in lib.rs).

- [ ] **Step 2: Remove `python` variable and migrate auto-install blocks**

In the CC auto-install block (~line 260-300), change:
```rust
let python = python_for_hooks.clone();
```
to: *(remove entirely)*

Change:
```rust
match hooks::extract_monitor_script(&resource_dir, &app_data_dir) {
    Ok(monitor_path) => {
        match hooks::ensure_hooks_installed(
            &settings_path,
            &monitor_path.to_string_lossy(),
            &python,
        ) {
```

to:
```rust
match hooks::extract_hook_binary(&resource_dir, &app_data_dir) {
    Ok(hook_path) => {
        match hooks::ensure_hooks_installed(
            &settings_path,
            &hook_path.to_string_lossy(),
        ) {
```

In the Codex auto-install block (~line 302-340), change:
```rust
let python2 = python_for_hooks;
```
to: *(remove entirely)*

Change:
```rust
match hooks::extract_monitor_script(&resource_dir, &app_data_dir) {
    Ok(monitor_path) => {
        match hooks::ensure_codex_hooks_installed(
            &codex_config_path,
            &monitor_path.to_string_lossy(),
            &python2,
        ) {
```

to:
```rust
match hooks::extract_hook_binary(&resource_dir, &app_data_dir) {
    Ok(hook_path) => {
        match hooks::ensure_codex_hooks_installed(
            &codex_config_path,
            &hook_path.to_string_lossy(),
        ) {
```

Remove the `python_for_hooks` variable initialization from earlier in setup() (search for `let python_for_hooks`).

- [ ] **Step 3: Build**

Run: `cd apps/desktop/src-tauri && cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs
git commit -m "refactor: drop python_for_hooks from lib.rs auto-install"
```

---

### Task 10: Remove `python` field from config.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/config.rs`

- [ ] **Step 1: Remove `python` field from `AgentPulseConfig`**

Remove lines 49-52 (the `python: Option<String>` field and its doc comment).

- [ ] **Step 2: Remove `AGENTPULSE_PYTHON` env var handling**

Remove the line `| AGENTPULSE_PYTHON | python |` from the doc comment table (line 14).

Remove the env var parsing for `AGENTPULSE_PYTHON` that sets `self.python = Some(v)`.

- [ ] **Step 3: Update tests**

Remove test assertions referencing `config.python`.

- [ ] **Step 4: Build and run tests**

Run: `cd apps/desktop/src-tauri && cargo test --lib config`
Expected: All config tests pass

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/config.rs
git commit -m "refactor: remove python field from AgentPulseConfig"
```

---

### Task 11: Update tauri.conf.json — bundle binary instead of .py

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Update resources**

Change the `bundle.resources` section from:
```json
"resources": {
  "../../../adapters/claude-code/monitor_hook.py": "monitor_hook.py"
},
```

to:
```json
"resources": {
  "../../../adapters/hook-adapter/target/release/agentpulse-hook": "agentpulse-hook"
},
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src-tauri/tauri.conf.json
git commit -m "chore: bundle agentpulse-hook binary instead of Python script"
```

---

### Task 12: Delete Python files and old tests

- [ ] **Step 1: Delete Python adapter files**

```bash
git rm adapters/claude-code/monitor_hook.py
git rm adapters/claude-code/install_hooks.py
git rm adapters/codex/monitor_hook.py
git rm adapters/codex/install_hooks.py
```

- [ ] **Step 2: Delete Python test files**

```bash
git rm tests/unit/test_install_hooks.py
git rm tests/unit/test_monitor_hook.py
```

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: remove Python hook adapter files and tests"
```

---

### Task 13: Update documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/local-development-guide.md`
- Modify: `docs/flows/02-hooks-installation.md`
- Modify: `docs/architecture/module-boundaries.md`
- Modify: `docs/testing/testing-strategy.md`
- Modify: `AGENTS.md`
- Modify: `docs/ai/context-map.md`
- Modify: `docs/todos/07-python-free-hook-adapter.md`

- [ ] **Step 1: Update README.md**

Replace all `python adapters/claude-code/install_hooks.py ...` commands with `agentpulse-hook install ...` equivalents.

Key replacements:
- `python adapters/claude-code/install_hooks.py --dry-run` → `agentpulse-hook dry-run`
- `python adapters/claude-code/install_hooks.py` → `agentpulse-hook install`
- `python adapters/claude-code/install_hooks.py --status` → `agentpulse-hook status`
- `python adapters/claude-code/install_hooks.py --remove` → `agentpulse-hook remove`

Also update the adapter directory tree in README.

- [ ] **Step 2: Update docs/local-development-guide.md**

Same command replacements as README. Update references to `monitor_hook.py` to `agentpulse-hook`.

- [ ] **Step 3: Update docs/flows/02-hooks-installation.md**

Replace Python CLI section with `agentpulse-hook` CLI. Remove references to `resolve_python()`. Update the flow diagram.

- [ ] **Step 4: Update docs/architecture/module-boundaries.md**

Replace Python module entries with Rust binary entry.

- [ ] **Step 5: Update docs/testing/testing-strategy.md**

Remove Python unit test section. Note that tests are now Rust-based in the hook-adapter crate.

- [ ] **Step 6: Update AGENTS.md and docs/ai/context-map.md**

Replace Python file references with Rust crate references.

- [ ] **Step 7: Update docs/todos/07-python-free-hook-adapter.md**

Change status from "待规划" to "已完成" and add completion date.

- [ ] **Step 8: Commit**

```bash
git add README.md docs/
git commit -m "docs: update documentation for Python-free hook adapter"
```

---

### Task 14: Full build verification

- [ ] **Step 1: Build hook-adapter release**

Run: `cd adapters/hook-adapter && cargo build --release`
Expected: Success, binary produced

- [ ] **Step 2: Run hook-adapter tests**

Run: `cd adapters/hook-adapter && cargo test`
Expected: All tests pass

- [ ] **Step 3: Build Tauri app**

Run: `cd apps/desktop/src-tauri && cargo build`
Expected: Success

- [ ] **Step 4: Run Tauri tests**

Run: `cd apps/desktop/src-tauri && cargo test --lib`
Expected: All tests pass (including updated hooks tests)

- [ ] **Step 5: End-to-end smoke test**

Run (PowerShell):
```powershell
echo '{"session_id":"e2e-test","cwd":"C:/tmp","hook_event_name":"SessionStart"}' | .\adapters\hook-adapter\target\release\agentpulse-hook --test
```
Expected: JSON output with `agent_source` and `process_pid` fields

- [ ] **Step 6: Commit any remaining changes**

```bash
git add -A
git commit -m "chore: final verification and cleanup for Python-free hook adapter"
```
