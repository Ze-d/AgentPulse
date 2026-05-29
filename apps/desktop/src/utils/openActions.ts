import { openPath } from "@tauri-apps/plugin-opener";

export async function openDirectory(path: string): Promise<void> {
  try {
    await openPath(path);
  } catch (e) {
    throw new Error(`Failed to open directory: ${e}`);
  }
}

export async function openTranscript(path: string): Promise<void> {
  try {
    await openPath(path);
  } catch (e) {
    throw new Error(`Failed to open transcript: ${e}`);
  }
}
