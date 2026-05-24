# Terminal Pulse Style Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform AgentPulse floating panel into a terminal/CLI-style companion with monospace fonts, adaptive height, and transparent rounded corners.

**Architecture:** Pure frontend refactor — no Rust changes. Monospace font stack applied globally. SessionCard simplified to single-line terminal rows with colored text status (no dots). FloatingPanel uses a Vue watcher to call Tauri Window API for height adjustment after each poll. Window transparency configured in tauri.conf.json eliminates white corner artifacts.

**Tech Stack:** Vue 3 + TypeScript + Pinia + Tauri v2 Window API + CSS custom properties

---

### Task 1: Update status labels for terminal style

**Files:**
- Modify: `apps/desktop/src/types/agent.ts:51-60`

- [ ] **Step 1: Shorten STATUS_LABELS to single-word terminal-style labels**

```typescript
export const STATUS_LABELS: Record<AgentStatus, string> = {
  starting: "starting",
  running: "running",
  tool_running: "tool",
  waiting_input: "waiting",
  waiting_permission: "permission",
  completed: "done",
  failed: "failed",
  unknown: "???",
};
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/types/agent.ts
git commit -m "@ style: shorten STATUS_LABELS to single-word terminal labels"
```

---

### Task 2: Apply monospace font and transparent background

**Files:**
- Modify: `apps/desktop/src/assets/main.css`
- Modify: `apps/desktop/src/App.vue`

- [ ] **Step 1: Update main.css — monospace font stack + transparent background**

Replace the entire file content:

```css
@import "tailwindcss";

:root {
  --color-base: #1e1e2e;
  --color-surface0: #313244;
  --color-surface1: #45475a;
  --color-text: #cdd6f4;
  --color-subtext0: #a6adc8;
  --color-overlay0: #6c7086;
  --color-green: #a6e3a1;
  --color-red: #f38ba8;
  --color-yellow: #f9e2af;
  --color-blue: #89b4fa;
  --color-mauve: #cba6f7;
  --color-peach: #fab387;
  --font-mono: "Cascadia Code", "JetBrains Mono", "Fira Code", "Consolas", monospace;
}

html,
body {
  margin: 0;
  padding: 0;
  background: transparent;
  overflow: hidden;
  font-family: var(--font-mono);
  color: var(--color-text);
}
```

- [ ] **Step 2: Update App.vue — ensure root is transparent**

Edit `apps/desktop/src/App.vue`, in the `<style>` block, ensure background is transparent:

```css
html,
body,
#app {
  margin: 0;
  padding: 0;
  background: transparent;
  overflow: hidden;
  width: 100%;
  height: 100%;
}
```

(No change needed if already `transparent` — verify it reads `background: transparent` not `transparent` with any other values.)

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/assets/main.css apps/desktop/src/App.vue
git commit -m "@ style: apply monospace font stack and transparent background"
```

---

### Task 3: Configure transparent window and adaptive height limits

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`
- Modify: `apps/desktop/src-tauri/capabilities/default.json`

- [ ] **Step 1: Update tauri.conf.json window config**

Edit the `"windows"` array element in `apps/desktop/src-tauri/tauri.conf.json`:

```json
{
  "label": "main",
  "title": "AgentPulse",
  "width": 320,
  "height": 64,
  "minWidth": 280,
  "minHeight": 64,
  "maxHeight": 420,
  "resizable": true,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "visible": true
}
```

Changes: `height: 200 → 64`, added `minHeight: 64` (was 120), added `maxHeight: 420`, added `transparent: true`.

- [ ] **Step 2: Add window:allow-set-size permission**

Edit `apps/desktop/src-tauri/capabilities/default.json`, add to permissions:

```json
{
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-set-size",
    "opener:default"
  ]
}
```

- [ ] **Step 3: Verify Rust compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: `Finished dev profile`

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/tauri.conf.json apps/desktop/src-tauri/capabilities/default.json
git commit -m "@ config: enable transparent window and adaptive height limits"
```

---

### Task 4: Refactor SessionCard to terminal row style

**Files:**
- Modify: `apps/desktop/src/components/SessionCard.vue`

- [ ] **Step 1: Rewrite SessionCard.vue**

Replace the entire file content:

```vue
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
      <span></span>
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
  margin-left: auto;
}
</style>
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/SessionCard.vue
git commit -m "@ style: refactor SessionCard to terminal row layout"
```

---

### Task 5: Refactor ExpandedDetail to terminal style

**Files:**
- Modify: `apps/desktop/src/components/ExpandedDetail.vue`

- [ ] **Step 1: Rewrite ExpandedDetail.vue**

Replace the entire file content:

```vue
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
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/ExpandedDetail.vue
git commit -m "@ style: refactor ExpandedDetail to terminal layout"
```

---

### Task 6: Refactor FloatingPanel with CLI header and adaptive height

**Files:**
- Modify: `apps/desktop/src/components/FloatingPanel.vue`

- [ ] **Step 1: Rewrite FloatingPanel.vue**

Replace the entire file content:

```vue
<script setup lang="ts">
import { watch, onMounted, onUnmounted } from "vue";
import { useSessionStore } from "../stores/sessionStore";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
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

async function adjustWindowSize() {
  const headerHeight = 40;
  const cardHeight = 36;
  const padding = 28;
  const expandedExtra = store.expandedSessionId ? 120 : 0;

  const contentHeight = store.sessions.length > 0
    ? headerHeight + store.sessions.length * cardHeight + padding + expandedExtra
    : 64;

  const height = Math.min(Math.max(contentHeight, 64), 420);

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
    <div class="panel-header" data-tauri-drag-region>
      <h1 class="prompt">
        <span class="path">~/agentpulse</span>
        <span class="dollar"> $</span>
      </h1>
      <span class="count">[{{ store.activeSessions.length }} active]</span>
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
  padding: 14px;
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
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--color-surface0);
  cursor: grab;
}

.panel-header > * {
  pointer-events: none;
}

.panel-header:active {
  cursor: grabbing;
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
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/FloatingPanel.vue
git commit -m "@ style: refactor FloatingPanel with CLI header and adaptive height"
```

---

### Task 7: Final verification

- [ ] **Step 1: Full TypeScript check**

Run: `cd apps/desktop && npx vue-tsc --noEmit`
Expected: zero errors

- [ ] **Step 2: Rust compilation check**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: `Finished dev profile`

- [ ] **Step 3: Final commit if any cleanup needed**

```bash
git status
# Should show clean working tree
```

---

## Verification Summary

| Check | Command | Expected |
|-------|---------|----------|
| TypeScript | `cd apps/desktop && npx vue-tsc --noEmit` | zero errors |
| Rust | `cd apps/desktop/src-tauri && cargo check` | `Finished dev profile` |
| Visual | `cd apps/desktop && npm run tauri dev` | terminal-style panel, draggable, no white corners, adaptive height |
