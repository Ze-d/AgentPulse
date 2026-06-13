//! Cross-module integration tests.
//!
//! Verifies that DB + EventServer + StateMachine work together correctly
//! across a full session lifecycle.  Uses an in-memory database behind an
//! `Arc<Mutex<Database>>` so the test can retain a handle for assertions.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use agentpulse_lib::db::Database;
use agentpulse_lib::event_server::EventServer;
use agentpulse_lib::process_checker;
use agentpulse_lib::{AgentSource, AgentStatus};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Arc<Mutex<Database>>, EventServer) {
    let db = Database::new_in_memory().expect("in-memory db");
    let arc = Arc::new(Mutex::new(db));
    let server = EventServer::from_arc(arc.clone());
    (arc, server)
}

fn cc_session_start(session_id: &str, cwd: &str, pid: u32) -> serde_json::Value {
    json!({
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "cwd": cwd,
        "process_pid": pid,
        "transcript_path": "/tmp/transcript.json"
    })
}

fn cc_pre_tool_use(session_id: &str, tool: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "PreToolUse",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "tool_name": tool
    })
}

fn cc_post_tool_use(session_id: &str, tool: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "PostToolUse",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "tool_name": tool
    })
}

fn cc_notification_permission(session_id: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "Notification",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "notification_type": "permission_prompt",
        "message": "Approve this command?"
    })
}

fn cc_stop(session_id: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "Stop",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "last_assistant_message": "Task completed."
    })
}

fn codex_session_start(session_id: &str, cwd: &str, pid: u32) -> serde_json::Value {
    json!({
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "cwd": cwd,
        "process_pid": pid,
        "agent_source": "codex"
    })
}

fn codex_permission_request(session_id: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "PermissionRequest",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "agent_source": "codex"
    })
}

fn codex_stop(session_id: &str) -> serde_json::Value {
    json!({
        "hook_event_name": "Stop",
        "session_id": session_id,
        "cwd": "/home/user/project",
        "agent_source": "codex",
        "last_assistant_message": "Done."
    })
}

fn get_session(db: &Arc<Mutex<Database>>, session_id: &str) -> agentpulse_lib::AgentSession {
    db.lock().unwrap().get_session(session_id).unwrap().unwrap()
}

fn get_events(db: &Arc<Mutex<Database>>, session_id: &str) -> Vec<agentpulse_lib::AgentEvent> {
    db.lock().unwrap().get_events_for_session(session_id).unwrap()
}

// ---------------------------------------------------------------------------
// Full Claude Code session lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_full_cc_session_lifecycle() {
    let (db, server) = setup();

    let sid = "cc-lifecycle-1";
    let pid = 42;

    // 1. SessionStart → Starting (new session, status from event)
    let (event, session) = server
        .handle_event(&cc_session_start(sid, "/home/user/my-project", pid))
        .expect("SessionStart");
    assert_eq!(event.source, AgentSource::ClaudeCode);
    assert_eq!(session.status, AgentStatus::Starting);
    assert_eq!(session.project_name, "my-project");
    assert_eq!(session.pid, Some(pid));

    // 2. PreToolUse → Running (state machine: Starting + _ → Running)
    let (_event, session) = server
        .handle_event(&cc_pre_tool_use(sid, "Bash"))
        .expect("PreToolUse #1");
    assert_eq!(
        session.status,
        AgentStatus::Running,
        "Starting + PreToolUse → Running"
    );

    // 3. PreToolUse → ToolRunning (state machine: Running + PreToolUse → ToolRunning)
    let (_event, session) = server
        .handle_event(&cc_pre_tool_use(sid, "Bash"))
        .expect("PreToolUse #2");
    assert_eq!(session.status, AgentStatus::ToolRunning);
    assert_eq!(session.last_tool_name.as_deref(), Some("Bash"));
    assert_eq!(session.pid, Some(pid), "PID should persist");

    // 4. PostToolUse → Running (state machine: ToolRunning + PostToolUse → Running)
    let (_event, session) = server
        .handle_event(&cc_post_tool_use(sid, "Bash"))
        .expect("PostToolUse");
    assert_eq!(session.status, AgentStatus::Running);

    // 5. Notification(permission_prompt) → WaitingPermission
    //    (normalization: PermissionRequest; state machine: Running + PermissionRequest → WaitingPermission)
    let (_event, session) = server
        .handle_event(&cc_notification_permission(sid))
        .expect("Notification");
    assert_eq!(session.status, AgentStatus::WaitingPermission);

    // 6. Stop → Completed
    let (_event, session) = server.handle_event(&cc_stop(sid)).expect("Stop");
    assert_eq!(session.status, AgentStatus::Completed);
    assert!(session.completed_at.is_some());

    // Verify persisted in DB.
    let s = get_session(&db, sid);
    assert_eq!(s.status, AgentStatus::Completed);
}

