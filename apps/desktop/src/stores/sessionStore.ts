import { defineStore } from "pinia";
import type { AgentSession } from "../types/agent";
import { getSessions } from "../utils/ipc";
import { createLogger } from "../utils/logger";

const logger = createLogger("SessionStore");

interface SessionState {
  sessions: AgentSession[];
  expandedSessionId: string | null;
  pollingInterval: number | null;
  error: string | null;
  isLoading: boolean;
}

export const useSessionStore = defineStore("sessions", {
  state: (): SessionState => ({
    sessions: [],
    expandedSessionId: null,
    pollingInterval: null,
    error: null,
    isLoading: true,
  }),

  getters: {
    activeSessions: (state) =>
      state.sessions.filter(
        (s) => s.status !== "completed" && s.status !== "failed"
      ),

    attentionSessions: (state) =>
      state.sessions.filter((s) => s.needsAttention),

    expandedSession: (state) =>
      state.sessions.find((s) => s.sessionId === state.expandedSessionId) ?? null,
  },

  actions: {
    async fetchSessions() {
      try {
        const result = await getSessions();
        this.sessions = result;
        this.error = null;
        logger.debug(`fetchSessions: ${result.length} sessions`);
      } catch (e) {
        logger.error("fetchSessions failed", e);
        this.error = String(e);
      } finally {
        this.isLoading = false;
      }
    },

    startPolling(intervalMs = 2000) {
      this.stopPolling();
      logger.debug(`startPolling: interval=${intervalMs}ms`);
      this.fetchSessions();
      this.pollingInterval = window.setInterval(() => {
        this.fetchSessions();
      }, intervalMs);
    },

    stopPolling() {
      if (this.pollingInterval !== null) {
        logger.debug("stopPolling");
        clearInterval(this.pollingInterval);
        this.pollingInterval = null;
      }
    },

    toggleExpand(sessionId: string) {
      this.expandedSessionId =
        this.expandedSessionId === sessionId ? null : sessionId;
    },

    clearError() {
      this.error = null;
    },
  },
});
