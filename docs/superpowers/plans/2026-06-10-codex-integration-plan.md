# Codex Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OpenAI Codex CLI support so AgentPulse monitors both Claude Code and Codex sessions in the same floating panel.

**Architecture:** Reuse existing HTTP event server, state machine, and database. Add a Codex-specific event normalizer alongside the existing Claude Code one. Route events based on an injected `agent_source` JSON field. Add Python adapter scripts for Codex TOML-based hook configuration.

**Tech Stack:** Rust (existing), Python 3 stdlib, `toml` crate (new Rust dependency)

---

## File Map

| File | Responsibility |
|------|---------------|
| `adapters/codex/install_hooks.py` | **New.** Install/remove Codex hooks in `~/.codex/config.toml` |
| `adapters/codex/monitor_hook.py` | **New.** Read Codex hook JSON from stdin, inject `agent_source` + `process_pid`, POST to server |
| `apps/desktop/src-tauri/Cargo.toml` | Add `toml` crate dependency |
| `apps/desktop/src-tauri/src/event_server.rs` | Add `normalize_codex_event()`, update `handle_event` routing |
| `apps/desktop/src-tauri/src/hooks.rs` | Add Codex TOML config management functions |
| `apps/desktop/src-tauri/src/lib.rs` | Add Codex hook auto-install on startup |
| `apps/desktop/src-tauri/src/commands.rs` | Add Codex hook status/install/uninstall Tauri commands |
| `apps/desktop/src-tauri/tests/event_server_test.rs` | Add Codex normalization tests |

---

### Task 1: Add `normalize_codex_event()` to event_server.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/event_server.rs`
- Modify: `apps/desktop/src-tauri/tests/event_server_test.rs`

- [ ] **Step 1: Write the failing Codex normalization tests**

Add to `apps/desktop/src-tauri/tests/event_server_test.rs` after the existing tests:

```rust
// ── Codex normalization tests ──

#[test]
fn test_normalize_codex_session_start() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/home/user/codex-project",
        "hook_event_name": "SessionStart",
        "transcript_path": "/tmp/codex-transcript.json",
        "model": "gpt-5",
        "permission_mode": "default",
        "source": "startup",
        "turn_id": "turn-1"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::SessionStart);
    assert_eq!(event.status, AgentStatus::Starting);
    assert_eq!(event.project_name, Some("codex-project".into()));
    // Codex-specific fields are ignored — not present in output
}

#[test]
fn test_normalize_codex_pre_tool_use() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "cargo build"},
        "tool_use_id": "tu-1",
        "turn_id": "turn-1",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::PreToolUse);
    assert_eq!(event.status, AgentStatus::ToolRunning);
    assert_eq!(event.tool_name, Some("Bash".into()));
}

#[test]
fn test_normalize_codex_post_tool_use() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "PostToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "cargo build"},
        "tool_response": "Compiling...",
        "tool_use_id": "tu-1",
        "turn_id": "turn-1",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::PostToolUse);
    assert_eq!(event.status, AgentStatus::Running);
}

#[test]
fn test_normalize_codex_permission_request() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "PermissionRequest",
        "tool_name": "Bash",
        "tool_input": {"command": "rm -rf /"},
        "turn_id": "turn-1",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::PermissionRequest);
    assert_eq!(event.status, AgentStatus::WaitingPermission);
}

#[test]
fn test_normalize_codex_user_prompt_submit() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "UserPromptSubmit",
        "prompt": "refactor this module",
        "turn_id": "turn-2",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::Notification);
    assert_eq!(event.status, AgentStatus::Running);
}

#[test]
fn test_normalize_codex_stop() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "Stop",
        "last_assistant_message": "Done refactoring.",
        "stop_hook_active": false,
        "turn_id": "turn-2",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default"
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.event_type, EventType::Stop);
    assert_eq!(event.status, AgentStatus::Completed);
    assert_eq!(event.message, Some("Done refactoring.".into()));
}

#[test]
fn test_normalize_codex_process_pid_passthrough() {
    let raw = serde_json::json!({
        "session_id": "cx-sess-001",
        "cwd": "/tmp",
        "hook_event_name": "SessionStart",
        "transcript_path": null,
        "model": "gpt-5",
        "permission_mode": "default",
        "source": "startup",
        "process_pid": 4242
    });

    let event = normalize_codex_event(&raw);
    assert_eq!(event.process_pid, Some(4242));
}
```

- [ ] **Step 2: Run test to verify it fails**

```powershell
cd apps/desktop/src-tauri; cargo test normalize_codex -- --nocapture
```
Expected: COMPILE ERROR — `normalize_codex_event` not found.

- [ ] **Step 3: Write `normalize_codex_event()` in event_server.rs**

Add before the `EventServer` impl block in `apps/desktop/src-tauri/src/event_server.rs` (after the existing `normalize_claude_code_event` function):

