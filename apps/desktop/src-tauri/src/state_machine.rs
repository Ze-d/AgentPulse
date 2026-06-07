use crate::*;

pub struct StateMachine;

impl Default for StateMachine {
    fn default() -> Self {
        Self
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self
    }

    pub fn transition(&self, current: AgentStatus, event_type: &EventType) -> AgentStatus {
        let from = current.clone();
        let next = match (current, event_type) {
            // Session start
            (_, EventType::SessionStart) => AgentStatus::Starting,

            // Terminal events take priority over the Starting catch-all
            (_, EventType::Stop) => AgentStatus::Completed,
            (_, EventType::Failure) => AgentStatus::Failed,

            // Any other event from Starting transitions to Running
            (AgentStatus::Starting, _) => AgentStatus::Running,

            // Tool execution
            (AgentStatus::Running, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::WaitingInput, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::WaitingPermission, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::Completed, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::Failed, EventType::PreToolUse) => AgentStatus::ToolRunning,
            (AgentStatus::ToolRunning, EventType::PostToolUse) => AgentStatus::Running,

            // Permission & input
            (AgentStatus::Running, EventType::PermissionRequest) => AgentStatus::WaitingPermission,
            (_, EventType::Notification) => AgentStatus::WaitingInput,

            // Wake terminal states on any other new activity
            (AgentStatus::Completed, _) => AgentStatus::Running,
            (AgentStatus::Failed, _) => AgentStatus::Running,

            // Default: keep current status
            _ => from.clone(),
        };
        tracing::trace!(
            from = ?from,
            event = ?event_type,
            to = ?next,
            "state transition"
        );
        next
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
