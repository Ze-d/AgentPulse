use crate::config::AgentPulseConfig;
use crate::db::Database;
use crate::hooks;
use crate::AgentSession;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::State;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: AgentPulseConfig,
}

/// Subset of config exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendConfig {
    pub poll_interval_ms: u64,
}

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<AgentSession>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let sessions = db.list_all_sessions().map_err(|e| e.to_string())?;
    tracing::debug!(count = sessions.len(), "get_sessions");
    Ok(sessions)
}

#[tauri::command]
pub fn get_session_detail(
    state: State<AppState>,
    session_id: String,
) -> Result<Option<AgentSession>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let session = db.get_session(&session_id).map_err(|e| e.to_string())?;
    tracing::debug!(
        session_id = %session_id,
        found = session.is_some(),
        "get_session_detail"
    );
    Ok(session)
}

#[tauri::command]
pub fn get_session_events(
    state: State<AppState>,
    session_id: String,
) -> Result<Vec<crate::AgentEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let events = db
        .get_events_for_session(&session_id)
        .map_err(|e| e.to_string())?;
    tracing::debug!(
        session_id = %session_id,
        count = events.len(),
        "get_session_events"
    );
    Ok(events)
}

#[tauri::command]
pub fn get_hook_status_cmd(app_handle: tauri::AppHandle) -> Result<HashMap<String, bool>, String> {
    tracing::debug!("get_hook_status_cmd");
    let settings_path = app_handle
        .path()
        .resolve(".claude/settings.json", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::get_hook_status(&settings_path)
}

#[tauri::command]
pub fn install_hooks_cmd(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("user triggered hook installation");
    let settings_path = app_handle
        .path()
        .resolve(".claude/settings.json", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let hook_path = hooks::extract_hook_binary(&resource_dir, &app_data_dir)?;
    hooks::ensure_hooks_installed(&settings_path, &hook_path.to_string_lossy())
}

#[tauri::command]
pub fn uninstall_hooks_cmd(app_handle: tauri::AppHandle) -> Result<String, String> {
    tracing::info!("user triggered hook removal");
    let settings_path = app_handle
        .path()
        .resolve(".claude/settings.json", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::unregister_hooks(&settings_path)
}

#[tauri::command]
pub fn get_codex_hook_status_cmd(app_handle: tauri::AppHandle) -> Result<HashMap<String, bool>, String> {
    tracing::debug!("get_codex_hook_status_cmd");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::get_codex_hook_status(&config_path)
}

#[tauri::command]
pub fn install_codex_hooks_cmd(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("user triggered codex hook installation");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let hook_path = hooks::extract_hook_binary(&resource_dir, &app_data_dir)?;
    hooks::ensure_codex_hooks_installed(&config_path, &hook_path.to_string_lossy())
}

#[tauri::command]
pub fn uninstall_codex_hooks_cmd(app_handle: tauri::AppHandle) -> Result<String, String> {
    tracing::info!("user triggered codex hook removal");
    let config_path = app_handle
        .path()
        .resolve(".codex/config.toml", tauri::path::BaseDirectory::Home)
        .map_err(|e| e.to_string())?;
    hooks::unregister_codex_hooks(&config_path)
}

#[tauri::command]
pub fn hide_main_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    tracing::trace!("hide_main_window called");
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window.hide().map_err(|e| e.to_string())
}

/// Receive log events from the frontend and route them through the tracing
/// subscriber for persistent storage (file) and console output.
///
/// The `module` parameter is included as a structured field so log entries can
/// be filtered by frontend component. The `target` is always `"frontend"` since
/// tracing requires a compile-time constant for that field.
#[tauri::command]
pub fn log_event(level: String, module: String, message: String, details: Option<String>) {
    match level.to_lowercase().as_str() {
        "error" => {
            if let Some(ref d) = details {
                tracing::error!(module = %module, details = %d, "{message}");
            } else {
                tracing::error!(module = %module, "{message}");
            }
        }
        "warn" => {
            if let Some(ref d) = details {
                tracing::warn!(module = %module, details = %d, "{message}");
            } else {
                tracing::warn!(module = %module, "{message}");
            }
        }
        "info" => tracing::info!(module = %module, "{message}"),
        "debug" => tracing::debug!(module = %module, "{message}"),
        "trace" => tracing::trace!(module = %module, "{message}"),
        _ => tracing::info!(module = %module, raw_level = %level, "{message}"),
    }
}

#[tauri::command]
pub fn delete_session(state: State<AppState>, session_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_session(&session_id).map_err(|e| e.to_string())?;
    tracing::info!(session_id = %session_id, "delete_session: user dismissed");
    Ok(())
}

/// Return the subset of configuration values that the frontend needs.
#[tauri::command]
pub fn get_config(state: State<AppState>) -> FrontendConfig {
    FrontendConfig {
        poll_interval_ms: state.config.poll_interval_ms,
    }
}
