import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../types/agent";
import { createLogger } from "./logger";

const logger = createLogger("IPC");

export async function getSessions(): Promise<AgentSession[]> {
  try {
    return await invoke<AgentSession[]>("get_sessions");
  } catch (e) {
    logger.error("get_sessions IPC failed", e);
    throw e;
  }
}

export interface FrontendConfig {
  pollIntervalMs: number;
}

export async function getConfig(): Promise<FrontendConfig> {
  try {
    return await invoke<FrontendConfig>("get_config");
  } catch (e) {
    logger.warn("get_config IPC failed, using defaults", e);
    return { pollIntervalMs: 2000 };
  }
}

export async function hideMainWindow(): Promise<void> {
  try {
    await invoke("hide_main_window");
  } catch (e) {
    logger.error("hide_main_window IPC failed", e);
    throw e;
  }
}

export async function deleteSession(sessionId: string): Promise<void> {
  try {
    await invoke("delete_session", { sessionId });
    logger.debug(`deleteSession: ${sessionId}`);
  } catch (e) {
    logger.error("delete_session IPC failed", e);
    throw e;
  }
}
