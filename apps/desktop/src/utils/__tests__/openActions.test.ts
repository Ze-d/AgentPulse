import { describe, it, expect, vi, beforeEach } from "vitest";

const { mockOpenPath, mockOpenUrl } = vi.hoisted(() => ({
  mockOpenPath: vi.fn(),
  mockOpenUrl: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: mockOpenPath,
  openUrl: mockOpenUrl,
  revealItemInDir: vi.fn(),
}));

import { openDirectory, openTranscript } from "../openActions";

describe("openDirectory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls openPath with the directory path — not openUrl", async () => {
    mockOpenPath.mockResolvedValue(undefined);

    await openDirectory("/home/user/project");

    expect(mockOpenPath).toHaveBeenCalledWith("/home/user/project");
    expect(mockOpenUrl).not.toHaveBeenCalled();
  });

  it("throws when openPath fails, so caller can set store.error", async () => {
    const err = new Error("Permission denied");
    mockOpenPath.mockRejectedValue(err);

    await expect(openDirectory("/protected")).rejects.toThrow(
      "Failed to open directory: Error: Permission denied"
    );

    expect(mockOpenPath).toHaveBeenCalledWith("/protected");
  });
});

describe("openTranscript", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls openPath with the transcript path — not openUrl", async () => {
    mockOpenPath.mockResolvedValue(undefined);

    await openTranscript("/tmp/transcript.json");

    expect(mockOpenPath).toHaveBeenCalledWith("/tmp/transcript.json");
    expect(mockOpenUrl).not.toHaveBeenCalled();
  });

  it("throws when openPath fails, so caller can set store.error", async () => {
    const err = new Error("File not found");
    mockOpenPath.mockRejectedValue(err);

    await expect(openTranscript("/missing/transcript.json")).rejects.toThrow(
      "Failed to open transcript: Error: File not found"
    );

    expect(mockOpenPath).toHaveBeenCalledWith("/missing/transcript.json");
  });
});
