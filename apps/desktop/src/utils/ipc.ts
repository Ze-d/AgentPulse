import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../types/agent";

export function getSessions(): Promise<AgentSession[]> {
  return invoke<AgentSession[]>("get_sessions");
}

export function hideMainWindow(): Promise<void> {
  return invoke("hide_main_window");
}
