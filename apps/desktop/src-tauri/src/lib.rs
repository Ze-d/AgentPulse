pub mod commands;
pub mod db;
pub mod event_server;
pub mod hooks;
pub mod state_machine;
pub mod tray;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::Manager;
use db::Database;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AgentSource {
    #[serde(rename = "claude-code")]
    ClaudeCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Starting,
    Running,
    #[serde(rename = "tool_running")]
    ToolRunning,
    #[serde(rename = "waiting_input")]
    WaitingInput,
    #[serde(rename = "waiting_permission")]
    WaitingPermission,
    Completed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    #[serde(rename = "session_start")]
    SessionStart,
    #[serde(rename = "pre_tool_use")]
    PreToolUse,
    #[serde(rename = "post_tool_use")]
    PostToolUse,
    #[serde(rename = "permission_request")]
    PermissionRequest,
    Notification,
    Stop,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub id: String,
    pub source: AgentSource,
    pub session_id: String,
    pub cwd: String,
    pub project_name: Option<String>,
    pub event_type: EventType,
    pub status: AgentStatus,
    pub message: Option<String>,
    pub tool_name: Option<String>,
    pub transcript_path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub session_id: String,
    pub source: AgentSource,
    pub cwd: String,
    pub project_name: String,
    pub status: AgentStatus,
    pub started_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub last_message: Option<String>,
    pub last_tool_name: Option<String>,
    pub transcript_path: Option<String>,
    pub needs_attention: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let database = Database::new_in_memory().expect("Failed to initialize database");
    let db = Arc::new(Mutex::new(database));

    let db_for_server = db.clone();
    std::thread::spawn(move || {
        let _ = event_server::EventServer::start_shared(db_for_server, "127.0.0.1:17878");
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(commands::AppState { db: db.clone() })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::get_session_detail,
            commands::get_session_events,
            commands::get_hook_status_cmd,
            commands::install_hooks_cmd,
            commands::uninstall_hooks_cmd,
        ])
        .setup(|app| {
            tray::setup_tray(app)?;

            // Ensure hooks are installed on every launch (idempotent).
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let resource_dir = match app_handle.path().resource_dir() {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("resource_dir: {e}");
                        return;
                    }
                };
                let app_data_dir = match app_handle.path().app_data_dir() {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("app_data_dir: {e}");
                        return;
                    }
                };
                let settings_path = match app_handle
                    .path()
                    .resolve(".claude/settings.json", tauri::path::BaseDirectory::Home)
                {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("resolve settings path: {e}");
                        return;
                    }
                };

                match hooks::extract_monitor_script(&resource_dir, &app_data_dir) {
                    Ok(monitor_path) => {
                        match hooks::ensure_hooks_installed(
                            &settings_path,
                            &monitor_path.to_string_lossy(),
                        ) {
                            Ok(status) => log::info!("AgentPulse hooks: {status}"),
                            Err(e) => log::error!("hooks: {e}"),
                        }
                    }
                    Err(e) => log::error!("extract monitor script: {e}"),
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AgentPulse");
}
