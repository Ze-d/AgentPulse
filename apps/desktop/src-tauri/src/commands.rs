use crate::db::Database;
use crate::hooks;
use crate::AgentSession;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::State;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
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
pub fn install_hooks_cmd(app_handle: tauri::AppHandle) -> Result<String, String> {
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
    let monitor_path = hooks::extract_monitor_script(&resource_dir, &app_data_dir)?;
    hooks::ensure_hooks_installed(&settings_path, &monitor_path.to_string_lossy())
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
pub fn hide_main_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    tracing::trace!("hide_main_window called");
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window.hide().map_err(|e| e.to_string())
}

/// Receive log events from the frontend and route them through the tracing
/// subscriber for persistent storage (file) and console output.
#[tauri::command]
pub fn log_event(level: String, module: String, message: String, details: Option<String>) {
    match level.to_lowercase().as_str() {
        "error" => {
            if let Some(ref d) = details {
                tracing::error!(target: &module, details = %d, "{message}");
            } else {
                tracing::error!(target: &module, "{message}");
            }
        }
        "warn" => {
            if let Some(ref d) = details {
                tracing::warn!(target: &module, details = %d, "{message}");
            } else {
                tracing::warn!(target: &module, "{message}");
            }
        }
        "info" => tracing::info!(target: &module, "{message}"),
        "debug" => tracing::debug!(target: &module, "{message}"),
        "trace" => tracing::trace!(target: &module, "{message}"),
        _ => tracing::info!(target: &module, level = %level, "{message}"),
    }
}
