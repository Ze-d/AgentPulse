import { ref, computed } from "vue";

const SWIPE_THRESHOLD = 80;

export function useSwipeDismiss(onDismiss: () => void) {
  const translateX = ref(0);
  const swiping = ref(false);
  const dismissed = ref(false);
  const startX = ref(0);
  const startY = ref(0);
  const hasMoved = ref(false);

  const isDismissing = computed(() => translateX.value > SWIPE_THRESHOLD);
  const progress = computed(() =>
    Math.min(translateX.value / SWIPE_THRESHOLD, 1)
  );

  function startSwipe(clientX: number, clientY: number) {
    startX.value = clientX;
    startY.value = clientY;
    swiping.value = true;
    hasMoved.value = false;
  }

  function moveSwipe(clientX: number, clientY: number) {
    if (!swiping.value) return;
    const dx = clientX - startX.value;
    const dy = clientY - startY.value;

    // Ignore vertical swipes — let the scroll container handle them
    if (Math.abs(dy) > Math.abs(dx) * 1.2 && !hasMoved.value) {
      swiping.value = false;
      return;
    }

    // Small deadzone to distinguish swipe from tap
    if (!hasMoved.value && Math.abs(dx) + Math.abs(dy) < 4) return;
    hasMoved.value = true;

    // Only rightward swipe (positive X), clamped to 0..200
    translateX.value = Math.max(0, Math.min(dx, 200));
  }

  function endSwipe() {
    swiping.value = false;
    if (translateX.value > SWIPE_THRESHOLD) {
      dismissed.value = true;
      setTimeout(() => {
        onDismiss();
      }, 250);
    } else {
      translateX.value = 0;
    }
  }

  // Touch event wrappers — bound directly on the element
  function onTouchStart(e: TouchEvent) {
    startSwipe(e.touches[0]?.clientX ?? 0, e.touches[0]?.clientY ?? 0);
  }

  function onTouchMove(e: TouchEvent) {
    moveSwipe(e.touches[0]?.clientX ?? 0, e.touches[0]?.clientY ?? 0);
  }

  function onTouchEnd() {
    endSwipe();
  }

  // Mouse-down bound on element; move/up handled at document level by caller
  function onMouseDown(e: MouseEvent) {
    startSwipe(e.clientX, e.clientY);
  }

  return {
    translateX,
    swiping,
    isDismissing,
    dismissed,
    progress,
    hasMoved,
    startSwipe,
    moveSwipe,
    endSwipe,
    onTouchStart,
    onTouchMove,
    onTouchEnd,
    onMouseDown,
  };
}
