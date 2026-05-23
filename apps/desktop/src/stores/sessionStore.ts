import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { AgentSession, AgentEvent } from "../types/agent";

interface SessionState {
  sessions: AgentSession[];
  selectedSessionId: string | null;
  expandedSessionId: string | null;
  pollingInterval: number | null;
  error: string | null;
}

export const useSessionStore = defineStore("sessions", {
  state: (): SessionState => ({
    sessions: [],
    selectedSessionId: null,
    expandedSessionId: null,
    pollingInterval: null,
    error: null,
  }),

  getters: {
    activeSessions: (state) =>
      state.sessions.filter(
        (s) => s.status !== "completed" && s.status !== "failed"
      ),

    attentionSessions: (state) =>
      state.sessions.filter((s) => s.needsAttention),

    selectedSession: (state) =>
      state.sessions.find((s) => s.sessionId === state.selectedSessionId) ?? null,

    expandedSession: (state) =>
      state.sessions.find((s) => s.sessionId === state.expandedSessionId) ?? null,
  },

  actions: {
    async fetchSessions() {
      try {
        const result = await invoke<AgentSession[]>("get_sessions");
        console.debug("[AgentPulse] fetchSessions:", result?.length ?? 0, "active");
        this.sessions = result;
        this.error = null;
      } catch (e) {
        console.error("[AgentPulse] fetchSessions error:", e);
        this.error = String(e);
      }
    },

    async fetchSessionDetail(sessionId: string) {
      try {
        const session = await invoke<AgentSession | null>(
          "get_session_detail",
          { sessionId }
        );
        if (session) {
          const idx = this.sessions.findIndex(
            (s) => s.sessionId === sessionId
          );
          if (idx >= 0) {
            this.sessions[idx] = session;
          }
        }
      } catch (e) {
        this.error = String(e);
      }
    },

    async fetchSessionEvents(sessionId: string): Promise<AgentEvent[]> {
      try {
        return await invoke<AgentEvent[]>("get_session_events", { sessionId });
      } catch (e) {
        this.error = String(e);
        return [];
      }
    },

    startPolling(intervalMs = 2000) {
      this.stopPolling();
      this.fetchSessions();
      this.pollingInterval = window.setInterval(() => {
        this.fetchSessions();
      }, intervalMs);
    },

    stopPolling() {
      if (this.pollingInterval !== null) {
        clearInterval(this.pollingInterval);
        this.pollingInterval = null;
      }
    },

    toggleExpand(sessionId: string) {
      this.expandedSessionId =
        this.expandedSessionId === sessionId ? null : sessionId;
    },
  },
});
