use agentpulse_lib::event_server::*;
use agentpulse_lib::*;

#[test]
fn test_normalize_event_determines_status() {
    let raw = serde_json::json!({
        "session_id": "sess-001",
        "cwd": "/home/user/project",
        "hook_event_name": "SessionStart",
        "transcript_path": "/tmp/transcript.json"
    });

    let event = normalize_claude_code_event(&raw);
    assert_eq!(event.source, AgentSource::ClaudeCode);
    assert_eq!(event.event_type, EventType::SessionStart);
    assert_eq!(event.status, AgentStatus::Starting);
    assert_eq!(event.project_name, Some("project".into()));
}

#[test]
fn test_normalize_pre_tool_use() {
    let raw = serde_json::json!({
        "session_id": "sess-001",
        "cwd": "/tmp",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "npm test"}
    });

    let event = normalize_claude_code_event(&raw);
    assert_eq!(event.event_type, EventType::PreToolUse);
    assert_eq!(event.status, AgentStatus::ToolRunning);
    assert_eq!(event.tool_name, Some("Bash".into()));
}

#[test]
fn test_normalize_stop() {
    let raw = serde_json::json!({
        "session_id": "sess-001",
        "cwd": "/tmp",
        "hook_event_name": "Stop",
        "last_assistant_message": "Task complete"
    });

    let event = normalize_claude_code_event(&raw);
    assert_eq!(event.event_type, EventType::Stop);
    assert_eq!(event.status, AgentStatus::Completed);
    assert_eq!(event.message, Some("Task complete".into()));
}

#[test]
fn test_normalize_notification_permission() {
    let raw = serde_json::json!({
        "session_id": "sess-001",
        "cwd": "/tmp",
        "hook_event_name": "Notification",
        "notification_type": "permission_prompt",
        "message": "Claude needs permission to run Bash"
    });

    let event = normalize_claude_code_event(&raw);
    assert_eq!(event.event_type, EventType::PermissionRequest);
    assert_eq!(event.status, AgentStatus::WaitingPermission);
}

#[test]
fn test_normalize_unknown_event_keeps_running() {
    let raw = serde_json::json!({
        "session_id": "sess-001",
        "cwd": "/tmp",
        "hook_event_name": "UserPromptSubmit"
    });

    let event = normalize_claude_code_event(&raw);
    assert_eq!(event.status, AgentStatus::Running);
}

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
