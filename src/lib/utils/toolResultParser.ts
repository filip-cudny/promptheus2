import type { ParsedToolResult, ToolHint } from "$lib/types/toolResult";

function isToolHint(value: unknown): value is ToolHint {
  if (typeof value !== "object" || value === null) return false;
  const obj = value as Record<string, unknown>;
  return (
    typeof obj.status === "string" &&
    typeof obj.summary === "string" &&
    typeof obj.reasonCode === "string"
  );
}

export function parseToolResult(raw: string | null): ParsedToolResult {
  if (!raw) return { kind: "empty" };

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { kind: "text", text: raw };
  }

  if (typeof parsed === "object" && parsed !== null) {
    const obj = parsed as Record<string, unknown>;
    if ("data" in obj && "hint" in obj && isToolHint(obj.hint)) {
      return { kind: "envelope", hint: obj.hint, data: obj.data };
    }
  }

  return { kind: "json", value: parsed };
}
