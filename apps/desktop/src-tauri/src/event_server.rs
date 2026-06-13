use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::db::Database;
use crate::state_machine::StateMachine;
use crate::{AgentEvent, AgentSession, AgentSource, AgentStatus, EventType};

// ---------------------------------------------------------------------------
// Common field extraction
// ---------------------------------------------------------------------------

struct CommonFields {
    hook_event_name: String,
    session_id: String,
    cwd: String,
    transcript_path: Option<String>,
    message: Option<String>,
    tool_name: Option<String>,
    process_pid: Option<u32>,
    project_name: Option<String>,
}

fn extract_common_fields(raw: &Value) -> CommonFields {
    let hook_event_name = raw["hook_event_name"].as_str().unwrap_or("").to_string();
    let session_id = raw["session_id"].as_str().unwrap_or("unknown").to_string();
    let cwd = raw["cwd"].as_str().unwrap_or("").to_string();
    let transcript_path = raw["transcript_path"].as_str().map(|s| s.to_string());

    let message = raw["message"]
        .as_str()
        .or_else(|| raw["last_assistant_message"].as_str())
        .map(|s| s.to_string());

    let tool_name = raw["tool_name"].as_str().map(|s| s.to_string());
    let process_pid = raw["process_pid"].as_u64().map(|v| v as u32);

    let project_name = if cwd.is_empty() {
        None
    } else {
        std::path::Path::new(&cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    };

    CommonFields {
        hook_event_name,
        session_id,
        cwd,
        transcript_path,
        message,
        tool_name,
        process_pid,
        project_name,
    }
}

// ---------------------------------------------------------------------------
// Event mapping strategies
// ---------------------------------------------------------------------------

type EventMapping = fn(&str, &Value) -> (EventType, AgentStatus);

fn cc_event_mapping(hook_event_name: &str, raw: &Value) -> (EventType, AgentStatus) {
    match hook_event_name {
        "SessionStart" => (EventType::SessionStart, AgentStatus::Starting),
        "PreToolUse" => (EventType::PreToolUse, AgentStatus::ToolRunning),
        "PostToolUse" => (EventType::PostToolUse, AgentStatus::Running),
        "PostToolUseFailure" => (EventType::Failure, AgentStatus::Failed),
        "Stop" | "SubagentStop" => (EventType::Stop, AgentStatus::Completed),
        "Notification" => {
            let notification_type = raw["notification_type"].as_str().unwrap_or("");
            match notification_type {
                "permission_prompt" => {
                    (EventType::PermissionRequest, AgentStatus::WaitingPermission)
                }
                "idle_prompt" => (EventType::Notification, AgentStatus::WaitingInput),
                _ => (EventType::Notification, AgentStatus::Running),
            }
        }
        "UserPromptSubmit" => (EventType::Notification, AgentStatus::Running),
        _ => (EventType::Notification, AgentStatus::Running),
    }
}

fn codex_event_mapping(hook_event_name: &str, _raw: &Value) -> (EventType, AgentStatus) {
    match hook_event_name {
        "SessionStart" => (EventType::SessionStart, AgentStatus::Starting),
        "PreToolUse" => (EventType::PreToolUse, AgentStatus::ToolRunning),
        "PostToolUse" => (EventType::PostToolUse, AgentStatus::Running),
        "PermissionRequest" => (EventType::PermissionRequest, AgentStatus::WaitingPermission),
        "Stop" | "SubagentStop" => (EventType::Stop, AgentStatus::Completed),
        "UserPromptSubmit" => (EventType::Notification, AgentStatus::Running),
        _ => (EventType::Notification, AgentStatus::Running),
    }
}

// ---------------------------------------------------------------------------
// Shared normalization
// ---------------------------------------------------------------------------

fn normalize_event_inner(raw: &Value, source: AgentSource, mapping: EventMapping) -> AgentEvent {
    let fields = extract_common_fields(raw);

    let (event_type, status) = mapping(&fields.hook_event_name, raw);

    AgentEvent {
        id: Uuid::new_v4().to_string(),
        source,
        session_id: fields.session_id,
        cwd: fields.cwd,
        project_name: fields.project_name,
        event_type,
        status,
        message: fields.message,
        tool_name: fields.tool_name,
        transcript_path: fields.transcript_path,
        created_at: Utc::now().timestamp_millis(),
        process_pid: fields.process_pid,
    }
}

// ---------------------------------------------------------------------------
// Public normalizers
// ---------------------------------------------------------------------------

pub fn normalize_claude_code_event(raw: &Value) -> AgentEvent {
    normalize_event_inner(raw, AgentSource::ClaudeCode, cc_event_mapping)
}

pub fn normalize_codex_event(raw: &Value) -> AgentEvent {
    normalize_event_inner(raw, AgentSource::Codex, codex_event_mapping)
}

pub fn normalize_event_by_source(raw: &Value) -> AgentEvent {
    let agent_source = raw["agent_source"].as_str().unwrap_or("(none)");
    tracing::debug!(agent_source, "normalize_event_by_source dispatching");
    match raw["agent_source"].as_str() {
        Some("codex") => normalize_codex_event(raw),
        _ => normalize_claude_code_event(raw),
    }
}

// ---------------------------------------------------------------------------
// Event server
// ---------------------------------------------------------------------------

/// Event server that normalizes hook events, applies the state machine, and
/// persists results via the `Database`.  The HTTP layer is backed by axum.
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

    pub fn from_arc(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    pub fn shutdown_signal(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Normalize the raw JSON event, apply the state-machine transition,
    /// upsert the session, insert the event, and return both.
    pub fn handle_event(
        &self,
        raw: &Value,
    ) -> Result<(AgentEvent, AgentSession), String> {
        let event = normalize_event_by_source(raw);

        let db = self.db.lock().map_err(|e| format!("lock error: {}", e))?;
        let machine = StateMachine::new();
        let now = Utc::now().timestamp_millis();

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
                    source: event.source.clone(),
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
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

type DbState = Arc<Mutex<Database>>;

async fn handle_post_events(
    State(db): State<DbState>,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let server = EventServer::from_arc(db);
    match server.handle_event(&payload) {
        Ok((event, session)) => {
            tracing::info!(
                session_id = %event.session_id,
                source = ?event.source,
                event_type = ?event.event_type,
                status = ?session.status,
                "event processed"
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"event": event, "session": session})),
            )
        }
        Err(e) => {
            tracing::error!(error = %e, "event_server: handle_event failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e})),
            )
        }
    }
}

async fn handle_get_sessions(
    State(db): State<DbState>,
) -> (StatusCode, Json<Value>) {
    match db.lock() {
        Ok(d) => match d.list_all_sessions() {
            Ok(sessions) => {
                tracing::debug!(count = sessions.len(), "GET /api/sessions");
                (StatusCode::OK, Json(serde_json::json!(sessions)))
            }
            Err(e) => {
                tracing::error!(error = %e, "event_server: db list_sessions failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("db error: {}", e)})),
                )
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "event_server: db lock poisoned");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
        }
    }
}

async fn handle_get_health() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

// ---------------------------------------------------------------------------
// Server startup
// ---------------------------------------------------------------------------

/// Build the axum router with all 3 routes.
fn build_router(state: DbState) -> Router {
    Router::new()
        .route("/api/events", post(handle_post_events))
        .route("/api/sessions", get(handle_get_sessions))
        .route("/api/health", get(handle_get_health))
        .with_state(state)
}

/// Start the axum HTTP server on a background thread with its own tokio
/// runtime, and return a shutdown signal.  Call `shutdown.store(true)` to
/// gracefully stop the server.
///
/// Replaces the previous `EventServer::start_shared` (tiny_http-based).
pub fn serve(
    db: Arc<Mutex<Database>>,
    addr: SocketAddr,
) -> Result<Arc<AtomicBool>, String> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_signal = shutdown.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime for event server");

        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|e| format!("bind {}: {}", addr, e))?;

            tracing::info!(addr = %addr, "event server listening (axum)");

            let app = build_router(db);

            // Build a graceful-shutdown future from the AtomicBool.
            let shutdown_signal = {
                let flag = shutdown_for_signal;
                async move {
                    loop {
                        if flag.load(Ordering::Relaxed) {
                            tracing::info!("event_server: shutdown signaled");
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            };

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal)
                .await
                .map_err(|e| format!("server error: {}", e))?;

            Ok::<_, String>(())
        })
        .expect("event server exited with error");
    });

    Ok(shutdown)
}
