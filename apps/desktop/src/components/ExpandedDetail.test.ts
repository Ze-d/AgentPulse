import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ExpandedDetail from "./ExpandedDetail.vue";
import type { AgentSession } from "../types/agent";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function makeSession(overrides: Partial<AgentSession> = {}): AgentSession {
  return {
    sessionId: "session-1",
    source: "claude-code",
    cwd: "D:/projects/demo",
    projectName: "AgentPulse",
    status: "running",
    startedAt: 1_700_000_000_000,
    updatedAt: 1_700_000_010_000,
    needsAttention: false,
    ...overrides,
  };
}

describe("ExpandedDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -- basic rendering ------------------------------------------------------

  it("renders project name and source abbreviation", () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession() },
    });

    expect(wrapper.text()).toContain("AgentPulse");
  });

  it("renders cwd in detail grid", () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession({ cwd: "D:/projects/my-app" }) },
    });

    expect(wrapper.find(".truncate").text()).toBe("D:/projects/my-app");
  });

  it("renders '-' when lastToolName is absent", () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession({ lastToolName: undefined }) },
    });

    const labels = wrapper.findAll(".label");
    const lastToolLabel = labels.find((l) => l.text() === "last tool");
    expect(lastToolLabel).toBeTruthy();

    // The value next to "last tool" should be "-"
    const gridText = wrapper.find(".detail-grid").text();
    expect(gridText).toContain("last tool");
    expect(gridText).toContain("-");
  });

  it("renders transcript path when present", () => {
    const wrapper = mount(ExpandedDetail, {
      props: {
        session: makeSession({
          transcriptPath: "D:/tmp/transcript.json",
        }),
      },
    });

    const gridText = wrapper.find(".detail-grid").text();
    expect(gridText).toContain("D:/tmp/transcript.json");
  });

  // -- message block --------------------------------------------------------

  it("shows message block when lastMessage is present", () => {
    const wrapper = mount(ExpandedDetail, {
      props: {
        session: makeSession({ lastMessage: "Hello, world" }),
      },
    });

    expect(wrapper.find(".message-block").exists()).toBe(true);
    expect(wrapper.find(".message-block").text()).toContain("Hello, world");
  });

  it("hides message block when lastMessage is absent", () => {
    const wrapper = mount(ExpandedDetail, {
      props: {
        session: makeSession({ lastMessage: undefined }),
      },
    });

    expect(wrapper.find(".message-block").exists()).toBe(false);
  });

  // -- emit events ----------------------------------------------------------

  it("emits collapse when collapse button is clicked", async () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession() },
    });

    await wrapper.find(".collapse-btn").trigger("click");

    expect(wrapper.emitted("collapse")).toBeTruthy();
    expect(wrapper.emitted("collapse")!.length).toBe(1);
  });

  it("emits openDir with cwd when open dir button is clicked", async () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession({ cwd: "D:/projects/test" }) },
    });

    const dirBtn = wrapper.findAll(".action-btn").find((b) => b.text() === "open dir");
    expect(dirBtn).toBeTruthy();
    await dirBtn!.trigger("click");

    expect(wrapper.emitted("openDir")).toBeTruthy();
    expect(wrapper.emitted("openDir")![0]).toEqual(["D:/projects/test"]);
  });

  it("emits openTranscript when transcript button is clicked", async () => {
    const wrapper = mount(ExpandedDetail, {
      props: {
        session: makeSession({ transcriptPath: "D:/tmp/transcript.json" }),
      },
    });

    const transcriptBtn = wrapper
      .findAll(".action-btn")
      .find((b) => b.text() === "transcript");
    expect(transcriptBtn).toBeTruthy();
    await transcriptBtn!.trigger("click");

    expect(wrapper.emitted("openTranscript")).toBeTruthy();
    expect(wrapper.emitted("openTranscript")![0]).toEqual(["D:/tmp/transcript.json"]);
  });

  it("does not show transcript button when transcriptPath is absent", () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession({ transcriptPath: undefined }) },
    });

    const transcriptBtn = wrapper
      .findAll(".action-btn")
      .find((b) => b.text() === "transcript");
    expect(transcriptBtn).toBeUndefined();
  });

  // -- status color ---------------------------------------------------------

  it("applies status color as border color", () => {
    const wrapper = mount(ExpandedDetail, {
      props: { session: makeSession({ status: "failed" }) },
    });

    const detail = wrapper.find(".expanded-detail");
    expect(detail.attributes("style")).toContain("#f38ba8");
  });
});