```rust
/// Normalize a raw Codex CLI hook JSON event into an `AgentEvent`.
///
/// Codex emits the same `hook_event_name` values (PascalCase) and the same
/// `session_id`, `cwd`, `transcript_path` structure as Claude Code.  Extra
/// Codex-only fields (`model`, `permission_mode`, `turn_id`) are silently
/// ignored.  Unlike Claude Code, `PermissionRequest` is its own top-level
/// hook event rather than a `Notification` sub-type.
pub fn normalize_codex_event(raw: &serde_json::Value) -> AgentEvent {
    let hook_event_name = raw["hook_event_name"].as_str().unwrap_or("");
    let session_id = raw["session_id"].as_str().unwrap_or("unknown");
    let cwd = raw["cwd"].as_str().unwrap_or("");
    let transcript_path = raw["transcript_path"].as_str().map(|s| s.to_string());

    // Prefer `message` field; fall back to `last_assistant_message`.
    let message = raw["message"]
        .as_str()
        .or_else(|| raw["last_assistant_message"].as_str())
        .map(|s| s.to_string());

    let tool_name = raw["tool_name"].as_str().map(|s| s.to_string());
    let process_pid = raw["process_pid"].as_u64().map(|v| v as u32);

    // Derive project name from the last path component of cwd.
    let project_name = if cwd.is_empty() {
        None
    } else {
        std::path::Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    };

    let (event_type, status) = match hook_event_name {
        "SessionStart" => (EventType::SessionStart, AgentStatus::Starting),
        "PreToolUse" => (EventType::PreToolUse, AgentStatus::ToolRunning),
        "PostToolUse" => (EventType::PostToolUse, AgentStatus::Running),
        "PermissionRequest" => (EventType::PermissionRequest, AgentStatus::WaitingPermission),
        "Stop" | "SubagentStop" => (EventType::Stop, AgentStatus::Completed),
        "UserPromptSubmit" => (EventType::Notification, AgentStatus::Running),
        _ => (EventType::Notification, AgentStatus::Running),
    };

    AgentEvent {
        id: Uuid::new_v4().to_string(),
        source: AgentSource::Codex,
        session_id: session_id.to_string(),
        cwd: cwd.to_string(),
        project_name,
        event_type,
        status,
        message,
        tool_name,
        transcript_path,
        created_at: Utc::now().timestamp_millis(),
        process_pid,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```powershell
cd apps/desktop/src-tauri; cargo test normalize_codex -- --nocapture
```
Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/event_server.rs apps/desktop/src-tauri/tests/event_server_test.rs
git commit -m "feat: add normalize_codex_event() for Codex CLI hook normalization"
```

---

### Task 2: Route events by `agent_source` in event_server.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/event_server.rs`
- Modify: `apps/desktop/src-tauri/tests/event_server_test.rs`

- [ ] **Step 1: Write tests for agent_source-based routing**

Add before the Codex tests in `event_server_test.rs`:

```rust
#[test]
fn test_normalize_dispatches_codex_by_agent_source_field() {
    let raw = serde_json::json!({
        "agent_source": "codex",
        "session_id": "cx-dispatch",
        "cwd": "/home/user/proj",
        "hook_event_name": "Stop"
    });

    let event = normalize_event_by_source(&raw);
    assert_eq!(event.source, AgentSource::Codex);
    assert_eq!(event.status, AgentStatus::Completed);
}

#[test]
fn test_normalize_dispatches_claude_when_no_agent_source_field() {
    // Backward compatible: events without agent_source default to ClaudeCode
    let raw = serde_json::json!({
        "session_id": "cc-dispatch",
        "cwd": "/home/user/proj",
        "hook_event_name": "Stop"
    });

    let event = normalize_event_by_source(&raw);
    assert_eq!(event.source, AgentSource::ClaudeCode);
    assert_eq!(event.status, AgentStatus::Completed);
}

#[test]
fn test_normalize_dispatches_claude_when_unknown_agent_source() {
    let raw = serde_json::json!({
        "agent_source": "some-future-agent",
        "session_id": "future-dispatch",
        "cwd": "/tmp",
        "hook_event_name": "Stop"
    });

    let event = normalize_event_by_source(&raw);
    // Unknown sources fall back to ClaudeCode
    assert_eq!(event.source, AgentSource::ClaudeCode);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```powershell
cd apps/desktop/src-tauri; cargo test normalize_event_by_source -- --nocapture
```
Expected: COMPILE ERROR — `normalize_event_by_source` not found.

- [ ] **Step 3: Add `normalize_event_by_source()` and update `handle_event()`**

In `event_server.rs`, add after `normalize_codex_event` and before the `EventServer` impl:

```rust
/// Dispatch to the appropriate normalizer based on the `agent_source` field.
///
/// - `"codex"` → `normalize_codex_event`
/// - missing / `"claude-code"` / anything else → `normalize_claude_code_event` (backward compatible)
pub fn normalize_event_by_source(raw: &serde_json::Value) -> AgentEvent {
    match raw["agent_source"].as_str() {
        Some("codex") => normalize_codex_event(raw),
        _ => normalize_claude_code_event(raw),
    }
}
```

Update the `handle_event` method — replace `normalize_claude_code_event` with `normalize_event_by_source`:

The line:
```rust
let event = normalize_claude_code_event(raw);
```
becomes:
```rust
let event = normalize_event_by_source(raw);
```

- [ ] **Step 4: Run all event_server tests**

```powershell
cd apps/desktop/src-tauri; cargo test -- --nocapture
```
Expected: ALL tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/event_server.rs apps/desktop/src-tauri/tests/event_server_test.rs
git commit -m "feat: route events by agent_source field in event server"
```

---

### Task 3: Add TOML config management to hooks.rs

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/hooks.rs`

- [ ] **Step 1: Add `toml` dependency to Cargo.toml**

Add under `[dependencies]` in `apps/desktop/src-tauri/Cargo.toml`:

```toml
toml = "0.8"
```

- [ ] **Step 2: Verify Cargo.toml update compiles**

```powershell
cd apps/desktop/src-tauri; cargo check
```
Expected: Compiles successfully.

- [ ] **Step 3: Write Codex TOML tests**

Add at the end of the `#[cfg(test)] mod tests` block in `hooks.rs` (before the closing `}`):

