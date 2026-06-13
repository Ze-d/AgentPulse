import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SessionCard from "./SessionCard.vue";
import type { AgentSession } from "../types/agent";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function makeSession(overrides: Partial<AgentSession> = {}): AgentSession {
  return {
    sessionId: "session-1",
    source: "claude-code",
    cwd: "/repo",
    projectName: "AgentPulse",
    status: "starting",
    startedAt: 1_700_000_000_000,
    updatedAt: 1_700_000_000_000,
    needsAttention: false,
    ...overrides,
  };
}

describe("SessionCard", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  // -- swipe availability ---------------------------------------------------

  it("allows swipe dismissal for running status (recovery from interrupted sessions)", async () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ status: "running" }) },
    });

    // All statuses including "running" should be swipeable so users can
    // manually dismiss panels when the backend process checker cannot
    // recover the session (e.g. hung process or missing PID).
    expect(wrapper.find(".session-card-wrapper").classes()).toContain("swipeable");
    expect(wrapper.find(".swipe-bg").exists()).toBe(true);
  });

  // -- status color ---------------------------------------------------------

  it("renders correct border color for completed status", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ status: "completed" }) },
    });

    const card = wrapper.find(".session-card");
    expect(card.attributes("style")).toContain("#a6e3a1"); // Catppuccin Green
  });

  it("renders correct border color for failed status", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ status: "failed" }) },
    });

    const card = wrapper.find(".session-card");
    expect(card.attributes("style")).toContain("#f38ba8"); // Catppuccin Red
  });

  it("renders correct border color for running status", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ status: "running" }) },
    });

    const card = wrapper.find(".session-card");
    expect(card.attributes("style")).toContain("#94e2d5"); // Catppuccin Teal
  });

  // -- click event ----------------------------------------------------------

  it("emits click with sessionId when card is clicked", async () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ sessionId: "click-test-1" }) },
    });

    await wrapper.find(".session-card").trigger("click");

    expect(wrapper.emitted("click")).toBeTruthy();
    expect(wrapper.emitted("click")![0]).toEqual(["click-test-1"]);
  });

  // -- tool name display ----------------------------------------------------

  it("displays tool name when session has lastToolName", () => {
    const wrapper = mount(SessionCard, {
      props: {
        session: makeSession({
          status: "tool_running",
          lastToolName: "Bash",
        }),
      },
    });

    expect(wrapper.find(".tool").text()).toBe("Bash");
  });

  it("does not render tool row when lastToolName is absent", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ lastToolName: undefined }) },
    });

    expect(wrapper.find(".tool").exists()).toBe(false);
  });

  // -- attention pulse ------------------------------------------------------

  it("applies attention class when session needs attention", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ needsAttention: true }) },
    });

    expect(wrapper.find(".session-card").classes()).toContain("attention");
  });

  it("does not apply attention class normally", () => {
    const wrapper = mount(SessionCard, {
      props: { session: makeSession({ needsAttention: false }) },
    });

    expect(wrapper.find(".session-card").classes()).not.toContain("attention");
  });

  // -- project name / source abbreviation -----------------------------------

  it("renders source abbreviation and project name", () => {
    const wrapper = mount(SessionCard, {
      props: {
        session: makeSession({
          source: "codex",
          projectName: "my-project",
        }),
      },
    });

    expect(wrapper.find(".project").text()).toContain("cx");
    expect(wrapper.find(".project").text()).toContain("my-project");
  });
});
