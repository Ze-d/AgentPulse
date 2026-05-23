use serde::{Deserialize, Serialize};

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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