```rust
#[test]
fn test_codex_hook_config_serializes_to_toml() {
    let config = build_codex_hook_configs("/usr/local/bin/monitor_hook.py", "python3");
    let toml_str = toml::to_string_pretty(&config).unwrap();
    // Must contain all 6 events
    assert!(toml_str.contains("SessionStart"));
    assert!(toml_str.contains("PreToolUse"));
    assert!(toml_str.contains("PostToolUse"));
    assert!(toml_str.contains("PermissionRequest"));
    assert!(toml_str.contains("UserPromptSubmit"));
    assert!(toml_str.contains("Stop"));
    // Must include the command
    assert!(toml_str.contains("python3"));
    assert!(toml_str.contains("monitor_hook.py"));
}

#[test]
fn test_codex_install_merges_with_existing_config() {
    // Create a temp dir to simulate ~/.codex/
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    // Pre-populate with some existing config
    let existing = r#"
model = "gpt-5"

[hooks]
SomeOtherEvent = [
    { matcher = "", hooks = [{ type = "command", command = "echo hi" }] }
]
"#;
    std::fs::write(&config_path, existing).unwrap();

    // Install Codex hooks
    let result = ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python");
    assert!(result.is_ok());

    // Verify TOML is still valid and contains both our hooks and the existing ones
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("SomeOtherEvent"));
    assert!(content.contains("SessionStart"));
    assert!(content.contains("model"));
}

#[test]
fn test_codex_install_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    // First install
    assert_eq!(
        ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python").unwrap(),
        "installed"
    );
    // Second install
    assert_eq!(
        ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python").unwrap(),
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
    ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python").unwrap();
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

    ensure_codex_hooks_installed(&config_path, "/app/monitor_hook.py", "python").unwrap();

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
```

- [ ] **Step 4: Run tests to verify they fail**

```powershell
cd apps/desktop/src-tauri; cargo test codex_ -- --nocapture
```
Expected: COMPILE ERROR — `build_codex_hook_configs` etc. not found.

- [ ] **Step 5: Add Codex TOML types and functions to hooks.rs**

Add after the `HOOK_EVENTS` constant at the top of `hooks.rs`:

```rust
/// The 6 Codex hook events that AgentPulse subscribes to.
const CODEX_HOOK_EVENTS: [&str; 6] = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "UserPromptSubmit",
    "Stop",
];

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
```

Add at the top of hooks.rs with the other `use` imports:

```rust
use serde::{Deserialize, Serialize};
```

(Note: `serde::Serialize` may already be in scope via the existing `use serde_json::{json, Value};` — verify it compiles.)

Now add the Codex functions after the existing Claude Code section (after `get_hook_status` and before the `#[cfg(test)]` block):

```rust
// ---------------------------------------------------------------------------
// Codex TOML configuration management
// ---------------------------------------------------------------------------

/// Build the Codex hook configs object with the 6 events pointing at `monitor_script`.
fn build_codex_hook_configs(monitor_script: &str, python: &str) -> CodexHooksToml {
    let command = format!("{python} \"{monitor_script}\"");
    let group = vec![CodexMatcherGroup {
        matcher: String::new(),
        hooks: vec![CodexHookHandler {
            handler_type: "command".to_string(),
            command: command.clone(),
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

/// Resolve the path to Codex's config.toml (typically `~/.codex/config.toml`).
fn resolve_codex_config_path(home: &Path) -> PathBuf {
    home.join(".codex").join("config.toml")
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

/// Save a CodexConfigToml to disk.  Preserves top-level keys that we don't
/// manage by merging into the existing file.
fn save_codex_config(path: &Path, hooks: &CodexHooksToml) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }

    // Read the existing file as a raw TOML value so we preserve non-hook keys.
    let existing_raw: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or(toml::Value::Table(toml::Table::new()));

    // Convert our hooks to a TOML value and insert into the existing table.
    let hooks_value = toml::to_string(hooks)
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .unwrap_or(toml::Value::Table(toml::Table::new()));

    let mut table = match existing_raw {
        toml::Value::Table(t) => t,
        other => {
            let mut t = toml::Table::new();
            // Best-effort: if the existing file is non-table (unlikely), keep it under a key
            t.insert("__previous_root".to_string(), other);
            t
        }
    };
    table.insert("hooks".to_string(), hooks_value);

    let new_raw = toml::Value::Table(table);
    let toml_str = toml::to_string_pretty(&new_raw).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, &toml_str).map_err(|e| format!("write {}: {e}", path.display()))?;
    tracing::debug!(path = %path.display(), len = toml_str.len(), "codex config.toml saved");
    Ok(())
}

/// Ensure all 6 AgentPulse Codex hooks are present and point to `monitor_script`.
///
/// Returns `"already_ok"`, `"installed"`, or `"updated"`.
pub fn ensure_codex_hooks_installed(
    config_path: &Path,
    monitor_script: &str,
    python: &str,
) -> Result<String, String> {
    let config = load_codex_config(config_path);
    let our_hooks = build_codex_hook_configs(monitor_script, python);

    let existing = config.hooks.clone().unwrap_or_default();

    // Check whether all 6 events already have the correct command.
    let all_ok = hooks_are_equal(&existing, &our_hooks);

    if all_ok {
        return Ok("already_ok".to_string());
    }

    // Merge: keep existing hooks for events we don't manage, replace ours.
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

/// Remove the 6 AgentPulse Codex hook events from config.toml.
pub fn unregister_codex_hooks(config_path: &Path) -> Result<String, String> {
    if !config_path.exists() {
        return Ok("no_config_file".to_string());
    }

    let config = load_codex_config(config_path);
    let existing = config.hooks.clone().unwrap_or_default();

    let cleared = CodexHooksToml {
        session_start: vec![],
        pre_tool_use: vec![],
        post_tool_use: vec![],
        permission_request: vec![],
        user_prompt_submit: vec![],
        stop: vec![],
    };

    let merged = merge_codex_hooks(&existing, &cleared);
    save_codex_config(config_path, &merged)?;

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
```

- [ ] **Step 6: Run Codex hooks tests**

