import { computed } from "vue";
import type { AgentSession } from "../types/agent";
import { STATUS_COLORS, STATUS_LABELS, formatDuration } from "../types/agent";

export function useSessionDisplay(session: AgentSession) {
  const statusColor = computed(() => STATUS_COLORS[session.status]);
  const statusLabel = computed(() => STATUS_LABELS[session.status]);
  const duration = computed(() =>
    formatDuration(session.startedAt, session.completedAt)
  );

  return { statusColor, statusLabel, duration };
}