// ---------------------------------------------------------------------------
// Full Codex session lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_full_codex_session_lifecycle() {
    let (db, server) = setup();

    let sid = "codex-lifecycle-1";
    let pid = 99;

    // 1. SessionStart (Codex) → Starting (new session, status from event)
    let (_event, session) = server
        .handle_event(&codex_session_start(sid, "/home/user/codex-project", pid))
        .expect("Codex SessionStart");
    assert_eq!(session.source, AgentSource::Codex);
    assert_eq!(session.status, AgentStatus::Starting);
    assert_eq!(session.project_name, "codex-project");

    // 2. PreToolUse → Running (state machine: Starting + _ → Running)
    let (_event, session) = server
        .handle_event(&cc_pre_tool_use(sid, "Write"))
        .expect("PreToolUse #1");
    assert_eq!(session.status, AgentStatus::Running);

    // 3. PreToolUse → ToolRunning (state machine: Running + PreToolUse → ToolRunning)
    let (_event, session) = server
        .handle_event(&cc_pre_tool_use(sid, "Write"))
        .expect("PreToolUse #2");
    assert_eq!(session.status, AgentStatus::ToolRunning);
    assert_eq!(session.pid, Some(pid));

    // 4. PostToolUse → Running (state machine: ToolRunning + PostToolUse → Running)
    let (_event, session) = server
        .handle_event(&cc_post_tool_use(sid, "Write"))
        .expect("PostToolUse");
    assert_eq!(session.status, AgentStatus::Running);

    // 5. PermissionRequest (Codex native) → WaitingPermission
    //    (state machine: Running + PermissionRequest → WaitingPermission)
    let (_event, session) = server
        .handle_event(&codex_permission_request(sid))
        .expect("Codex PermissionRequest");
    assert_eq!(session.status, AgentStatus::WaitingPermission);

    // 6. Stop → Completed
    let (_event, session) = server.handle_event(&codex_stop(sid)).expect("Stop");
    assert_eq!(session.status, AgentStatus::Completed);
    assert!(session.completed_at.is_some());

    let s = get_session(&db, sid);
    assert_eq!(s.status, AgentStatus::Completed);
    assert_eq!(s.source, AgentSource::Codex);
}

// ---------------------------------------------------------------------------
// Event count verification
// ---------------------------------------------------------------------------

#[test]
fn test_all_events_stored_for_full_lifecycle() {
    let (db, server) = setup();

    let sid = "events-count-1";

    server
        .handle_event(&cc_session_start(sid, "/tmp", 1))
        .unwrap();
    // Sleep 1ms between each event to ensure distinct timestamps.
    thread::sleep(Duration::from_millis(1));
    server
        .handle_event(&cc_pre_tool_use(sid, "Read"))
        .unwrap();
    thread::sleep(Duration::from_millis(1));
    server
        .handle_event(&cc_post_tool_use(sid, "Read"))
        .unwrap();
    thread::sleep(Duration::from_millis(1));
    server.handle_event(&cc_stop(sid)).unwrap();

    let events = get_events(&db, sid);
    assert_eq!(events.len(), 4, "should have 4 events");

    // With distinct timestamps, events should be newest-first.
    let event_types: Vec<String> =
        events.iter().map(|e| format!("{:?}", e.event_type)).collect();
    assert!(
        event_types[0].contains("Stop"),
        "newest should be Stop, got: {:?}",
        event_types
    );
}

// ---------------------------------------------------------------------------
// Process PID persistence
// ---------------------------------------------------------------------------

#[test]
fn test_process_pid_persists_across_events() {
    let (db, server) = setup();

    let sid = "pid-test-1";
    let pid = 7777;

    server
        .handle_event(&cc_session_start(sid, "/tmp", pid))
        .unwrap();

    // PreToolUse does NOT include process_pid — PID should persist.
    let payload_without_pid = json!({
        "hook_event_name": "PreToolUse",
        "session_id": sid,
        "cwd": "/tmp",
        "tool_name": "Grep"
    });
    let (_event, session) = server.handle_event(&payload_without_pid).unwrap();
    assert_eq!(session.pid, Some(pid), "PID should persist from SessionStart");

    // Stop also without PID.
    let stop_without_pid = json!({
        "hook_event_name": "Stop",
        "session_id": sid,
        "cwd": "/tmp"
    });
    let (_event, session) = server.handle_event(&stop_without_pid).unwrap();
    assert_eq!(session.pid, Some(pid), "PID should persist through Stop");

    let s = get_session(&db, sid);
    assert_eq!(s.pid, Some(pid));
}

