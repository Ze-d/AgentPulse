<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";
import { useSessionStore } from "../stores/sessionStore";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import SessionCard from "./SessionCard.vue";
import ExpandedDetail from "./ExpandedDetail.vue";
import { openUrl } from "@tauri-apps/plugin-opener";

const store = useSessionStore();
const closeClicked = ref(false);

onMounted(() => {
  store.startPolling(2000);
});

onUnmounted(() => {
  store.stopPolling();
});

function handleCardClick(sessionId: string) {
  store.toggleExpand(sessionId);
}

async function handleOpenDir(cwd: string) {
  try {
    await openUrl(convertFileSrc(cwd));
  } catch (e) {
    console.error("[AgentPulse] Failed to open dir:", e);
  }
}

async function handleOpenTranscript(path: string) {
  try {
    await openUrl(convertFileSrc(path));
  } catch (e) {
    console.error("[AgentPulse] Failed to open transcript:", e);
  }
}

function handleCloseMousedown() {
  console.log("[AgentPulse] mousedown on close button");
}

async function handleClose() {
  closeClicked.value = true;
  try {
    await invoke("hide_main_window");
  } catch (e) {
    console.error("[AgentPulse] invoke hide_main_window failed:", e);
  }
  closeClicked.value = false;
}

async function adjustWindowSize() {
  const headerHeight = 28;
  const cardHeight = 34;
  const padding = 20;
  const expandedExtra = store.expandedSessionId ? 120 : 0;

  const contentHeight = store.sessions.length > 0
    ? headerHeight + store.sessions.length * cardHeight + padding + expandedExtra
    : 72;

  const height = Math.min(Math.max(contentHeight, 72), 420);

  try {
    await getCurrentWindow().setSize(new LogicalSize(320, height));
  } catch {
    // window API may not be available in browser dev mode
  }
}

watch(
  () => [store.sessions.length, store.expandedSessionId],
  () => adjustWindowSize(),
  { immediate: true }
);
</script>

<template>
  <div class="floating-panel">
    <div class="panel-header">
      <div class="header-drag" data-tauri-drag-region>
        <h1 class="prompt">
          <span class="path">~/agentpulse</span>
          <span class="dollar"> $</span>
        </h1>
        <span class="count">[{{ store.activeSessions.length }} active]</span>
      </div>
      <button
        class="close-btn"
        :class="{ 'close-clicked': closeClicked }"
        @mousedown="handleCloseMousedown"
        @click="handleClose"
        title="Close"
      >{{ closeClicked ? '...' : 'x' }}</button>
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
      data-tauri-drag-region
    >
      <span class="waiting">$ waiting for hooks...</span>
    </div>

    <div v-else class="session-list">
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
  padding: 10px 12px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  user-select: none;
  -webkit-user-select: none;
  line-height: 1.2;
}

.panel-header {
  display: flex;
  align-items: center;
  margin-bottom: 4px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--color-surface0);
}

.header-drag {
  display: flex;
  align-items: center;
  flex: 1;
  cursor: grab;
}

.header-drag:active {
  cursor: grabbing;
}

.header-drag > * {
  pointer-events: none;
}

.prompt {
  font-size: 13px;
  font-weight: 700;
  margin: 0;
}

.path {
  color: var(--color-blue);
}

.dollar {
  color: var(--color-mauve);
}

.count {
  font-size: 11px;
  color: var(--color-overlay0);
  margin-left: auto;
}

.close-btn {
  background: none;
  border: none;
  color: var(--color-overlay0);
  font-size: 14px;
  font-family: inherit;
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  pointer-events: auto;
}

.close-btn:hover {
  color: var(--color-red);
}

.close-clicked {
  color: var(--color-green) !important;
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
  border-radius: 4px;
  padding: 4px 8px;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--color-red);
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  cursor: grab;
  min-height: 24px;
}

.empty-state > * {
  pointer-events: none;
}

.waiting {
  color: var(--color-overlay0);
  font-size: 11px;
}
</style>
