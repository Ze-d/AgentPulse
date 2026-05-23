use crate::db::Database;
use crate::AgentSession;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<AgentSession>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_active_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_detail(
    state: State<AppState>,
    session_id: String,
) -> Result<Option<AgentSession>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_session(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_events(
    state: State<AppState>,
    session_id: String,
) -> Result<Vec<crate::AgentEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_events_for_session(&session_id).map_err(|e| e.to_string())
}