```powershell
cd apps/desktop/src-tauri; cargo test codex_ -- --nocapture
```
Expected: 6 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/hooks.rs
git commit -m "feat: add Codex TOML config management to hooks module"
```

---

### Task 4: Add Codex hook auto-install on startup to lib.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Update lib.rs to auto-install Codex hooks at startup**

In `lib.rs`, find the existing `setup` closure where CC hooks are installed (the `std::thread::spawn` block handling `ensure_hooks_installed`). Add a second `std::thread::spawn` for Codex hooks. The code after modification should have two parallel hook install attempts.

Locate the `std::thread::spawn(move || {` block inside `.setup(...)`. After that block closes (after `});`), add the following for Codex:

```rust
// Ensure Codex hooks are installed on every launch (idempotent).
let app_handle2 = app.handle().clone();
let python2 = python_for_hooks.clone();
std::thread::spawn(move || {
    let resource_dir = match app_handle2.path().resource_dir() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, "failed to get resource_dir for codex hook extraction");
            return;
        }
    };
    let app_data_dir = match app_handle2.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, "failed to get app_data_dir for codex hook extraction");
            return;
        }
    };
    let codex_config_path = match app_handle2
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to resolve codex config path");
            return;
        }
    };

    match hooks::extract_monitor_script(&resource_dir, &app_data_dir) {
        Ok(monitor_path) => {
            match hooks::ensure_codex_hooks_installed(
                &codex_config_path,
                &monitor_path.to_string_lossy(),
                &python2,
            ) {
                Ok(status) => tracing::info!(status = %status, "Codex AgentPulse hooks"),
                Err(e) => tracing::error!(error = %e, "failed to ensure codex hooks installed"),
            }
        }
        Err(e) => tracing::error!(error = %e, "failed to extract monitor script for codex"),
    }
});
```

- [ ] **Step 2: Verify compilation**

```powershell
cd apps/desktop/src-tauri; cargo check
```
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs
git commit -m "feat: auto-install Codex hooks on startup"
```

---

### Task 5: Add Codex Tauri commands to commands.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs` (register new commands)

- [ ] **Step 1: Add Codex hook commands to commands.rs**

Add after the existing `uninstall_hooks_cmd` function in `commands.rs`:

```rust
#[tauri::command]
pub fn get_codex_hook_status_cmd(app_handle: tauri::AppHandle) -> Result<HashMap<String, bool>, String> {
    tracing::debug!("get_codex_hook_status_cmd");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::get_codex_hook_status(&config_path)
}

#[tauri::command]
pub fn install_codex_hooks_cmd(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
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
    let monitor_path = hooks::extract_monitor_script(&resource_dir, &app_data_dir)?;
    let python = hooks::resolve_python(state.config.python.as_deref());
    hooks::ensure_codex_hooks_installed(&config_path, &monitor_path.to_string_lossy(), &python)
}

#[tauri::command]
pub fn uninstall_codex_hooks_cmd(app_handle: tauri::AppHandle) -> Result<String, String> {
    tracing::info!("user triggered codex hook removal");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::unregister_codex_hooks(&config_path)
}
```

- [ ] **Step 2: Register new commands in lib.rs**

In `lib.rs`, find the `.invoke_handler(tauri::generate_handler![...])` section and add the three new commands:

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_sessions,
    commands::get_session_detail,
    commands::get_session_events,
    commands::delete_session,
    commands::get_hook_status_cmd,
    commands::install_hooks_cmd,
    commands::uninstall_hooks_cmd,
    commands::hide_main_window,
    commands::log_event,
    commands::get_config,
    commands::get_codex_hook_status_cmd,
    commands::install_codex_hooks_cmd,
    commands::uninstall_codex_hooks_cmd,
])
```

- [ ] **Step 3: Verify compilation**

```powershell
cd apps/desktop/src-tauri; cargo check
```
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/commands.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat: add Codex hook management Tauri commands"
```

---

### Task 6: Create `adapters/codex/install_hooks.py`

**Files:**
- Create: `adapters/codex/__init__.py` (empty)
- Create: `adapters/codex/install_hooks.py`

- [ ] **Step 1: Write the Codex hook installer**

Create `adapters/codex/install_hooks.py`:

