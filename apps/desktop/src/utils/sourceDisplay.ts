const SOURCE_ABBR: Record<string, string> = {
  "claude-code": "cc",
  codex: "cx",
  gemini: "gm",
  copilot: "cp",
};

export function sourceAbbr(source: string): string {
  return SOURCE_ABBR[source] ?? source;
}