// ---------------------------------------------------------------------------
// NeedsAttention flag (verified against state machine)
// ---------------------------------------------------------------------------

#[test]
fn test_needs_attention_correct_for_all_statuses() {
    // From StateMachine::needs_attention:
    //   true:  WaitingInput, WaitingPermission, Completed, Failed
    //   false: Starting, Running, ToolRunning, Unknown
    assert!(!agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::Starting
    ));
    assert!(!agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::Running
    ));
    assert!(!agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::ToolRunning
    ));
    assert!(agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::WaitingInput
    ));
    assert!(agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::WaitingPermission
    ));
    assert!(agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::Completed
    ));
    assert!(agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::Failed
    ));
}

// ---------------------------------------------------------------------------
// Process checker — terminal sessions preserved
// ---------------------------------------------------------------------------

#[test]
fn test_process_checker_skips_completed_and_failed_sessions() {
    assert!(!process_checker::is_active_status(&AgentStatus::Completed));
    assert!(!process_checker::is_active_status(&AgentStatus::Failed));
    assert!(!process_checker::is_active_status(&AgentStatus::Unknown));
    assert!(process_checker::is_active_status(&AgentStatus::Starting));
    assert!(process_checker::is_active_status(&AgentStatus::Running));
    assert!(process_checker::is_active_status(&AgentStatus::ToolRunning));
}

#[test]
fn test_completed_session_persists_in_db() {
    let (db, server) = setup();

    let sid = "completed-keep-1";

    server
        .handle_event(&cc_session_start(sid, "/tmp", 12345))
        .unwrap();
    server.handle_event(&cc_stop(sid)).unwrap();

    let s = get_session(&db, sid);
    assert_eq!(s.status, AgentStatus::Completed);

    // Session should still exist (not deleted by process checker logic).
    assert!(db.lock().unwrap().get_session(sid).unwrap().is_some());
}

// ---------------------------------------------------------------------------
// Failed state lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_failed_session_lifecycle() {
    let (db, server) = setup();

    let sid = "failed-test-1";

    server
        .handle_event(&cc_session_start(sid, "/tmp", 1))
        .unwrap();

    let failure_payload = json!({
        "hook_event_name": "PostToolUseFailure",
        "session_id": sid,
        "cwd": "/tmp",
        "tool_name": "Bash"
    });
    let (_event, session) = server.handle_event(&failure_payload).unwrap();

    // PostToolUseFailure → Failed (state machine: (_, Failure) → Failed)
    assert_eq!(session.status, AgentStatus::Failed);
    // Failed IS flagged as needs_attention.
    assert!(agentpulse_lib::state_machine::StateMachine::needs_attention(
        &AgentStatus::Failed
    ));
    // But NOT an active status (process checker skips it).
    assert!(!process_checker::is_active_status(&AgentStatus::Failed));

    let s = get_session(&db, sid);
    assert_eq!(s.status, AgentStatus::Failed);
}

// ---------------------------------------------------------------------------
// Session recovery — new activity wakes a completed session
// ---------------------------------------------------------------------------

#[test]
fn test_completed_session_wakes_on_new_activity() {
    let (db, server) = setup();

    let sid = "wake-test-1";

    server
        .handle_event(&cc_session_start(sid, "/tmp", 1))
        .unwrap();
    server.handle_event(&cc_stop(sid)).unwrap();

    let s = get_session(&db, sid);
    assert_eq!(s.status, AgentStatus::Completed);

    // New SessionStart for the same ID → should reset to Starting.
    let (_event, session) = server
        .handle_event(&cc_session_start(sid, "/tmp", 1))
        .unwrap();
    assert_eq!(session.status, AgentStatus::Starting);
    // completed_at is preserved from the prior lifecycle (not cleared on wake).
    assert!(session.completed_at.is_some());

    // project_name preserves original value (not updated on wake).
    let s = get_session(&db, sid);
    assert_eq!(s.project_name, "tmp");
    assert_eq!(s.status, AgentStatus::Starting);
}