```python
#!/usr/bin/env python3
"""
Install Codex hooks for AgentPulse monitoring.

Adds hook configuration to ~/.codex/config.toml (user-level) in TOML format
so AgentPulse receives lifecycle events from all Codex CLI sessions.

The hook data is passed via stdin as JSON — Codex CLI uses the same
command-type hook mechanism as Claude Code.

Usage:
  python install_hooks.py                  # Install hooks (idempotent)
  python install_hooks.py --remove          # Remove hooks
  python install_hooks.py --status          # Show install status
  python install_hooks.py --dry-run         # Preview changes without modifying
  python install_hooks.py --force           # Force reinstall existing hooks
"""
import argparse
import shutil
import sys
from pathlib import Path

HOOK_EVENTS = [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "UserPromptSubmit",
    "Stop",
]

DEFAULT_CONFIG_PATH = Path.home() / ".codex" / "config.toml"


def get_adapter_path() -> str:
    """Get absolute path to monitor_hook.py (same dir as this script)."""
    return str(Path(__file__).parent / "monitor_hook.py")


def build_hook_configs(monitor_script: str) -> dict:
    """Build the [hooks] section dict for all 6 Codex hook events."""
    escaped = monitor_script.replace("\\", "\\\\")
    hooks_config = {}
    for event in HOOK_EVENTS:
        hooks_config[event] = [
            {
                "matcher": "",
                "hooks": [
                    {"type": "command", "command": f'python "{escaped}"'}
                ],
            }
        ]
    return hooks_config


def load_config_toml(path: Path) -> dict:
    """Load the TOML config file. Returns empty dict if missing or unparseable.

    Since Python's stdlib has no TOML parser, we read the raw file and do
    a best-effort manual merge.  We treat the file as a raw string and
    splice in our [hooks] section.
    """
    if not path.exists():
        return {"_raw": "", "_sections": {}}

    with open(path, "r", encoding="utf-8") as f:
        raw = f.read()

    # Parse into a simplified model: track section names and their raw text.
    return {"_raw": raw, "_sections": _parse_toml_sections(raw)}


def _parse_toml_sections(raw: str) -> dict:
    """Parse TOML into {section_name: raw_lines} dict.

    This is a minimal parser that handles the subset of TOML used by
    Codex config files.  It only cares about top-level [section] headers.
    """
    sections = {}
    current_section = "_top"
    current_lines = []

    for line in raw.split("\n"):
        stripped = line.strip()
        # Top-level section header: [section_name]
        if stripped.startswith("[") and stripped.endswith("]") and not stripped.startswith("[["):
            sections[current_section] = "\n".join(current_lines)
            current_section = stripped[1:-1]
            current_lines = []
        else:
            current_lines.append(line)

    sections[current_section] = "\n".join(current_lines)
    return sections


def _format_hook_entry(event: str, command: str) -> str:
    """Format a single hook event entry as TOML."""
    return f'''{event} = [
  {{ matcher = "", hooks = [
    {{ type = "command", command = '{command}' }}
  ]}}
]'''


def merge_hooks_to_raw(raw: str, monitor_script: str) -> str:
    """Merge our 6 hook events into the raw TOML content.

    If a [hooks] section already exists, we add/update our 6 events within
    it while preserving all other entries.  If not, we append a new [hooks]
    section at the end.
    """
    sections = _parse_toml_sections(raw)
    command = f"python \"{monitor_script.replace(chr(92), chr(92)+chr(92))}\""

    # Build our hook entries as TOML text
    our_entries = {}
    for event in HOOK_EVENTS:
        our_entries[event] = _format_hook_entry(event, command)

    if "hooks" in sections:
        # Merge into existing [hooks] section
        existing = sections["hooks"]
        # Remove any existing entries for our events, then append ours
        lines = existing.strip().split("\n")
        new_lines = []
        skip_until_next = False
        for line in lines:
            stripped = line.strip()
            # Check if this line starts a hook event that we manage
            is_our_event = any(
                stripped.startswith(f'{e} =') for e in HOOK_EVENTS
            )
            if is_our_event:
                skip_until_next = True
                continue
            if skip_until_next:
                # Skip until we hit a line that starts a new top-level key
                # (not indented, containing '=')
                if "=" in stripped and not stripped.startswith((" ", "\t", "{")):
                    skip_until_next = False
                    new_lines.append(line)
                continue
            new_lines.append(line)

        # Append our entries
        for event in HOOK_EVENTS:
            new_lines.append(our_entries[event])

        sections["hooks"] = "\n".join(new_lines)
    else:
        # Add a new [hooks] section
        entries = "\n".join(our_entries.values())
        sections["hooks"] = entries

    # Reassemble
    # Handle the case where the original file has explicit section ordering
    result_parts = []
    seen_hooks = False
    # Try to preserve original order by scanning the raw text
    processed_sections = set()
    current_section = "_top"
    current_lines = []

    for line in raw.split("\n"):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]") and not stripped.startswith("[["):
            # Flush previous section
            if current_section not in processed_sections:
                processed_sections.add(current_section)
            current_section = stripped[1:-1]
            current_lines = []
        else:
            current_lines.append(line)

    # Build output: iterate through the original section order
    output_lines = []

    # We'll use a simpler approach: rebuild from sections dict, but put hooks last
    for section_name, content in sorted(sections.items()):
        if section_name == "_top":
            if content.strip():
                output_lines.append(content.strip())
        elif section_name == "hooks":
            pass  # Will add at end
        else:
            output_lines.append(f"[{section_name}]")
            if content.strip():
                output_lines.append(content.strip())

    # Add hooks section last
    if "hooks" in sections:
        output_lines.append("[hooks]")
        output_lines.append(sections["hooks"].strip())

    return "\n\n".join(output_lines) + "\n"


def install(config_path: Path, force: bool = False) -> str:
    """Install Codex hooks. Returns 'installed' or 'already_installed'."""
    adapter_path = get_adapter_path()

    config_path.parent.mkdir(parents=True, exist_ok=True)

    # Read existing config (if any)
    if config_path.exists():
        existing_raw = config_path.read_text(encoding="utf-8")
    else:
        existing_raw = ""

    if not force:
        # Check if all 6 events are already installed by scanning the raw text
        all_installed = all(
            f"{event} =" in existing_raw for event in HOOK_EVENTS
        )
        if all_installed:
            return "already_installed"

    # Backup before modifying
    if config_path.exists():
        backup_path = config_path.with_suffix(".toml.bak")
        shutil.copy2(config_path, backup_path)

    new_raw = merge_hooks_to_raw(existing_raw, adapter_path)
    config_path.write_text(new_raw, encoding="utf-8")
    return "installed"


def remove_hooks(config_path: Path) -> str:
    """Remove AgentPulse hooks from config.toml. Returns 'removed' or 'no_config_file'."""
    if not config_path.exists():
        return "no_config_file"

    backup_path = config_path.with_suffix(".toml.bak")
    shutil.copy2(config_path, backup_path)

    raw = config_path.read_text(encoding="utf-8")
    sections = _parse_toml_sections(raw)

    if "hooks" not in sections:
        return "removed"  # Nothing to remove

    # Remove our 6 events from the hooks section
    existing = sections["hooks"]
    lines = existing.strip().split("\n")
    new_lines = []
    skip_until_next = False
    for line in lines:
        stripped = line.strip()
        is_our_event = any(
            stripped.startswith(f'{e} =') for e in HOOK_EVENTS
        )
        if is_our_event:
            skip_until_next = True
            continue
        if skip_until_next:
            if "=" in stripped and not stripped.startswith((" ", "\t", "{")):
                skip_until_next = False
                new_lines.append(line)
            continue
        new_lines.append(line)

    sections["hooks"] = "\n".join(new_lines)

    # Reassemble
    output_lines = []
    for section_name, content in sorted(sections.items()):
        if section_name == "_top":
            if content.strip():
                output_lines.append(content.strip())
        elif section_name == "hooks":
            if content.strip():
                output_lines.append("[hooks]")
                output_lines.append(content.strip())
        else:
            output_lines.append(f"[{section_name}]")
            if content.strip():
                output_lines.append(content.strip())

    config_path.write_text("\n\n".join(output_lines) + "\n", encoding="utf-8")
    return "removed"


def status(config_path: Path) -> dict:
    """Return hook status dict for the given config path."""
    if not config_path.exists():
        return {event: False for event in HOOK_EVENTS}

    raw = config_path.read_text(encoding="utf-8")
    return {event: f"{event} =" in raw for event in HOOK_EVENTS}


def dry_run(config_path: Path) -> dict:
    """Return dict describing what install would do."""
    adapter_path = get_adapter_path()
    current_status = status(config_path)
    return {
        "hooks_to_install": [e for e, v in current_status.items() if not v],
        "hooks_already_installed": [e for e, v in current_status.items() if v],
        "monitor_script": adapter_path,
        "config_path": str(config_path),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Install/uninstall AgentPulse Codex hooks"
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--remove", action="store_true", help="Remove hooks")
    group.add_argument("--status", action="store_true", help="Show hook install status")
    group.add_argument("--dry-run", action="store_true", help="Preview changes without modifying")
    parser.add_argument("--force", action="store_true", help="Force overwrite existing hooks")
    parser.add_argument(
        "--config",
        type=str,
        default=str(DEFAULT_CONFIG_PATH),
        help="Path to config.toml",
    )
    args = parser.parse_args()

    config_path = Path(args.config)

    if args.remove:
        result = remove_hooks(config_path)
        if result == "no_config_file":
            print("No config.toml file found. Nothing to remove.")
        else:
            print(f"AgentPulse Codex hooks removed from {config_path}")
    elif args.status:
        s = status(config_path)
        for event in HOOK_EVENTS:
            mark = "[OK]" if s[event] else "[--]"
            print(f"  {mark} {event}")
    elif args.dry_run:
        info = dry_run(config_path)
        print(f"Config path: {info['config_path']}")
        print(f"Monitor script: {info['monitor_script']}")
        for event in info["hooks_already_installed"]:
            print(f"  [SKIP] {event} (already installed)")
        for event in info["hooks_to_install"]:
            print(f"  [WILL INSTALL] {event}")
    else:
        print("Installing AgentPulse Codex hooks...")
        print(f"Adapter: {get_adapter_path()}")
        print()
        result = install(config_path, force=args.force)
        if result == "already_installed":
            print("All Codex hooks already installed. Use --force to reinstall.")
        else:
            for event in HOOK_EVENTS:
                print(f"  [OK] {event}")
            print()
            print(f"Hooks installed to {config_path}")
            print("AgentPulse will now receive events from all Codex CLI sessions.")
            print("Make sure the AgentPulse desktop app is running.")


if __name__ == "__main__":
    main()
```

