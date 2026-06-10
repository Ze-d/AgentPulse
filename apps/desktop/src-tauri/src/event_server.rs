use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use uuid::Uuid;

use crate::db::Database;
use crate::state_machine::StateMachine;
use crate::{AgentEvent, AgentSession, AgentSource, AgentStatus, EventType};

/// Normalize a raw Claude Code hook JSON event into an `AgentEvent`.
///
/// Extracts `hook_event_name`, `session_id`, `cwd`, `transcript_path`,
/// `notification_type`, `message`, `last_assistant_message`, and `tool_name`
/// from the raw JSON and maps them to the corresponding `EventType` and
/// `AgentStatus`. Derives `project_name` from the basename of `cwd`.
pub fn normalize_claude_code_event(raw: &serde_json::Value) -> AgentEvent {
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
    let notification_type = raw["notification_type"].as_str().unwrap_or("");
    let process_pid = raw["process_pid"].as_u64().map(|v| v as u32);

    // Derive project name from the last path component of cwd.
    let project_name = if cwd.is_empty() {
        None
    } else {
        Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    };

    let (event_type, status) = match hook_event_name {
        "SessionStart" => (EventType::SessionStart, AgentStatus::Starting),
        "PreToolUse" => (EventType::PreToolUse, AgentStatus::ToolRunning),
        "PostToolUse" => (EventType::PostToolUse, AgentStatus::Running),
        "PostToolUseFailure" => (EventType::Failure, AgentStatus::Failed),
        "Stop" | "SubagentStop" => (EventType::Stop, AgentStatus::Completed),
        "Notification" => match notification_type {
            "permission_prompt" => (EventType::PermissionRequest, AgentStatus::WaitingPermission),
            "idle_prompt" => (EventType::Notification, AgentStatus::WaitingInput),
            _ => (EventType::Notification, AgentStatus::Running),
        },
        "UserPromptSubmit" => (EventType::Notification, AgentStatus::Running),
        _ => (EventType::Notification, AgentStatus::Running),
    };

    AgentEvent {
        id: Uuid::new_v4().to_string(),
        source: AgentSource::ClaudeCode,
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

/// HTTP server that receives Claude Code hook events, normalizes them,
/// applies the state machine, and persists the results via the `Database`.
pub struct EventServer {
    db: Arc<Mutex<Database>>,
    shutdown: Arc<AtomicBool>,
}

impl EventServer {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create an EventServer from an already-shared database reference.
    pub fn from_arc(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the server thread to stop accepting new requests.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Return a clone of the shutdown flag for external signaling.
    pub fn shutdown_signal(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Normalize the raw JSON event, apply the state-machine transition,
    /// upsert the session, insert the event, and return both.
    pub fn handle_event(
        &self,
        raw: &serde_json::Value,
    ) -> Result<(AgentEvent, AgentSession), String> {
        let event = normalize_claude_code_event(raw);

        let db = self.db.lock().map_err(|e| format!("lock error: {}", e))?;
        let machine = StateMachine::new();
        let now = Utc::now().timestamp_millis();

        // Look up an existing session or create a fresh one.
        let existing = db
            .get_session(&event.session_id)
            .map_err(|e| format!("db error: {}", e))?;

        let session = match existing {
            Some(old) => {
                let new_status = machine.transition(old.status.clone(), &event.event_type);
                let completed_at =
                    if matches!(new_status, AgentStatus::Completed | AgentStatus::Failed) {
                        Some(now)
                    } else {
                        old.completed_at
                    };

                AgentSession {
                    session_id: old.session_id,
                    source: old.source,
                    cwd: event.cwd.clone(),
                    project_name: old.project_name,
                    status: new_status.clone(),
                    started_at: old.started_at,
                    updated_at: now,
                    completed_at,
                    last_message: event.message.clone().or(old.last_message),
                    last_tool_name: event.tool_name.clone().or(old.last_tool_name),
                    transcript_path: event.transcript_path.clone().or(old.transcript_path),
                    needs_attention: StateMachine::needs_attention(&new_status),
                    // Prefer the new PID; fall back to the old one.
                    // If old.pid was None but a new PID arrives (e.g. from a
                    // later event), backfill it so process checker can work.
                    pid: event.process_pid.or(old.pid),
                }
            }
            None => {
                let project_name = event
                    .project_name
                    .clone()
                    .unwrap_or_else(|| "unknown".into());
                AgentSession {
                    session_id: event.session_id.clone(),
                    source: AgentSource::ClaudeCode,
                    cwd: event.cwd.clone(),
                    project_name,
                    status: event.status.clone(),
                    started_at: now,
                    updated_at: now,
                    completed_at: None,
                    last_message: event.message.clone(),
                    last_tool_name: event.tool_name.clone(),
                    transcript_path: event.transcript_path.clone(),
                    needs_attention: StateMachine::needs_attention(&event.status),
                    pid: event.process_pid,
                }
            }
        };

        db.upsert_session(&session)
            .map_err(|e| format!("db error: {}", e))?;
        db.insert_event(&event)
            .map_err(|e| format!("db error: {}", e))?;

        Ok((event, session))
    }

    /// Create a `Database`, wrap it in an `EventServer`, and spawn a
    /// `tiny_http` server on `addr` that handles three routes:
    ///
    /// - `POST /api/events`  -- accept event JSON, return 201
    /// - `GET  /api/sessions` -- return active sessions as JSON
    /// - `GET  /api/health`  -- return `{"status":"ok"}`
    ///
    /// Returns an `Arc<AtomicBool>` that can be used to signal graceful
    /// shutdown of the server thread.
    pub fn start(db: Database, addr: &str) -> Result<Arc<AtomicBool>, String> {
        let server =
            tiny_http::Server::http(addr).map_err(|e| format!("failed to start server: {}", e))?;
        let event_server = Self::new(db);
        let shutdown = event_server.shutdown_signal();
        Self::run_server_loop(server, event_server);
        Ok(shutdown)
    }

    /// Start the event server with a shared database reference.
    ///
    /// Same as `start` but accepts an `Arc<Mutex<Database>>` directly,
    /// allowing the caller to share the same database instance with other
    /// components (e.g. Tauri state).
    ///
    /// Returns an `Arc<AtomicBool>` that can be used to signal graceful
    /// shutdown of the server thread.
    pub fn start_shared(db: Arc<Mutex<Database>>, addr: &str) -> Result<Arc<AtomicBool>, String> {
        let server =
            tiny_http::Server::http(addr).map_err(|e| format!("failed to start server: {}", e))?;
        let event_server = Self::from_arc(db);
        let shutdown = event_server.shutdown_signal();
        Self::run_server_loop(server, event_server);
        Ok(shutdown)
    }

    /// Internal: spawn a thread that runs the HTTP request loop.
    fn run_server_loop(server: tiny_http::Server, event_server: EventServer) {
        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                // Bug 1.6: check shutdown flag to allow graceful stop
                if event_server.shutdown.load(Ordering::Relaxed) {
                    tracing::info!("event_server: shutdown signaled, stopping accept loop");
                    break;
                }

                let url = request.url().to_string();
                let method = format!("{}", request.method());

                let response = match (method.as_str(), url.as_str()) {
                    ("POST", "/api/events") => {
                        let mut body = String::new();
                        let body_ok = request.as_reader().read_to_string(&mut body).is_ok();

                        if !body_ok {
                            tracing::warn!("event_server: failed to read request body");
                            json_response(400, &serde_json::json!({"error": "failed to read body"}))
                        } else {
                            match serde_json::from_str::<serde_json::Value>(&body) {
                                Ok(json) => match event_server.handle_event(&json) {
                                    Ok((event, session)) => {
                                        tracing::info!(
                                            session_id = %event.session_id,
                                            event_type = ?event.event_type,
                                            status = ?session.status,
                                            "event processed"
                                        );
                                        json_response(
                                            201,
                                            &serde_json::json!({"event": event, "session": session}),
                                        )
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            error = %e,
                                            "event_server: handle_event failed"
                                        );
                                        json_response(500, &serde_json::json!({"error": e}))
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        body_len = body.len(),
                                        "event_server: invalid JSON body"
                                    );
                                    json_response(
                                        400,
                                        &serde_json::json!({"error": format!("invalid JSON: {}", e)}),
                                    )
                                }
                            }
                        }
                    }
                    ("GET", "/api/sessions") => match event_server.db.lock() {
                        Ok(db) => match db.list_all_sessions() {
                            Ok(sessions) => {
                                tracing::debug!(count = sessions.len(), "GET /api/sessions");
                                json_response(200, &serde_json::json!(sessions))
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    "event_server: db list_sessions failed"
                                );
                                json_response(
                                    500,
                                    &serde_json::json!({"error": format!("db error: {}", e)}),
                                )
                            }
                        },
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "event_server: db lock poisoned on GET /api/sessions"
                            );
                            json_response(
                                500,
                                &serde_json::json!({"error": "internal server error"}),
                            )
                        }
                    },
                    ("GET", "/api/health") => {
                        json_response(200, &serde_json::json!({"status": "ok"}))
                    }
                    _ => {
                        tracing::debug!(
                            method = %method,
                            url = %url,
                            "event_server: unknown route"
                        );
                        json_response(404, &serde_json::json!({"error": "not found"}))
                    }
                };

                let _ = request.respond(response);
            }
        });
    }
}

/// Build a JSON HTTP response with the given status code.
fn json_response(
    status_code: u16,
    data: &serde_json::Value,
) -> tiny_http::Response<Box<dyn Read + Send>> {
    let body = serde_json::to_string(data).unwrap_or_default();
    tiny_http::Response::from_string(body)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_status_code(tiny_http::StatusCode(status_code))
        .boxed()
}
