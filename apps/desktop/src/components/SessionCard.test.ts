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

  it("updates swipe availability when the reused session changes status", async () => {
    const wrapper = mount(SessionCard, {
      props: {
        session: makeSession({ status: "starting" }),
      },
    });

    expect(wrapper.find(".session-card-wrapper").classes()).toContain(
      "swipeable",
    );
    expect(wrapper.find(".swipe-bg").exists()).toBe(true);

    await wrapper.setProps({
      session: makeSession({ status: "running" }),
    });

    expect(wrapper.find(".session-card-wrapper").classes()).not.toContain(
      "swipeable",
    );
    expect(wrapper.find(".swipe-bg").exists()).toBe(false);
  });
});