Create empty `adapters/codex/__init__.py`:

```python
# AgentPulse Codex CLI adapter
```

- [ ] **Step 2: Verify Python syntax**

```powershell
python -c "import ast; ast.parse(open('adapters/codex/install_hooks.py').read()); print('OK')"
```
Expected: `OK`

- [ ] **Step 3: Complete manual smoke test of install_hooks.py**

```powershell
python adapters/codex/install_hooks.py --dry-run --config $env:TEMP\test_codex_config.toml
```
Expected: Shows 6 hooks to install.

- [ ] **Step 4: Commit**

```bash
git add adapters/codex/__init__.py adapters/codex/install_hooks.py
git commit -m "feat: add Codex hook installer for ~/.codex/config.toml"
```

---

### Task 7: Create `adapters/codex/monitor_hook.py`

**Files:**
- Create: `adapters/codex/monitor_hook.py`

- [ ] **Step 1: Write the Codex monitor hook**

Create `adapters/codex/monitor_hook.py`. This is a thin wrapper around the Claude Code version — the only difference is that it injects `"agent_source": "codex"` into the JSON before POSTing:

```python
#!/usr/bin/env python3
"""
Codex CLI monitor hook for AgentPulse.

Reads hook JSON from stdin (Codex CLI passes hook data via stdin),
injects `agent_source: "codex"` and `process_pid`, then POSTs to the
local AgentPulse event server.

The hook JSON format from Codex is structurally identical to Claude Code:
  - session_id, cwd, hook_event_name, transcript_path
  - Plus Codex-specific fields: model, permission_mode, turn_id

Usage in ~/.codex/config.toml:
  [hooks]
  SessionStart = [
    { matcher = "", hooks = [
      { type = "command", command = "python /path/to/monitor_hook.py" }
    ]}
  ]

Environment variables:
  AGENTPULSE_URL         - Event server URL (default: http://127.0.0.1:17878/api/events)
  AGENTPULSE_TIMEOUT     - Request timeout in seconds (default: 5)
  AGENTPULSE_LOG_LEVEL   - Logging level: DEBUG, INFO, WARNING, ERROR (default: INFO)
"""
import argparse
import ctypes
import json
import logging
import os
import sys
import time
import urllib.error
import urllib.request

AGENTPULSE_URL = os.environ.get(
    "AGENTPULSE_URL", "http://127.0.0.1:17878/api/events"
)
DEFAULT_TIMEOUT = int(os.environ.get("AGENTPULSE_TIMEOUT", "5"))
MAX_RETRIES = 3
RETRY_DELAY = 1.0

logging.basicConfig(
    level=getattr(logging, os.environ.get("AGENTPULSE_LOG_LEVEL", "INFO")),
    format="%(asctime)s [AgentPulse:Codex] %(levelname)s %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)


# Shell process names that sit between the agent and our hook script.
_SHELL_NAMES = frozenset({
    "cmd.exe", "powershell.exe", "pwsh.exe",
    "sh.exe", "bash.exe", "conhost.exe",
})

# Recognised agent binary names (used for PID detection fallback).
_AGENT_BINARIES = frozenset({
    "node.exe",       # Claude Code
    "codex.exe",      # Codex CLI
    "codex",          # Codex CLI (Linux/macOS)
})


def read_stdin() -> dict | None:
    """Read hook JSON from stdin. Returns None if empty, exits on parse error."""
    raw_input = sys.stdin.read().strip()
    if not raw_input:
        logger.info("No stdin data, skipping")
        return None
    try:
        return json.loads(raw_input)
    except json.JSONDecodeError as e:
        logger.error("Failed to parse stdin as JSON: %s", e)
        sys.exit(1)


def _find_agent_pid() -> int:
    """Walk up the parent chain to find the agent process PID.

    The agent spawns hook commands through a shell (cmd.exe / powershell.exe),
    so ``os.getppid()`` returns the shell PID which exits instantly. We walk
    upward until we find a non-shell process — ideally a recognised agent
    binary — and return its PID.

    Falls back to ``os.getppid()`` on error or non-Windows platforms.
    """
    if sys.platform != "win32":
        return os.getppid()

    try:
        pid_to_parent, pid_to_name = _snapshot_processes()

        # Walk up from the current process, skipping known shell wrappers.
        cur = os.getpid()
        last_non_shell = cur
        for _ in range(5):  # safety limit
            parent = pid_to_parent.get(cur)
            if parent is None:
                break
            name = pid_to_name.get(parent, "").lower()
            if name not in _SHELL_NAMES:
                last_non_shell = parent
                # If we hit a recognised agent, return immediately.
                if name in _AGENT_BINARIES:
                    return parent
                # Otherwise keep walking — we may be inside a nested tool call.
            cur = parent

        return last_non_shell or os.getppid()
    except Exception:
        return os.getppid()


def _snapshot_processes() -> tuple[dict[int, int], dict[int, str]]:
    """Take a process snapshot and return (pid→parent_pid, pid→name)."""
    TH32CS_SNAPPROCESS = 0x00000002
    INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value

    class PROCESSENTRY32(ctypes.Structure):
        _fields_ = [
            ("dwSize", ctypes.c_ulong),
            ("cntUsage", ctypes.c_ulong),
            ("th32ProcessID", ctypes.c_ulong),
            ("th32DefaultHeapID", ctypes.POINTER(ctypes.c_ulong)),
            ("th32ModuleID", ctypes.c_ulong),
            ("cntThreads", ctypes.c_ulong),
            ("th32ParentProcessID", ctypes.c_ulong),
            ("pcPriClassBase", ctypes.c_long),
            ("dwFlags", ctypes.c_ulong),
            ("szExeFile", ctypes.c_char * 260),
        ]

    kernel32 = ctypes.windll.kernel32
    snapshot = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snapshot == INVALID_HANDLE_VALUE:
        return {}, {}

    pid_to_parent: dict[int, int] = {}
    pid_to_name: dict[int, str] = {}

    entry = PROCESSENTRY32()
    entry.dwSize = ctypes.sizeof(PROCESSENTRY32)

    if kernel32.Process32First(snapshot, ctypes.byref(entry)):
        while True:
            pid = entry.th32ProcessID
            pid_to_parent[pid] = entry.th32ParentProcessID
            pid_to_name[pid] = entry.szExeFile.decode("utf-8", errors="replace")
            if not kernel32.Process32Next(snapshot, ctypes.byref(entry)):
                break

    kernel32.CloseHandle(snapshot)
    return pid_to_parent, pid_to_name


def send_event(url: str, data: dict, timeout: int) -> int:
    """POST event JSON to AgentPulse server. Returns HTTP status or -1 on failure."""
    body = json.dumps(data).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    last_error = None
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                status = resp.status
                if status == 201:
                    logger.info("Event sent successfully (attempt %d)", attempt)
                else:
                    logger.warning(
                        "Server returned %d (attempt %d)", status, attempt
                    )
                return status
        except urllib.error.HTTPError as e:
            logger.warning(
                "Server returned %d (attempt %d)", e.code, attempt
            )
            return e.code
        except (urllib.error.URLError, OSError) as e:
            last_error = e
            if attempt < MAX_RETRIES:
                logger.warning(
                    "Connection failed (attempt %d/%d): %s",
                    attempt,
                    MAX_RETRIES,
                    e,
                )
                time.sleep(RETRY_DELAY)

    logger.error(
        "Failed to send event after %d attempts: %s", MAX_RETRIES, last_error
    )
    return -1


def main():
    parser = argparse.ArgumentParser(
        description="AgentPulse Codex CLI monitor hook"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="Print payload to stdout instead of POSTing to server",
    )
    args = parser.parse_args()

    hook_data = read_stdin()
    if hook_data is None:
        sys.exit(0)

    # Inject the agent source so the server can route to normalize_codex_event.
    hook_data["agent_source"] = "codex"

    # Walk up the process tree to find the agent PID.
    hook_data["process_pid"] = _find_agent_pid()

    if args.test:
        print(json.dumps(hook_data, indent=2))
        sys.exit(0)

    status = send_event(AGENTPULSE_URL, hook_data, DEFAULT_TIMEOUT)
    if status < 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Verify Python syntax**

```powershell
python -c "import ast; ast.parse(open('adapters/codex/monitor_hook.py').read()); print('OK')"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add adapters/codex/monitor_hook.py
git commit -m "feat: add Codex monitor hook with agent_source injection"
```

---

### Task 8: Update CC monitor_hook.py PID detection

**Files:**
- Modify: `adapters/claude-code/monitor_hook.py`

- [ ] **Step 1: Update PID walking to be agent-agnostic**

Update `monitor_hook.py:69-97` — rename `_walk_process_tree_to_cc` to `_find_agent_pid` and broaden the binary detection:

Replace lines 52-53 (`_SHELL_NAMES` constant) with:

```python
_SHELL_NAMES = frozenset({
    "cmd.exe", "powershell.exe", "pwsh.exe",
    "sh.exe", "bash.exe", "conhost.exe",
})

