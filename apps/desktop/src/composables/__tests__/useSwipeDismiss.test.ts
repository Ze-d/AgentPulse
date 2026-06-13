import { describe, it, expect, vi, beforeEach } from "vitest";
import { useSwipeDismiss } from "../useSwipeDismiss";

describe("useSwipeDismiss", () => {
  let onDismiss: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onDismiss = vi.fn();
  });

  // -- initial state --------------------------------------------------------

  it("starts with translateX at 0 and not dismissed", () => {
    const { translateX, dismissed, isDismissing } = useSwipeDismiss(onDismiss);

    expect(translateX.value).toBe(0);
    expect(dismissed.value).toBe(false);
    expect(isDismissing.value).toBe(false);
  });

  // -- swipe below threshold resets -----------------------------------------

  it("resets translateX when swipe ends below threshold", () => {
    const { moveSwipe, endSwipe, startSwipe, translateX } =
      useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    // Move past the 4px deadzone first.
    moveSwipe(6, 1); // dx=6, dy=1 → horizontal, >4px total
    moveSwipe(50, 2);
    endSwipe();

    expect(translateX.value).toBe(0);
    expect(onDismiss).not.toHaveBeenCalled();
  });

  // -- swipe above threshold triggers dismiss -------------------------------

  it("triggers dismiss callback after crossing threshold", async () => {
    vi.useFakeTimers();
    const { moveSwipe, endSwipe, startSwipe, dismissed } =
      useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(6, 1); // past deadzone
    moveSwipe(100, 2); // crossed 80px threshold
    endSwipe();

    expect(dismissed.value).toBe(true);
    // onDismiss is called after a 250ms setTimeout
    expect(onDismiss).not.toHaveBeenCalled();

    vi.advanceTimersByTime(250);
    expect(onDismiss).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });

  // -- isDismissing reflects threshold crossing -----------------------------

  it("sets isDismissing to true when translateX crosses threshold", () => {
    const { moveSwipe, startSwipe, isDismissing } = useSwipeDismiss(onDismiss);

    expect(isDismissing.value).toBe(false);

    startSwipe(0, 0);
    moveSwipe(6, 1); // past deadzone
    moveSwipe(90, 2); // 90 > 80 threshold

    expect(isDismissing.value).toBe(true);
  });

  // -- vertical swipe is ignored --------------------------------------------

  it("cancels swipe when movement is primarily vertical (before crossing deadzone)", () => {
    const { moveSwipe, startSwipe, swiping } = useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    // First move stays within deadzone so hasMoved stays false.
    moveSwipe(1, 1); // total=2 < 4, deadzone — no-op
    // Second move crosses deadzone but is primarily vertical → cancelled.
    moveSwipe(10, 40); // dy=40, dx=10 → dy > dx*1.2, hasMoved is still false

    expect(swiping.value).toBe(false);
  });

  // -- deadzone prevents tap from being treated as swipe --------------------

  it("does not set hasMoved within the 4px deadzone", () => {
    const { moveSwipe, startSwipe, hasMoved } = useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(1, 1); // total=2 < 4

    expect(hasMoved.value).toBe(false);
  });

  it("sets hasMoved once outside the deadzone", () => {
    const { moveSwipe, startSwipe, hasMoved } = useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(3, 3); // total=6 > 4, sets hasMoved

    expect(hasMoved.value).toBe(true);
  });

  // -- translateX is clamped to 0..200 --------------------------------------

  it("clamps translateX to 0 minimum", () => {
    const { moveSwipe, startSwipe, translateX } = useSwipeDismiss(onDismiss);

    startSwipe(100, 0);
    moveSwipe(96, 1); // past deadzone
    moveSwipe(50, 0); // dx = -50 → clamped to 0

    expect(translateX.value).toBe(0);
  });

  it("clamps translateX to 200 maximum", () => {
    const { moveSwipe, startSwipe, translateX } = useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(6, 1); // past deadzone
    moveSwipe(300, 0); // dx = 300 → clamped to 200

    expect(translateX.value).toBe(200);
  });

  // -- mouse tracking -------------------------------------------------------

  it("onMouseDown starts swipe tracking", () => {
    const { onMouseDown, swiping, startSwipe } = useSwipeDismiss(onDismiss);

    // Simulate a mouse down event
    const mouseEvent = new MouseEvent("mousedown", {
      clientX: 50,
      clientY: 10,
    });
    onMouseDown(mouseEvent);

    expect(swiping.value).toBe(true);
  });

  // -- dismissed flag --------------------------------------------------------

  it("sets dismissed to true when swipe crosses threshold on end", () => {
    const { moveSwipe, endSwipe, startSwipe, dismissed } =
      useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(6, 1);
    moveSwipe(100, 2);
    endSwipe();

    expect(dismissed.value).toBe(true);
  });

  // -- progress computed ----------------------------------------------------

  it("reports progress as a fraction of threshold", () => {
    const { moveSwipe, startSwipe, progress } = useSwipeDismiss(onDismiss);

    startSwipe(0, 0);
    moveSwipe(6, 1);
    moveSwipe(40, 0); // 40 / 80 = 0.5

    expect(progress.value).toBeCloseTo(0.5, 1);
  });
});
