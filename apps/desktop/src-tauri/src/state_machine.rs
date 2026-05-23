use crate::*;

pub struct StateMachine;

impl StateMachine {
    pub fn new() -> Self {
        StateMachine
    }

    pub fn transition(&self, current: AgentStatus, event_type: &EventType) -> AgentStatus {
        match (current.clone(), event_type) {
            // Session start
            (_, EventType::SessionStart) => AgentStatus::Starting,
            (AgentStatus::Starting, _) => AgentStatus::Running,

            // Tool execution
            (AgentStatus::Running, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::WaitingInput, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::WaitingPermission, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::ToolRunning, EventType::PostToolUse) => AgentStatus::Running,

            // Permission & input
            (AgentStatus::Running, EventType::PermissionRequest) => AgentStatus::WaitingPermission,
            (_, EventType::Notification) => AgentStatus::WaitingInput,

            // Terminal events
            (AgentStatus::Running, EventType::Stop) => AgentStatus::Completed,
            (AgentStatus::ToolRunning, EventType::Stop) => AgentStatus::Completed,
            (AgentStatus::WaitingInput, EventType::Stop) => AgentStatus::Completed,
            (AgentStatus::WaitingPermission, EventType::Stop) => AgentStatus::Completed,

            // Failure
            (_, EventType::Failure) => AgentStatus::Failed,

            // Default: keep current status
            _ => current,
        }
    }

    pub fn needs_attention(status: &AgentStatus) -> bool {
        matches!(
            status,
            AgentStatus::WaitingInput
                | AgentStatus::WaitingPermission
                | AgentStatus::Completed
                | AgentStatus::Failed
        )
    }
}
