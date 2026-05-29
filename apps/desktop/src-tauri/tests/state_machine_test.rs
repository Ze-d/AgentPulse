use agentpulse_lib::state_machine::StateMachine;
use agentpulse_lib::*;

#[test]
fn test_session_start_transitions_to_starting() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Unknown, &EventType::SessionStart);
    assert_eq!(result, AgentStatus::Starting);
}

#[test]
fn test_pre_tool_use_transitions_to_tool_running() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Running, &EventType::PreToolUse);
    assert_eq!(result, AgentStatus::ToolRunning);
}

#[test]
fn test_post_tool_use_transitions_to_running() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::ToolRunning, &EventType::PostToolUse);
    assert_eq!(result, AgentStatus::Running);
}

#[test]
fn test_stop_transitions_to_completed() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Running, &EventType::Stop);
    assert_eq!(result, AgentStatus::Completed);
}

#[test]
fn test_notification_permission_prompt() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Running, &EventType::PermissionRequest);
    assert_eq!(result, AgentStatus::WaitingPermission);
}

#[test]
fn test_failure_transition() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::ToolRunning, &EventType::Failure);
    assert_eq!(result, AgentStatus::Failed);
}

#[test]
fn test_completed_wakes_on_pretooluse() {
    let sm = StateMachine::new();
    // User resumes work after seeing "done" — tool use starts
    let result = sm.transition(AgentStatus::Completed, &EventType::PreToolUse);
    assert_eq!(result, AgentStatus::ToolRunning);
}

#[test]
fn test_completed_wakes_on_notification() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Completed, &EventType::Notification);
    assert_eq!(result, AgentStatus::WaitingInput);
}

#[test]
fn test_completed_wakes_on_sessionstart() {
    let sm = StateMachine::new();
    let result = sm.transition(AgentStatus::Completed, &EventType::SessionStart);
    assert_eq!(result, AgentStatus::Starting);
}

#[test]
fn test_failed_wakes_on_new_activity() {
    let sm = StateMachine::new();
    // Failed session should recover when new activity arrives
    let result = sm.transition(AgentStatus::Failed, &EventType::PreToolUse);
    assert_eq!(result, AgentStatus::ToolRunning);
}

#[test]
fn test_needs_attention_flags() {
    assert!(StateMachine::needs_attention(&AgentStatus::WaitingInput));
    assert!(StateMachine::needs_attention(
        &AgentStatus::WaitingPermission
    ));
    assert!(StateMachine::needs_attention(&AgentStatus::Completed));
    assert!(StateMachine::needs_attention(&AgentStatus::Failed));
    assert!(!StateMachine::needs_attention(&AgentStatus::Running));
    assert!(!StateMachine::needs_attention(&AgentStatus::Starting));
}
