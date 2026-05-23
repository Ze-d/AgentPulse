use agentpulse_lib::*;

#[test]
fn test_agent_event_serialization() {
    let event = AgentEvent {
        id: "evt-001".into(),
        source: AgentSource::ClaudeCode,
        session_id: "sess-001".into(),
        cwd: "/home/user/project".into(),
        project_name: Some("project".into()),
        event_type: EventType::SessionStart,
        status: AgentStatus::Starting,
        message: None,
        tool_name: None,
        transcript_path: None,
        created_at: 1700000000000,
    };

    let json = serde_json::to_string(&event).unwrap();
    let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "evt-001");
    assert_eq!(parsed.source, AgentSource::ClaudeCode);
    assert_eq!(parsed.status, AgentStatus::Starting);
}

#[test]
fn test_agent_session_defaults() {
    let session = AgentSession {
        session_id: "sess-001".into(),
        source: AgentSource::ClaudeCode,
        cwd: "/tmp".into(),
        project_name: "tmp".into(),
        status: AgentStatus::Unknown,
        started_at: 1700000000000,
        updated_at: 1700000000000,
        completed_at: None,
        last_message: None,
        last_tool_name: None,
        transcript_path: None,
        needs_attention: false,
    };
    assert_eq!(session.status, AgentStatus::Unknown);
    assert!(!session.needs_attention);
}

#[test]
fn test_deserialize_agent_event_from_json() {
    let json = r#"{
        "id": "evt-002",
        "source": "claude-code",
        "sessionId": "sess-002",
        "cwd": "/tmp",
        "eventType": "stop",
        "status": "completed",
        "message": "done",
        "createdAt": 1700000000000
    }"#;

    let event: AgentEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event.source, AgentSource::ClaudeCode);
    assert_eq!(event.event_type, EventType::Stop);
    assert_eq!(event.status, AgentStatus::Completed);
}