_AGENT_BINARIES = frozenset({
    "node.exe",       # Claude Code
    "codex.exe",      # Codex CLI (Windows)
    "codex",          # Codex CLI (Linux/macOS)
    "gemini",         # Gemini CLI
    "copilot",        # GitHub Copilot CLI
})
```

Replace the `_walk_process_tree_to_cc` function (lines 69-97) and its call site with the updated version:

In `main()`, change:
```python
hook_data["process_pid"] = _walk_process_tree_to_cc()
```
to:
```python
hook_data["process_pid"] = _find_agent_pid()
```

Replace the entire `_walk_process_tree_to_cc` function with:

```python
def _find_agent_pid() -> int:
    """Walk up the parent chain to find the agent process PID.

    The agent spawns hook commands through a shell (cmd.exe / powershell.exe),
    so ``os.getppid()`` returns the shell PID which exits instantly. We walk
    upward until we find a non-shell process, preferably a recognised agent
    binary, and return its PID.

    Falls back to ``os.getppid()`` on error or non-Windows platforms.
    """
    if sys.platform != "win32":
        return os.getppid()

    try:
        pid_to_parent, pid_to_name = _snapshot_processes()

        cur = os.getpid()
        last_non_shell = cur
        for _ in range(5):  # safety limit
            parent = pid_to_parent.get(cur)
            if parent is None:
                break
            name = pid_to_name.get(parent, "").lower()
            if name not in _SHELL_NAMES:
                last_non_shell = parent
                if name in _AGENT_BINARIES:
                    return parent
            cur = parent

        return last_non_shell or os.getppid()
    except Exception:
        return os.getppid()
