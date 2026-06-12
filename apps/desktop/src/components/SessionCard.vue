<script setup lang="ts">
import { toRef, ref, onUnmounted, computed } from "vue";
import type { AgentSession } from "../types/agent";
import { DISMISSABLE_STATUSES } from "../types/agent";
import { sourceAbbr } from "../utils/sourceDisplay";
import { useSessionDisplay } from "../composables/useSessionDisplay";
import { useSwipeDismiss } from "../composables/useSwipeDismiss";
import { useSessionStore } from "../stores/sessionStore";

const props = defineProps<{
  session: AgentSession;
}>();

const emit = defineEmits<{
  click: [sessionId: string];
}>();

const sessionRef = toRef(props, "session");
const { statusColor, statusLabel, duration } = useSessionDisplay(sessionRef);
const store = useSessionStore();

const canSwipe = computed(() =>
  DISMISSABLE_STATUSES.includes(props.session.status),
);

/** Dimissing a "starting" session is a cancel action; other statuses are a dismiss. */
const swipeActionLabel = computed(() =>
  props.session.status === "starting" ? "✕ cancel" : "✕ dismiss",
);

/** Amber background for cancelling a starting session; red for other dismissals. */
const swipeBgActiveColor = computed(() =>
  props.session.status === "starting"
    ? "rgba(249, 226, 175, 0.25)"  // Catppuccin Yellow
    : "rgba(243, 139, 168, 0.2)",  // Catppuccin Red
);

const swipeLabelActiveColor = computed(() =>
  props.session.status === "starting" ? "#f9e2af" : "var(--color-red)",
);

const {
  translateX,
  isDismissing,
  dismissed,
  hasMoved,
  moveSwipe,
  endSwipe,
  onTouchStart,
  onTouchMove,
  onTouchEnd,
  onMouseDown,
} = useSwipeDismiss(() => {
  store.dismissSession(props.session.sessionId);
});

// Document-level mouse tracking for swipe
const mouseActive = ref(false);

function handleMouseDown(e: MouseEvent) {
  if (!canSwipe.value) return;
  onMouseDown(e);
  mouseActive.value = true;
  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
}

function handleMouseMove(e: MouseEvent) {
  moveSwipe(e.clientX, e.clientY);
}

function handleMouseUp() {
  mouseActive.value = false;
  document.removeEventListener("mousemove", handleMouseMove);
  document.removeEventListener("mouseup", handleMouseUp);
  endSwipe();
}

onUnmounted(() => {
  document.removeEventListener("mousemove", handleMouseMove);
  document.removeEventListener("mouseup", handleMouseUp);
});

function handleCardClick() {
  // Don't expand if user was swiping
  if (hasMoved.value) return;
  emit("click", props.session.sessionId);
}
</script>

<template>
  <div
    class="session-card-wrapper"
    :class="{ 'swipeable': canSwipe, 'dismissed': dismissed }"
    :style="{ transform: `translateX(${translateX}px)`, opacity: dismissed ? 0 : 1 }"
  >
    <!-- Background: dismiss/cancel indicator shown behind the card during swipe -->
    <div
      v-if="canSwipe"
      class="swipe-bg"
      :class="{ active: isDismissing }"
      :style="isDismissing ? { background: swipeBgActiveColor } : {}"
    >
      <span
        class="swipe-label"
        :style="isDismissing ? { color: swipeLabelActiveColor } : {}"
      >{{ isDismissing ? swipeActionLabel : '← slide' }}</span>
    </div>

    <div
      class="session-card"
      :class="{ attention: session.needsAttention }"
      :style="{ borderLeftColor: statusColor }"
      @click="handleCardClick"
      @touchstart.passive="canSwipe ? onTouchStart($event) : undefined"
      @touchmove.passive="canSwipe ? onTouchMove($event) : undefined"
      @touchend="canSwipe ? onTouchEnd() : undefined"
      @mousedown="handleMouseDown"
    >
      <div class="card-row">
        <span class="project" :style="{ color: statusColor }" :title="session.projectName">
          {{ sourceAbbr(session.source) }} &gt; {{ session.projectName }}
        </span>
        <span class="duration">{{ duration }}</span>
        <span class="status" :style="{ color: statusColor }">{{ statusLabel }}</span>
      </div>
      <div v-if="session.lastToolName" class="card-row secondary">
        <span class="tool">{{ session.lastToolName }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.session-card-wrapper {
  position: relative;
  margin-bottom: 4px;
  transition: opacity 0.25s ease;
  border-radius: 6px;
  overflow: hidden;
}

.swipe-bg {
  position: absolute;
  inset: 0;
  border-radius: 6px;
  background: var(--color-surface0);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding-right: 12px;
  transition: background 0.15s ease;
}

.swipe-bg.active {
  background: rgba(243, 139, 168, 0.2);
}

.swipe-label {
  font-size: 11px;
  color: var(--color-overlay0);
  transition: color 0.15s ease;
}

.swipe-bg.active .swipe-label {
  color: var(--color-red);
}

.session-card {
  background: var(--color-surface0);
  border-radius: 6px;
  padding: 6px 10px;
  border-left: 3px solid;
  cursor: pointer;
  line-height: 1.2;
  position: relative;
  z-index: 1;
  transition: background 0.15s ease;
  min-height: 36px;
  display: flex;
  flex-direction: column;
  justify-content: center;
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

.session-card.attention {
  animation: attention-pulse 2s ease-in-out infinite;
  box-shadow: inset 0 0 8px rgba(250, 179, 135, 0.3);
}

@keyframes attention-pulse {
  0%, 100% {
    box-shadow: inset 0 0 8px rgba(250, 179, 135, 0.3);
  }
  50% {
    box-shadow: inset 0 0 16px rgba(250, 179, 135, 0.6);
  }
}
</style>
