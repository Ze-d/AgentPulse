import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

// -- hoisted mocks ----------------------------------------------------------

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    setSize: vi.fn().mockResolvedValue(undefined),
  })),
  LogicalSize: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../utils/logger", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
  })),
}));

// -- imports (must be after mocks) ------------------------------------------

import FloatingPanel from "./FloatingPanel.vue";
import { useSessionStore } from "../stores/sessionStore";
import { nextTick } from "vue";

describe("FloatingPanel", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    // Default: getConfig returns default poll interval; getSessions returns empty.
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_config") return Promise.resolve({ pollIntervalMs: 100 });
      if (cmd === "get_sessions") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  // -- empty state ----------------------------------------------------------

  it("shows loading cursor when store is still loading", () => {
    const store = useSessionStore();
    store.isLoading = true;
    store.sessions = [];

    const wrapper = mount(FloatingPanel);

    expect(wrapper.find(".waiting").text()).toContain("_");
  });

  it("shows listening message when not loading and no sessions", async () => {
    const store = useSessionStore();
    store.isLoading = false;
    store.sessions = [];

    const wrapper = mount(FloatingPanel);

    await nextTick();

    expect(wrapper.find(".waiting").text()).toBe("$ agentpulse is listening...");
  });

  // -- session list ---------------------------------------------------------

  it("renders session count in header", () => {
    const store = useSessionStore();
    store.sessions = [
      { sessionId: "s1", status: "running" } as any,
      { sessionId: "s2", status: "starting" } as any,
    ];

    const wrapper = mount(FloatingPanel);

    expect(wrapper.find(".count").text()).toContain("[2 active]");
  });

  it("renders session cards for each session", () => {
    const store = useSessionStore();
    store.sessions = [
      {
        sessionId: "s1",
        source: "claude-code",
        cwd: "/repo",
        projectName: "test",
        status: "running",
        startedAt: Date.now(),
        updatedAt: Date.now(),
        needsAttention: false,
      } as any,
    ];

    const wrapper = mount(FloatingPanel);

    // At least one .session-card should exist
    expect(wrapper.find(".session-list").exists()).toBe(true);
  });

  // -- error banner ---------------------------------------------------------

  it("shows error banner when store has error", async () => {
    const store = useSessionStore();
    store.error = "Connection refused";
    store.sessions = [];
    store.isLoading = false;

    const wrapper = mount(FloatingPanel);

    await nextTick();

    expect(wrapper.find(".error-banner").exists()).toBe(true);
    expect(wrapper.find(".error-text").text()).toBe("Connection refused");
  });

  it("dismisses error banner on button click", async () => {
    const store = useSessionStore();
    store.error = "Something went wrong";
    store.sessions = [];
    store.isLoading = false;

    const wrapper = mount(FloatingPanel);

    await nextTick();
    expect(wrapper.find(".error-banner").exists()).toBe(true);

    await wrapper.find(".error-dismiss").trigger("click");

    expect(store.error).toBeNull();
  });

  it("does not show error banner when there is no error", () => {
    const store = useSessionStore();
    store.error = null;
    store.sessions = [];

    const wrapper = mount(FloatingPanel);

    expect(wrapper.find(".error-banner").exists()).toBe(false);
  });

  // -- header elements ------------------------------------------------------

  it("renders the panel header with prompt", () => {
    const wrapper = mount(FloatingPanel);

    expect(wrapper.find(".prompt").text()).toContain("~/agentpulse");
    expect(wrapper.find(".prompt").text()).toContain("$");
  });

  it("has a close button that minimizes to tray", () => {
    const wrapper = mount(FloatingPanel);

    const closeBtn = wrapper.find(".close-btn");
    expect(closeBtn.exists()).toBe(true);
    expect(closeBtn.attributes("title")).toBe("Minimize to tray");
  });
});