```

- [ ] **Step 2: Verify Python syntax**

```powershell
python -c "import ast; ast.parse(open('adapters/claude-code/monitor_hook.py').read()); print('OK')"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add adapters/claude-code/monitor_hook.py
git commit -m "refactor: make PID detection agent-agnostic (_find_agent_pid)"
```

---

### Task 9: Rust integration tests

**Files:**
- Modify: `apps/desktop/src-tauri/tests/event_server_test.rs`

- [ ] **Step 1: Add integration-style test for full Codex event chain**

Add to `event_server_test.rs`:

```rust
#[test]
fn test_codex_full_session_lifecycle_normalizes_all_events() {
    // Simulate a complete Codex session with all 6 event types
    let session_id = "cx-lifecycle-001";

    let events = vec![
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "SessionStart",
            "transcript_path": "/tmp/transcript.json",
            "model": "gpt-5",
            "permission_mode": "default",
            "source": "startup",
            "turn_id": "turn-1",
            "process_pid": 9999
        }), AgentStatus::Starting),
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {"path": "main.rs"},
            "tool_use_id": "tu-1",
            "turn_id": "turn-1",
            "transcript_path": null,
            "model": "gpt-5",
            "permission_mode": "default"
        }), AgentStatus::ToolRunning),
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "PostToolUse",
            "tool_name": "Write",
            "tool_input": {"path": "main.rs"},
            "tool_response": null,
            "tool_use_id": "tu-1",
            "turn_id": "turn-1",
            "transcript_path": null,
            "model": "gpt-5",
            "permission_mode": "default"
        }), AgentStatus::Running),
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "PermissionRequest",
            "tool_name": "Bash",
            "tool_input": {"command": "rm file"},
            "turn_id": "turn-1",
            "transcript_path": null,
            "model": "gpt-5",
            "permission_mode": "default"
        }), AgentStatus::WaitingPermission),
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "please continue",
            "turn_id": "turn-2",
            "transcript_path": null,
            "model": "gpt-5",
            "permission_mode": "default"
        }), AgentStatus::Running),
        (serde_json::json!({
            "agent_source": "codex",
            "session_id": session_id,
            "cwd": "/home/user/cx-proj",
            "hook_event_name": "Stop",
            "last_assistant_message": "All done!",
            "stop_hook_active": false,
            "turn_id": "turn-2",
            "transcript_path": null,
            "model": "gpt-5",
            "permission_mode": "default"
        }), AgentStatus::Completed),
    ];

    for (i, (raw, expected_status)) in events.iter().enumerate() {
        let event = normalize_event_by_source(raw);
        assert_eq!(
            event.source,
            AgentSource::Codex,
            "event[{i}]: source should be Codex"
        );
        assert_eq!(
            event.session_id, session_id,
            "event[{i}]: session_id mismatch"
        );
        assert_eq!(
            event.status, *expected_status,
            "event[{i}]: expected status {expected_status:?}, got {:?}",
            event.status
        );
    }
}
```

- [ ] **Step 2: Run the lifecycle test**

```powershell
cd apps/desktop/src-tauri; cargo test test_codex_full_session -- --nocapture
```
Expected: PASS.

- [ ] **Step 3: Run the full test suite**

```powershell
cd apps/desktop/src-tauri; cargo test
```
Expected: ALL tests PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/tests/event_server_test.rs
git commit -m "test: add Codex full session lifecycle normalization test"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ `normalize_codex_event()` — Task 1
- ✅ Route by `agent_source` field — Task 2
- ✅ Codex TOML config management (install/remove/status) — Task 3
- ✅ Startup auto-install Codex hooks — Task 4
- ✅ Codex Tauri commands — Task 5
- ✅ `adapters/codex/install_hooks.py` — Task 6
- ✅ `adapters/codex/monitor_hook.py` — Task 7
- ✅ PID detection agent-agnostic — Task 8
- ✅ Integration tests — Task 9

**2. Placeholder scan:** No TBD, TODO, or vague references. All code is concrete.

**3. Type consistency:**
- `normalize_codex_event` returns `AgentEvent` (same as CC version)
- `normalize_event_by_source` delegates to either normalizer
- Codex TOML types match the hooks module's internal conventions
- Python functions match existing install_hooks.py patterns
