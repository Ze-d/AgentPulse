import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("vite build target", () => {
  it("preserves modern JavaScript syntax that the release esbuild version cannot lower", () => {
    const configSource = readFileSync(resolve(__dirname, "../../vite.config.ts"), "utf8");

    expect(configSource).toMatch(/build:\s*{[\s\S]*target:\s*["']esnext["']/);
  });
});
