import { computed, type Ref, type ComputedRef } from "vue";
import type { AgentSession } from "../types/agent";
import { STATUS_COLORS, STATUS_LABELS, formatDuration } from "../types/agent";

export function useSessionDisplay(session: Ref<AgentSession> | ComputedRef<AgentSession>) {
  const statusColor = computed(() => STATUS_COLORS[session.value.status]);
  const statusLabel = computed(() => STATUS_LABELS[session.value.status]);
  const duration = computed(() =>
    formatDuration(session.value.startedAt, session.value.completedAt)
  );

  return { statusColor, statusLabel, duration };
}
