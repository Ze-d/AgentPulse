<script setup lang="ts">
import { computed } from "vue";
import type { AgentSession } from "../types/agent";
import { STATUS_COLORS, STATUS_LABELS, formatDuration } from "../types/agent";

const props = defineProps<{
  session: AgentSession;
}>();

const emit = defineEmits<{
  click: [sessionId: string];
}>();

const statusColor = computed(() => STATUS_COLORS[props.session.status]);
const statusLabel = computed(() => STATUS_LABELS[props.session.status]);
const duration = computed(() =>
  formatDuration(props.session.startedAt, props.session.completedAt)
);
</script>

<template>
  <div
    class="session-card"
    :style="{ borderLeftColor: statusColor }"
    @click="emit('click', session.sessionId)"
  >
    <div class="card-row">
      <span class="project" :style="{ color: statusColor }">
        {{ session.source === "claude-code" ? "cc" : session.source }} &gt; {{ session.projectName }}
      </span>
      <span class="duration">{{ duration }}</span>
      <span class="status" :style="{ color: statusColor }">{{ statusLabel }}</span>
    </div>
    <div v-if="session.lastToolName" class="card-row secondary">
      <span class="tool">{{ session.lastToolName }}</span>
    </div>
  </div>
</template>

<style scoped>
.session-card {
  background: var(--color-surface0);
  border-radius: 6px;
  padding: 6px 10px;
  margin-bottom: 4px;
  border-left: 3px solid;
  cursor: pointer;
  line-height: 1.2;
}

.session-card:hover {
  background: var(--color-surface1);
}

.card-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.card-row.secondary {
  display: grid;
  grid-template-columns: 1fr auto;
  font-size: 11px;
  margin-top: 1px;
}

.project {
  font-weight: 600;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.duration {
  color: var(--color-overlay0);
  flex-shrink: 0;
}

.status {
  flex-shrink: 0;
}

.tool {
  color: var(--color-overlay0);
}
</style>
