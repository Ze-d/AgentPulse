<script setup lang="ts">
import { computed } from "vue";
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

const statusColor = computed(() => STATUS_COLORS[props.session.status]);
const statusLabel = computed(() => STATUS_LABELS[props.session.status]);
const duration = computed(() =>
  formatDuration(props.session.startedAt, props.session.completedAt)
);
</script>

<template>
  <div class="expanded-detail" :style="{ borderColor: statusColor }">
    <div class="detail-header">
      <span :style="{ color: statusColor }">
        cc &gt; {{ session.projectName }}
      </span>
      <button class="collapse-btn" @click="emit('collapse')">[ - ]</button>
    </div>

    <div class="detail-grid">
      <span class="label">status</span>
      <span :style="{ color: statusColor }">{{ statusLabel }}</span>

      <span class="label">duration</span>
      <span>{{ duration }}</span>

      <span class="label">cwd</span>
      <span class="truncate" :title="session.cwd">{{ session.cwd }}</span>

      <span class="label">last tool</span>
      <span>{{ session.lastToolName || "-" }}</span>

      <span class="label">transcript</span>
      <span class="truncate">{{ session.transcriptPath || "-" }}</span>
    </div>

    <div
      v-if="session.lastMessage"
      class="message-block"
    >
      <span class="prompt">$ </span>{{ session.lastMessage }}
    </div>

    <div class="detail-actions">
      <button class="action-btn" @click="emit('openDir', session.cwd)">open dir</button>
      <button
        v-if="session.transcriptPath"
        class="action-btn"
        @click="emit('openTranscript', session.transcriptPath)"
      >transcript</button>
    </div>
  </div>
</template>

<style scoped>
.expanded-detail {
  background: var(--color-surface0);
  border-radius: 6px;
  border: 1px solid;
  padding: 10px;
  margin-bottom: 4px;
  font-size: 12px;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-weight: 600;
}

.collapse-btn {
  background: none;
  border: none;
  color: var(--color-overlay0);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: 11px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 72px 1fr;
  gap: 3px 8px;
  font-size: 11px;
}

.label {
  color: var(--color-overlay0);
}

.message-block {
  margin-top: 8px;
  padding: 6px 8px;
  background: var(--color-base);
  border-radius: 4px;
  font-size: 11px;
  color: var(--color-subtext0);
  max-height: 60px;
  overflow-y: auto;
}

.prompt {
  color: var(--color-mauve);
}

.detail-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 10px;
}

.action-btn {
  background: none;
  border: none;
  color: var(--color-blue);
  font-family: var(--font-mono);
  font-size: 11px;
  cursor: pointer;
}

.action-btn:hover {
  text-decoration: underline;
}
</style>
