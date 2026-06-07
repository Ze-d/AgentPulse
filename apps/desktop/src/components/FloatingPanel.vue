<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";
import { useSessionStore } from "../stores/sessionStore";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { hideMainWindow, getConfig } from "../utils/ipc";
import { createLogger } from "../utils/logger";
import SessionCard from "./SessionCard.vue";
import ExpandedDetail from "./ExpandedDetail.vue";
import { openDirectory, openTranscript } from "../utils/openActions";

const logger = createLogger("FloatingPanel");

const HEADER_HEIGHT = 28;
const CARD_HEIGHT = 34;
const PANEL_PADDING = 20;
const DEFAULT_POLL_INTERVAL = 2000;

const store = useSessionStore();
const closeClicked = ref(false);

onMounted(async () => {
  const config = await getConfig();
  store.startPolling(config.pollIntervalMs || DEFAULT_POLL_INTERVAL);
});

onUnmounted(() => {
  store.stopPolling();
});

function handleCardClick(sessionId: string) {
  store.toggleExpand(sessionId);
}

async function handleOpenDir(cwd: string) {
  try {
    await openDirectory(cwd);
    store.error = null;
    logger.info(`opened directory: ${cwd}`);
  } catch (e) {
    logger.error("failed to open directory", e);
    store.error = String(e);
  }
}

async function handleOpenTranscript(path: string) {
  try {
    await openTranscript(path);
    store.error = null;
    logger.info(`opened transcript: ${path}`);
  } catch (e) {
    logger.error("failed to open transcript", e);
    store.error = String(e);
  }
}

function handleCloseMousedown() {
  logger.debug("mousedown on close button");
}

async function handleClose() {
  closeClicked.value = true;
  try {
    await hideMainWindow();
    logger.debug("window hidden to tray");
  } catch (e) {
    logger.error("hide_main_window failed", e);
  }
  closeClicked.value = false;
}

async function adjustWindowSize() {
  const expandedExtra = store.expandedSessionId ? 120 : 0;

  const contentHeight = store.sessions.length > 0
    ? HEADER_HEIGHT + store.sessions.length * CARD_HEIGHT + PANEL_PADDING + expandedExtra
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
        <span class="count">[{{ store.sessions.length }} active]</span>
      </div>
      <button class="refresh-btn" @click="store.fetchSessions()" title="Refresh">↻</button>
      <button
        class="close-btn"
        :class="{ 'close-clicked': closeClicked }"
        @mousedown="handleCloseMousedown"
        @click="handleClose"
        title="Minimize to tray"
      >{{ closeClicked ? '...' : '_' }}</button>
    </div>

    <div
      v-if="store.error"
      class="error-banner"
    >
      <span class="error-text">{{ store.error }}</span>
      <button class="error-dismiss" @click="store.clearError()" title="Dismiss">x</button>
    </div>

    <div
      v-if="store.sessions.length === 0 && !store.error"
      class="empty-state"
      data-tauri-drag-region
    >
      <span v-if="store.isLoading" class="waiting">$ <span class="cursor-blink">_</span></span>
      <span v-else class="waiting">$ agentpulse is listening...</span>
    </div>

    <div v-else class="session-list">
      <template v-for="session in store.sessions" :key="session.sessionId">
        <Transition name="slide">
          <ExpandedDetail
            v-if="store.expandedSessionId === session.sessionId"
            :session="session"
            @collapse="store.toggleExpand(session.sessionId)"
            @open-dir="handleOpenDir"
            @open-transcript="handleOpenTranscript"
          />
        </Transition>
        <SessionCard
          v-if="store.expandedSessionId !== session.sessionId"
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

.refresh-btn {
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

.refresh-btn:hover {
  color: var(--color-blue);
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
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(243, 139, 168, 0.15);
  border: 1px solid var(--color-red);
  border-radius: 4px;
  padding: 4px 8px;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--color-red);
}

.error-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.error-dismiss {
  background: none;
  border: none;
  color: var(--color-red);
  cursor: pointer;
  font-size: 14px;
  font-family: inherit;
  padding: 0 2px;
  line-height: 1;
  flex-shrink: 0;
}

.error-dismiss:hover {
  color: var(--color-maroon);
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

.cursor-blink {
  animation: blink 1s step-end infinite;
  font-weight: 700;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.slide-enter-active,
.slide-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
