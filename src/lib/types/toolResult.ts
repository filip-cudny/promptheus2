export interface ToolHintRecovery {
  retryable: boolean;
  backoffMs: number;
  maxAttempts: number;
}

export interface ToolHintNextAction {
  tool: string;
  why: string;
  args: Record<string, unknown>;
  confidence: number;
}

export type HintStatus = "success" | "empty" | "partial" | "error";

export interface ToolHint {
  status: HintStatus;
  reasonCode: string;
  summary: string;
  nextActions: ToolHintNextAction[];
  recovery: ToolHintRecovery;
  diagnostics?: Record<string, string>;
}

export interface ToolEnvelope<T = unknown> {
  data: T;
  hint: ToolHint;
}

export type ParsedToolResult =
  | { kind: "empty" }
  | { kind: "envelope"; hint: ToolHint; data: unknown }
  | { kind: "json"; value: unknown }
  | { kind: "text"; text: string };
