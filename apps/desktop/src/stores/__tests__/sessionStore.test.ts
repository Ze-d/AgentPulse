import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

import { useSessionStore } from "../sessionStore";

describe("sessionStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe("clearError", () => {
    it("sets error to null", () => {
      const store = useSessionStore();
      store.error = "Something went wrong";

      store.clearError();

      expect(store.error).toBeNull();
    });

    it("is a no-op when error is already null", () => {
      const store = useSessionStore();
      store.error = null;

      store.clearError();

      expect(store.error).toBeNull();
    });
  });

  describe("attentionSessions", () => {
    it("returns only sessions that need attention", () => {
      const store = useSessionStore();
      store.sessions = [
        { sessionId: "s1", needsAttention: false } as any,
        { sessionId: "s2", needsAttention: true } as any,
        { sessionId: "s3", needsAttention: false } as any,
        { sessionId: "s4", needsAttention: true } as any,
      ];

      const result = store.attentionSessions;

      expect(result).toHaveLength(2);
      expect(result.map((s) => s.sessionId)).toEqual(["s2", "s4"]);
    });

    it("returns empty array when no sessions need attention", () => {
      const store = useSessionStore();
      store.sessions = [
        { sessionId: "s1", needsAttention: false } as any,
        { sessionId: "s2", needsAttention: false } as any,
      ];

      expect(store.attentionSessions).toHaveLength(0);
    });
  });

  describe("isLoading", () => {
    it("starts as true to indicate initial fetch is pending", () => {
      const store = useSessionStore();
      expect(store.isLoading).toBe(true);
    });

    it("is set to false after fetchSessions completes", async () => {
      const store = useSessionStore();
      mockInvoke.mockResolvedValue([]);

      await store.fetchSessions();

      expect(store.isLoading).toBe(false);
    });

    it("is set to false even when fetch fails", async () => {
      const store = useSessionStore();
      mockInvoke.mockRejectedValue("Error");

      await store.fetchSessions();

      expect(store.isLoading).toBe(false);
    });
  });

  describe("fetchSessions", () => {
    it("clears error on success", async () => {
      const store = useSessionStore();
      store.error = "Previous error";
      mockInvoke.mockResolvedValue([]);

      await store.fetchSessions();

      expect(store.error).toBeNull();
    });

    it("sets error on failure", async () => {
      const store = useSessionStore();
      mockInvoke.mockRejectedValue("Connection refused");

      await store.fetchSessions();

      expect(store.error).toBe("Connection refused");
    });
  });
});
