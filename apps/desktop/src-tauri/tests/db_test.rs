use agentpulse_lib::db::Database;
use agentpulse_lib::*;

fn setup_db() -> Database {
    Database::new_in_memory().unwrap()
}

#[test]
fn test_create_and_get_session() {
    let db = setup_db();
    let session = AgentSession {
        session_id: "sess-001".into(),
        source: AgentSource::ClaudeCode,
        cwd: "/home/user/project".into(),
        project_name: "project".into(),
        status: AgentStatus::Running,
        started_at: 1700000000000,
        updated_at: 1700000000100,
        completed_at: None,
        last_message: None,
        last_tool_name: None,
        transcript_path: None,
        needs_attention: false,
    };

    db.upsert_session(&session).unwrap();
    let got = db.get_session("sess-001").unwrap().unwrap();
    assert_eq!(got.session_id, "sess-001");
    assert_eq!(got.status, AgentStatus::Running);
    assert_eq!(got.project_name, "project");
}

#[test]
fn test_insert_event() {
    let db = setup_db();
    let session = AgentSession {
        session_id: "sess-002".into(),
        source: AgentSource::ClaudeCode,
        cwd: "/tmp".into(),
        project_name: "tmp".into(),
        status: AgentStatus::Starting,
        started_at: 1700000000000,
        updated_at: 1700000000000,
        completed_at: None,
        last_message: None,
        last_tool_name: None,
        transcript_path: None,
        needs_attention: false,
    };
    db.upsert_session(&session).unwrap();

    let event = AgentEvent {
        id: "evt-001".into(),
        source: AgentSource::ClaudeCode,
        session_id: "sess-002".into(),
        cwd: "/tmp".into(),
        project_name: Some("tmp".into()),
        event_type: EventType::SessionStart,
        status: AgentStatus::Starting,
        message: None,
        tool_name: None,
        transcript_path: None,
        created_at: 1700000000000,
    };

    db.insert_event(&event).unwrap();
    let events = db.get_events_for_session("sess-002").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "evt-001");
}

#[test]
fn test_list_active_sessions() {
    let db = setup_db();
    let running = AgentSession {
        session_id: "sess-A".into(),
        source: AgentSource::ClaudeCode,
        cwd: "/a".into(),
        project_name: "a".into(),
        status: AgentStatus::Running,
        started_at: 1700000000000,
        updated_at: 1700000000000,
        completed_at: None,
        last_message: None,
        last_tool_name: None,
        transcript_path: None,
        needs_attention: false,
    };
    let completed = AgentSession {
        session_id: "sess-B".into(),
        source: AgentSource::ClaudeCode,
        cwd: "/b".into(),
        project_name: "b".into(),
        status: AgentStatus::Completed,
        started_at: 1700000000000,
        updated_at: 1700000000100,
        completed_at: Some(1700000000100),
        last_message: None,
        last_tool_name: None,
        transcript_path: None,
        needs_attention: false,
    };

    db.upsert_session(&running).unwrap();
    db.upsert_session(&completed).unwrap();

    let active = db.list_active_sessions().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].session_id, "sess-A");
}
