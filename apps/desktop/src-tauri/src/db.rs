use std::path::Path;

use rusqlite::{params, Connection, Result};

use crate::{AgentEvent, AgentSession, AgentSource, AgentStatus, EventType};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sessions (
                session_id      TEXT PRIMARY KEY,
                source          TEXT NOT NULL,
                cwd             TEXT NOT NULL,
                project_name    TEXT NOT NULL,
                status          TEXT NOT NULL,
                started_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                completed_at    INTEGER,
                last_message    TEXT,
                last_tool_name  TEXT,
                transcript_path TEXT,
                needs_attention INTEGER NOT NULL DEFAULT 0,
                pid             INTEGER
            );

            CREATE TABLE IF NOT EXISTS events (
                id              TEXT PRIMARY KEY,
                source          TEXT NOT NULL,
                session_id      TEXT NOT NULL,
                cwd             TEXT NOT NULL,
                project_name    TEXT,
                event_type      TEXT NOT NULL,
                status          TEXT NOT NULL,
                message         TEXT,
                tool_name       TEXT,
                transcript_path TEXT,
                created_at      INTEGER NOT NULL,
                process_pid     INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            );

            CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
        ",
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Serialization helpers for enum types stored as strings in SQLite
    // ------------------------------------------------------------------

    fn serialize_agent_source(source: &AgentSource) -> String {
        serde_json::to_string(source)
            .unwrap()
            .trim_matches('"')
            .to_string()
    }

    fn serialize_agent_status(status: &AgentStatus) -> String {
        serde_json::to_string(status)
            .unwrap()
            .trim_matches('"')
            .to_string()
    }

    fn serialize_event_type(event_type: &EventType) -> String {
        serde_json::to_string(event_type)
            .unwrap()
            .trim_matches('"')
            .to_string()
    }

    fn deserialize_agent_source(s: &str) -> std::result::Result<AgentSource, String> {
        serde_json::from_str(&format!("\"{s}\""))
            .map_err(|e| format!("corrupt DB value for AgentSource '{s}': {e}"))
    }

    fn deserialize_agent_status(s: &str) -> std::result::Result<AgentStatus, String> {
        serde_json::from_str(&format!("\"{s}\""))
            .map_err(|e| format!("corrupt DB value for AgentStatus '{s}': {e}"))
    }

    fn deserialize_event_type(s: &str) -> std::result::Result<EventType, String> {
        serde_json::from_str(&format!("\"{s}\""))
            .map_err(|e| format!("corrupt DB value for EventType '{s}': {e}"))
    }

    /// Map a deserialize `Result<T, String>` into a `rusqlite::Result<T>` for
    /// use inside `query_map` closures.
    fn map_deser<T>(r: std::result::Result<T, String>) -> Result<T> {
        r.map_err(rusqlite::Error::InvalidParameterName)
    }

    fn map_session_row(row: &rusqlite::Row) -> Result<AgentSession> {
        let source_str: String = row.get(1)?;
        let status_str: String = row.get(4)?;
        Ok(AgentSession {
            session_id: row.get(0)?,
            source: Self::map_deser(Self::deserialize_agent_source(&source_str))?,
            cwd: row.get(2)?,
            project_name: row.get(3)?,
            status: Self::map_deser(Self::deserialize_agent_status(&status_str))?,
            started_at: row.get(5)?,
            updated_at: row.get(6)?,
            completed_at: row.get(7)?,
            last_message: row.get(8)?,
            last_tool_name: row.get(9)?,
            transcript_path: row.get(10)?,
            needs_attention: row.get::<_, i32>(11)? != 0,
            pid: row.get(12)?,
        })
    }

    // ------------------------------------------------------------------
    // Session CRUD
    // ------------------------------------------------------------------

    pub fn upsert_session(&self, session: &AgentSession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions
                (session_id, source, cwd, project_name, status, started_at,
                 updated_at, completed_at, last_message, last_tool_name,
                 transcript_path, needs_attention, pid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(session_id) DO UPDATE SET
                source          = ?2,
                cwd             = ?3,
                project_name    = ?4,
                status          = ?5,
                started_at      = ?6,
                updated_at      = ?7,
                completed_at    = ?8,
                last_message    = ?9,
                last_tool_name  = ?10,
                transcript_path = ?11,
                needs_attention = ?12,
                pid             = ?13",
            params![
                session.session_id,
                Self::serialize_agent_source(&session.source),
                session.cwd,
                session.project_name,
                Self::serialize_agent_status(&session.status),
                session.started_at,
                session.updated_at,
                session.completed_at,
                session.last_message,
                session.last_tool_name,
                session.transcript_path,
                session.needs_attention as i32,
                session.pid,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<AgentSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, source, cwd, project_name, status, started_at,
                    updated_at, completed_at, last_message, last_tool_name,
                    transcript_path, needs_attention, pid
             FROM sessions
             WHERE session_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![session_id], Self::map_session_row)?;

        match rows.next() {
            Some(Ok(session)) => Ok(Some(session)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    // ------------------------------------------------------------------
    // Event CRUD
    // ------------------------------------------------------------------

    pub fn insert_event(&self, event: &AgentEvent) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events
                (id, source, session_id, cwd, project_name, event_type,
                 status, message, tool_name, transcript_path, created_at,
                 process_pid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                event.id,
                Self::serialize_agent_source(&event.source),
                event.session_id,
                event.cwd,
                event.project_name,
                Self::serialize_event_type(&event.event_type),
                Self::serialize_agent_status(&event.status),
                event.message,
                event.tool_name,
                event.transcript_path,
                event.created_at,
                event.process_pid,
            ],
        )?;
        Ok(())
    }

    pub fn get_events_for_session(&self, session_id: &str) -> Result<Vec<AgentEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, session_id, cwd, project_name, event_type,
                    status, message, tool_name, transcript_path, created_at,
                    process_pid
             FROM events
             WHERE session_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let source_str: String = row.get(1)?;
            let event_type_str: String = row.get(5)?;
            let status_str: String = row.get(6)?;
            Ok(AgentEvent {
                id: row.get(0)?,
                source: Self::map_deser(Self::deserialize_agent_source(&source_str))?,
                session_id: row.get(2)?,
                cwd: row.get(3)?,
                project_name: row.get(4)?,
                event_type: Self::map_deser(Self::deserialize_event_type(&event_type_str))?,
                status: Self::map_deser(Self::deserialize_agent_status(&status_str))?,
                message: row.get(7)?,
                tool_name: row.get(8)?,
                transcript_path: row.get(9)?,
                created_at: row.get(10)?,
                process_pid: row.get(11)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    // ------------------------------------------------------------------
    // Session listing
    // ------------------------------------------------------------------

    pub fn list_all_sessions(&self) -> Result<Vec<AgentSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, source, cwd, project_name, status, started_at,
                    updated_at, completed_at, last_message, last_tool_name,
                    transcript_path, needs_attention, pid
             FROM sessions
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_session_row)?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM events WHERE session_id = ?1",
            params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn list_sessions_with_pid(&self) -> Result<Vec<AgentSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, source, cwd, project_name, status, started_at,
                    updated_at, completed_at, last_message, last_tool_name,
                    transcript_path, needs_attention, pid
             FROM sessions
             WHERE pid IS NOT NULL
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_session_row)?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn cleanup_old_sessions(&self, retain_days: u32, now_ms: i64) -> Result<usize> {
        let cutoff = now_ms - (retain_days as i64 * 24 * 60 * 60 * 1000);
        let mut stmt = self.conn.prepare(
            "DELETE FROM events WHERE session_id IN (
                SELECT session_id FROM sessions
                WHERE status IN ('completed', 'failed')
                  AND updated_at < ?
            )",
        )?;
        stmt.execute(params![cutoff])?;

        let mut stmt = self.conn.prepare(
            "DELETE FROM sessions
             WHERE status IN ('completed', 'failed')
               AND updated_at < ?",
        )?;
        let removed = stmt.execute(params![cutoff])?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            pid: None,
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
            pid: None,
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
            process_pid: None,
        };

        db.insert_event(&event).unwrap();
        let events = db.get_events_for_session("sess-002").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "evt-001");
    }

    #[test]
    fn test_event_process_pid_roundtrip() {
        let db = setup_db();
        let session = AgentSession {
            session_id: "sess-pid".into(),
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
            pid: None,
        };
        db.upsert_session(&session).unwrap();

        let event = AgentEvent {
            id: "evt-pid".into(),
            source: AgentSource::ClaudeCode,
            session_id: "sess-pid".into(),
            cwd: "/tmp".into(),
            project_name: Some("tmp".into()),
            event_type: EventType::SessionStart,
            status: AgentStatus::Starting,
            message: None,
            tool_name: None,
            transcript_path: None,
            created_at: 1700000000000,
            process_pid: Some(12345),
        };

        db.insert_event(&event).unwrap();
        let events = db.get_events_for_session("sess-pid").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].process_pid,
            Some(12345),
            "process_pid should survive round-trip"
        );
    }

    #[test]
    fn test_list_all_sessions_includes_completed() {
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
            pid: None,
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
            pid: None,
        };

        db.upsert_session(&running).unwrap();
        db.upsert_session(&completed).unwrap();

        let all = db.list_all_sessions().unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|s| s.session_id == "sess-A"));
        assert!(all.iter().any(|s| s.session_id == "sess-B"));
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = setup_db();
        let session = AgentSession {
            session_id: "sess-003".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/original".into(),
            project_name: "original".into(),
            status: AgentStatus::Starting,
            started_at: 1000,
            updated_at: 1000,
            completed_at: None,
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&session).unwrap();

        let updated = AgentSession {
            session_id: "sess-003".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/updated".into(),
            project_name: "updated".into(),
            status: AgentStatus::Running,
            started_at: 1000,
            updated_at: 2000,
            completed_at: None,
            last_message: Some("progress".into()),
            last_tool_name: None,
            transcript_path: None,
            needs_attention: true,
            pid: None,
        };
        db.upsert_session(&updated).unwrap();

        let got = db.get_session("sess-003").unwrap().unwrap();
        assert_eq!(got.cwd, "/updated");
        assert_eq!(got.project_name, "updated");
        assert_eq!(got.status, AgentStatus::Running);
        assert_eq!(got.last_message, Some("progress".into()));
        assert!(got.needs_attention);
    }

    #[test]
    fn test_get_session_not_found() {
        let db = setup_db();
        let got = db.get_session("nonexistent").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn test_events_ordered_desc() {
        let db = setup_db();
        let session = AgentSession {
            session_id: "sess-004".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/tmp".into(),
            project_name: "tmp".into(),
            status: AgentStatus::Running,
            started_at: 0,
            updated_at: 0,
            completed_at: None,
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&session).unwrap();

        for i in 0..3 {
            let event = AgentEvent {
                id: format!("evt-{i}"),
                source: AgentSource::ClaudeCode,
                session_id: "sess-004".into(),
                cwd: "/tmp".into(),
                project_name: None,
                event_type: EventType::Notification,
                status: AgentStatus::Running,
                message: None,
                tool_name: None,
                transcript_path: None,
                created_at: i,
                process_pid: None,
            };
            db.insert_event(&event).unwrap();
        }

        let events = db.get_events_for_session("sess-004").unwrap();
        assert_eq!(events.len(), 3);
        // newest first (DESC order)
        assert_eq!(events[0].id, "evt-2");
        assert_eq!(events[1].id, "evt-1");
        assert_eq!(events[2].id, "evt-0");
    }

    #[test]
    fn test_deserialize_corrupt_value_returns_error() {
        // Bug 1.2: corrupt DB values must not panic
        assert!(Database::deserialize_agent_source("invalid_source").is_err());
        assert!(Database::deserialize_agent_status("invalid_status").is_err());
        assert!(Database::deserialize_event_type("invalid_event").is_err());
    }

    #[test]
    fn test_deserialize_valid_values_succeed() {
        let source = Database::deserialize_agent_source("claude-code").unwrap();
        assert_eq!(source, AgentSource::ClaudeCode);

        let status = Database::deserialize_agent_status("running").unwrap();
        assert_eq!(status, AgentStatus::Running);

        let event_type = Database::deserialize_event_type("notification").unwrap();
        assert_eq!(event_type, EventType::Notification);
    }

    #[test]
    fn test_agent_source_new_variants_roundtrip() {
        let cases = vec![
            ("codex", AgentSource::Codex),
            ("gemini", AgentSource::Gemini),
            ("copilot", AgentSource::Copilot),
        ];
        for (str_val, variant) in cases {
            let deser = Database::deserialize_agent_source(str_val).unwrap();
            assert_eq!(deser, variant, "deserialize {str_val}");

            let ser = Database::serialize_agent_source(&variant);
            assert_eq!(ser, str_val, "serialize {str_val}");
        }
    }

    #[test]
    fn test_list_all_sessions_includes_failed() {
        let db = setup_db();
        let failed = AgentSession {
            session_id: "sess-F".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/f".into(),
            project_name: "f".into(),
            status: AgentStatus::Failed,
            started_at: 0,
            updated_at: 0,
            completed_at: Some(0),
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&failed).unwrap();

        let all = db.list_all_sessions().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].session_id, "sess-F");
    }

    #[test]
    fn test_file_based_db_persists_across_reopen() {
        let dir = std::env::temp_dir().join("agentpulse_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_persist.db");

        // Clean up any previous test run
        let _ = std::fs::remove_file(&path);

        // Create, write, drop
        {
            let db = Database::new(&path).unwrap();
            let session = AgentSession {
                session_id: "persist-1".into(),
                source: AgentSource::ClaudeCode,
                cwd: "/tmp".into(),
                project_name: "test".into(),
                status: AgentStatus::Running,
                started_at: 1,
                updated_at: 1,
                completed_at: None,
                last_message: None,
                last_tool_name: None,
                transcript_path: None,
                needs_attention: false,
                pid: None,
            };
            db.upsert_session(&session).unwrap();
        }

        // Reopen and verify data survived
        {
            let db = Database::new(&path).unwrap();
            let sessions = db.list_all_sessions().unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].session_id, "persist-1");
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cleanup_old_sessions_removes_old_completed() {
        let db = setup_db();

        let now_ms = 1700000000000i64;
        let seven_days_ms = 7 * 24 * 60 * 60 * 1000i64;

        // Old completed session (older than 7 days)
        let old = AgentSession {
            session_id: "old-done".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/old".into(),
            project_name: "old".into(),
            status: AgentStatus::Completed,
            started_at: now_ms - seven_days_ms - 1,
            updated_at: now_ms - seven_days_ms - 1,
            completed_at: Some(now_ms - seven_days_ms - 1),
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&old).unwrap();

        // Recent completed session (within 7 days)
        let recent = AgentSession {
            session_id: "recent-done".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/recent".into(),
            project_name: "recent".into(),
            status: AgentStatus::Completed,
            started_at: now_ms - 1000,
            updated_at: now_ms - 1000,
            completed_at: Some(now_ms - 1000),
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&recent).unwrap();

        // Active session (should never be cleaned)
        let active = AgentSession {
            session_id: "active-sess".into(),
            source: AgentSource::ClaudeCode,
            cwd: "/active".into(),
            project_name: "active".into(),
            status: AgentStatus::Running,
            started_at: now_ms - seven_days_ms - 1000,
            updated_at: now_ms,
            completed_at: None,
            last_message: None,
            last_tool_name: None,
            transcript_path: None,
            needs_attention: false,
            pid: None,
        };
        db.upsert_session(&active).unwrap();

        let removed = db.cleanup_old_sessions(7, now_ms).unwrap();
        assert!(
            removed >= 1,
            "should have removed at least the old completed session"
        );

        let remaining = db.list_all_sessions().unwrap();
        let ids: Vec<&str> = remaining.iter().map(|s| s.session_id.as_str()).collect();

        assert!(
            !ids.contains(&"old-done"),
            "old completed session should be removed"
        );
        assert!(
            ids.contains(&"recent-done"),
            "recent completed session should remain"
        );
        assert!(ids.contains(&"active-sess"), "active session should remain");
    }
}
