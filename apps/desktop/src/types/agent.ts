export type AgentSource = "claude-code";

export type AgentStatus =
  | "starting"
  | "running"
  | "tool_running"
  | "waiting_input"
  | "waiting_permission"
  | "completed"
  | "failed"
  | "unknown";

export type EventType =
  | "session_start"
  | "pre_tool_use"
  | "post_tool_use"
  | "permission_request"
  | "notification"
  | "stop"
  | "failure";

export interface AgentEvent {
  id: string;
  source: AgentSource;
  sessionId: string;
  cwd: string;
  projectName?: string;
  eventType: EventType;
  status: AgentStatus;
  message?: string;
  toolName?: string;
  transcriptPath?: string;
  createdAt: number;
}

export interface AgentSession {
  sessionId: string;
  source: AgentSource;
  cwd: string;
  projectName: string;
  status: AgentStatus;
  startedAt: number;
  updatedAt: number;
  completedAt?: number;
  lastMessage?: string;
  lastToolName?: string;
  transcriptPath?: string;
  needsAttention: boolean;
}

export const STATUS_LABELS: Record<AgentStatus, string> = {
  starting: "Starting",
  running: "Running",
  tool_running: "Tool Running",
  waiting_input: "Waiting Input",
  waiting_permission: "Waiting Permission",
  completed: "Completed",
  failed: "Failed",
  unknown: "Unknown",
};

export const STATUS_COLORS: Record<AgentStatus, string> = {
  starting: "#89b4fa",
  running: "#a6e3a1",
  tool_running: "#f9e2af",
  waiting_input: "#fab387",
  waiting_permission: "#fab387",
  completed: "#89b4fa",
  failed: "#f38ba8",
  unknown: "#6c7086",
};

export function formatDuration(startedAt: number, completedAt?: number): string {
  const end = completedAt ?? Date.now();
  const diffMs = end - startedAt;
  const seconds = Math.floor(diffMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}
