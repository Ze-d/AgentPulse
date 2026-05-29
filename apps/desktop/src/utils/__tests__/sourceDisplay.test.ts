import { describe, it, expect } from "vitest";
import { sourceAbbr } from "../sourceDisplay";

describe("sourceAbbr", () => {
  it('returns "cc" for claude-code', () => {
    expect(sourceAbbr("claude-code")).toBe("cc");
  });

  it('returns "cx" for codex', () => {
    expect(sourceAbbr("codex")).toBe("cx");
  });

  it('returns "gm" for gemini', () => {
    expect(sourceAbbr("gemini")).toBe("gm");
  });

  it('returns "cp" for copilot', () => {
    expect(sourceAbbr("copilot")).toBe("cp");
  });

  it("falls back to raw source string for unknown values", () => {
    expect(sourceAbbr("unknown-agent")).toBe("unknown-agent");
  });
});
