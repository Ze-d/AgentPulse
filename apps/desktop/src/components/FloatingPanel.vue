<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useSessionStore } from "../stores/sessionStore";
import SessionCard from "./SessionCard.vue";
import ExpandedDetail from "./ExpandedDetail.vue";
import { openUrl } from "@tauri-apps/plugin-opener";

const store = useSessionStore();

onMounted(() => {
  store.startPolling(2000);
});

onUnmounted(() => {
  store.stopPolling();
});

function handleCardClick(sessionId: string) {
  store.toggleExpand(sessionId);
}

function handleOpenDir(cwd: string) {
  openUrl(`file:///${cwd}`);
}

function handleOpenTranscript(path: string) {
  openUrl(`file:///${path}`);
}
</script>

<template>
  <div class="floating-panel" data-tauri-drag-region>
    <div class="panel-header" data-tauri-drag-region>
      <h1 class="text-sm font-bold" style="color: var(--color-mauve)">
        AgentPulse
      </h1>
      <span class="text-xs" style="color: var(--color-overlay0)">
        {{ store.activeSessions.length }} active
      </span>
    </div>

    <div
      v-if="store.error"
      class="error-banner"
    >
      {{ store.error }}
    </div>

    <div
      v-if="store.sessions.length === 0 && !store.error"
      class="empty-state"
    >
      <p style="color: var(--color-overlay0); font-size: 12px">
        No active sessions
      </p>
      <p style="color: var(--color-overlay0); font-size: 10px; margin-top: 4px">
        Waiting for Claude Code hook events...
      </p>
    </div>

    <div class="session-list">
      <template v-for="session in store.sessions" :key="session.sessionId">
        <ExpandedDetail
          v-if="store.expandedSessionId === session.sessionId"
          :session="session"
          @collapse="store.toggleExpand(session.sessionId)"
          @open-dir="handleOpenDir"
          @open-transcript="handleOpenTranscript"
        />
        <SessionCard
          v-else
          :session="session"
          @click="handleCardClick"
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
.floating-panel {
  background: var(--color-base);
  border-radius: 12px;
  padding: 12px;
  min-height: 100vh;
  height: 100vh;
  display: flex;
  flex-direction: column;
  user-select: none;
  -webkit-user-select: none;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-surface0);
  cursor: grab;
}

.panel-header:active {
  cursor: grabbing;
}

.session-list {
  flex: 1;
  overflow-y: auto;
}

.session-list::-webkit-scrollbar {
  width: 4px;
}

.session-list::-webkit-scrollbar-thumb {
  background: var(--color-surface1);
  border-radius: 2px;
}

.error-banner {
  background: rgba(243, 139, 168, 0.15);
  border: 1px solid var(--color-red);
  border-radius: 6px;
  padding: 6px 10px;
  margin-bottom: 8px;
  font-size: 11px;
  color: var(--color-red);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
}
</style>
