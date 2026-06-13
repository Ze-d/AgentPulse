pub mod commands;
pub mod config;
pub mod db;
pub mod event_server;
pub mod hooks;
pub mod logging;
pub mod process_checker;
pub mod state_machine;
pub mod tray;

use config::AgentPulseConfig;
use db::Database;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AgentSource {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "copilot")]
    Copilot,
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
    pub process_pid: Option<u32>,
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
    pub pid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClosePreference {
    action: String,
}

fn read_close_preference(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<ClosePreference>(&s).ok())
        .map(|p| p.action)
}

fn write_close_preference(path: &std::path::Path, action: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let pref = ClosePreference {
        action: action.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&pref) {
        let _ = std::fs::write(path, json);
    }
}

/// Extract the hook binary and install hooks for a single agent type.
///
/// Shared by both Claude Code (JSON settings) and Codex (TOML config) hook
/// installation — the logic is identical except for the config path and the
/// install function.
fn auto_install_hooks(
    app_handle: &tauri::AppHandle,
    config_relative_path: &str,
    label: &str,
    install_fn: fn(&std::path::Path, &str) -> Result<String, String>,
) {
    let resource_dir = match app_handle.path().resource_dir() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, label, "failed to get resource_dir");
            return;
        }
    };
    let app_data_dir = match app_handle.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, label, "failed to get app_data_dir");
            return;
        }
    };
    let config_path = match app_handle
        .path()
        .resolve(config_relative_path, tauri::path::BaseDirectory::Home)
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, label, "failed to resolve config path");
            return;
        }
    };

    match hooks::extract_hook_binary(&resource_dir, &app_data_dir) {
        Ok(hook_path) => {
            match install_fn(&config_path, &hook_path.to_string_lossy()) {
                Ok(status) => tracing::info!(status = %status, "{label} hooks"),
                Err(e) => tracing::error!(error = %e, label, "failed to ensure hooks"),
            }
        }
        Err(e) => tracing::error!(error = %e, label, "failed to extract hook binary"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_data_dir = logging::default_app_data_dir();
    let log_dir = app_data_dir.join("logs");
    let _log_guard = logging::init(Some(&log_dir));

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "AgentPulse starting");

    let config = AgentPulseConfig::load(&app_data_dir);
    tracing::info!(
        port = config.port,
        check_interval_secs = config.check_interval_secs,
        poll_interval_ms = config.poll_interval_ms,
        "configuration loaded"
    );

    let database = Database::new_in_memory().expect("Failed to initialize database");
    tracing::debug!("database initialized in-memory");
    let db = Arc::new(Mutex::new(database));

    let db_for_server = db.clone();
    let addr: SocketAddr = format!("127.0.0.1:{}", config.port)
        .parse()
        .expect("Invalid event server address");
    let shutdown = event_server::serve(db_for_server, addr)
        .expect("Failed to start event server");
    tracing::debug!(port = config.port, "event server spawned (axum)");

    process_checker::start(db.clone(), config.check_interval_secs);
    tracing::debug!(
        interval_secs = config.check_interval_secs,
        "process checker thread spawned"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState {
            db: db.clone(),
            config: config.clone(),
            shutdown: shutdown.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::get_session_detail,
            commands::get_session_events,
            commands::delete_session,
            commands::get_hook_status_cmd,
            commands::install_hooks_cmd,
            commands::uninstall_hooks_cmd,
            commands::get_codex_hook_status_cmd,
            commands::install_codex_hooks_cmd,
            commands::uninstall_codex_hooks_cmd,
            commands::hide_main_window,
            commands::log_event,
            commands::get_config,
        ])
        .setup(move |app| {
            tray::setup_tray(app)?;

            // Intercept window close: minimize to tray with remembered preference
            let window = app.get_webview_window("main").unwrap();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();

                    let pref_path = match app_handle.path().app_data_dir() {
                        Ok(dir) => Some(dir.join("close_action.json")),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to get app_data_dir for close preference");
                            None
                        }
                    };

                    let action = pref_path.as_ref().and_then(|p| read_close_preference(p));

                    match action.as_deref() {
                        Some("tray") => {
                            if let Some(w) = app_handle.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                        Some("quit") => {
                            if let Some(state) = app_handle.try_state::<commands::AppState>() {
                                state.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                            app_handle.exit(0);
                        }
                        _ => {
                            let minimize = app_handle
                                .dialog()
                                .message("Minimize to system tray?")
                                .title("AgentPulse")
                                .kind(MessageDialogKind::Info)
                                .buttons(MessageDialogButtons::YesNo)
                                .blocking_show();

                            if minimize {
                                if let Some(w) = app_handle.get_webview_window("main") {
                                    let _ = w.hide();
                                }

                                let remember = app_handle
                                    .dialog()
                                    .message("Always minimize to tray when closing?")
                                    .title("AgentPulse")
                                    .kind(MessageDialogKind::Info)
                                    .buttons(MessageDialogButtons::YesNo)
                                    .blocking_show();

                                if remember {
                                    if let Some(ref p) = pref_path {
                                        write_close_preference(p, "tray");
                                    }
                                }
                            } else {
                                let remember = app_handle
                                    .dialog()
                                    .message("Always quit when closing?")
                                    .title("AgentPulse")
                                    .kind(MessageDialogKind::Info)
                                    .buttons(MessageDialogButtons::YesNo)
                                    .blocking_show();

                                if remember {
                                    if let Some(ref p) = pref_path {
                                        write_close_preference(p, "quit");
                                    }
                                }

                                if let Some(state) = app_handle.try_state::<commands::AppState>() {
                                    state.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                                }
                                app_handle.exit(0);
                            }
                        }
                    }
                }
            });

            // Ensure hooks are installed on every launch (idempotent).
            // Runs in a background thread to avoid blocking startup.
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                auto_install_hooks(
                    &app_handle,
                    ".claude/settings.json",
                    "AgentPulse",
                    hooks::ensure_hooks_installed,
                );
                auto_install_hooks(
                    &app_handle,
                    ".codex/config.toml",
                    "Codex AgentPulse",
                    hooks::ensure_codex_hooks_installed,
                );
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AgentPulse");
}
