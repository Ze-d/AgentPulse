<script setup lang="ts">
import type { AgentSession } from "../types/agent";
import { STATUS_COLORS, STATUS_LABELS, formatDuration } from "../types/agent";

const props = defineProps<{
  session: AgentSession;
}>();

const emit = defineEmits<{
  collapse: [];
  openDir: [cwd: string];
  openTranscript: [path: string];
}>();

const statusColor = STATUS_COLORS[props.session.status];
const statusLabel = STATUS_LABELS[props.session.status];
const duration = formatDuration(
  props.session.startedAt,
  props.session.completedAt
);
</script>

<template>
  <div class="expanded-detail" :style="{ borderColor: statusColor }">
    <div class="flex items-center justify-between mb-3">
      <span class="text-sm font-bold" style="color: var(--color-mauve)">
        {{ session.source === "claude-code" ? "Claude Code" : session.source }}
        · {{ session.projectName }}
      </span>
      <button
        class="text-xs"
        style="color: var(--color-overlay0); background: none; border: none; cursor: pointer"
        @click="emit('collapse')"
      >
        Collapse
      </button>
    </div>

    <div class="detail-grid">
      <span class="label">Status</span>
      <span :style="{ color: statusColor }">{{ statusLabel }}</span>

      <span class="label">Duration</span>
      <span>{{ duration }}</span>

      <span class="label">Working Dir</span>
      <span class="truncate" :title="session.cwd">{{ session.cwd }}</span>

      <span class="label">Last Tool</span>
      <span>{{ session.lastToolName || "-" }}</span>

      <span class="label">Transcript</span>
      <span>{{ session.transcriptPath || "-" }}</span>
    </div>

    <div
      v-if="session.lastMessage"
      class="message-block"
    >
      {{ session.lastMessage }}
    </div>

    <div class="flex gap-3 mt-3 justify-end">
      <button
        class="action-link"
        @click="emit('openDir', session.cwd)"
      >
        Open Folder
      </button>
      <button
        v-if="session.transcriptPath"
        class="action-link"
        @click="emit('openTranscript', session.transcriptPath)"
      >
        Transcript
      </button>
    </div>
  </div>
</template>

<style scoped>
.expanded-detail {
  background: var(--color-surface0);
  border-radius: 8px;
  border: 1px solid;
  padding: 12px;
  margin-bottom: 6px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 4px 8px;
  font-size: 11px;
  color: var(--color-text);
}

.label {
  color: var(--color-overlay0);
}

.message-block {
  margin-top: 8px;
  padding: 8px;
  background: var(--color-base);
  border-radius: 4px;
  font-size: 10px;
  color: var(--color-subtext0);
  max-height: 60px;
  overflow-y: auto;
}

.action-link {
  background: none;
  border: none;
  color: var(--color-blue);
  font-size: 11px;
  cursor: pointer;
}

.action-link:hover {
  text-decoration: underline;
}
</style>
