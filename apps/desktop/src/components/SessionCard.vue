<script setup lang="ts">
import type { AgentSession } from "../types/agent";
import { STATUS_COLORS, STATUS_LABELS, formatDuration } from "../types/agent";

const props = defineProps<{
  session: AgentSession;
}>();

const emit = defineEmits<{
  click: [sessionId: string];
}>();

const statusColor = STATUS_COLORS[props.session.status];
const statusLabel = STATUS_LABELS[props.session.status];
const duration = formatDuration(
  props.session.startedAt,
  props.session.completedAt
);
</script>

<template>
  <div
    class="session-card"
    :style="{ borderLeftColor: statusColor }"
    :class="{ 'needs-attention': session.needsAttention }"
    @click="emit('click', session.sessionId)"
  >
    <div class="flex items-center gap-2 mb-1">
      <span
        class="status-dot"
        :style="{ backgroundColor: statusColor }"
      ></span>
      <span class="text-sm font-semibold" style="color: var(--color-text)">
        {{ session.source === "claude-code" ? "Claude Code" : session.source }}
      </span>
      <span class="ml-auto text-xs" style="color: var(--color-subtext0)">
        {{ duration }}
      </span>
    </div>
    <div class="text-xs" style="color: var(--color-subtext0)">
      {{ session.projectName }}
    </div>
    <div class="flex items-center justify-between mt-1">
      <span class="text-xs" style="color: var(--color-overlay0)">
        {{ statusLabel }}
      </span>
      <span
        v-if="session.lastToolName"
        class="text-xs"
        style="color: var(--color-overlay0)"
      >
        {{ session.lastToolName }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.session-card {
  background: var(--color-surface0);
  border-radius: 8px;
  padding: 10px;
  margin-bottom: 6px;
  border-left: 3px solid;
  cursor: pointer;
  transition: background 0.15s;
}

.session-card:hover {
  background: var(--color-surface1);
}

.needs-attention {
  animation: pulse-border 2s infinite;
}

@keyframes pulse-border {
  0%,
  100% {
    border-left-color: var(--color-peach);
  }
  50% {
    border-left-color: transparent;
  }
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
  flex-shrink: 0;
}
</style>
