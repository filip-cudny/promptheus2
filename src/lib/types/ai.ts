export type Effort = "none" | "minimal" | "low" | "medium" | "high" | "xhigh";

export type ReasoningMode =
  | { kind: "unsupported" }
  | { kind: "effort"; allowed: Effort[] }
  | { kind: "budget_tokens"; min: number; max: number }
  | { kind: "toggle" };

export interface ModelCapabilities {
  reasoning: ReasoningMode;
}

export type ToolCallStatus = "pending" | "in_progress" | "completed" | "failed" | "cancelled";

export type ToolCallType = "web_search" | "code_execution" | "file_read" | "file_write" | "api_call" | "custom";

export interface ToolCall {
  tool_call_id: string;
  tool_name: string;
  tool_display_name: string;
  tool_type: ToolCallType;
  arguments: Record<string, unknown>;
  result: string | null;
  error: string | null;
  status: ToolCallStatus;
  requires_confirmation: boolean;
  started_at: string | null;
  completed_at: string | null;
}

export type NodeUpdate =
  | { type: "environment"; value: string }
  | { type: "context"; content: string; reason: string; image_refs: string[] };

export type StreamEvent =
  | { event: "chunk"; data: { delta: string; accumulated: string; thinking_delta: string | null; accumulated_thinking: string | null } }
  | { event: "done"; data: { full_text: string; full_thinking: string | null; prompt_tokens: number | null; completion_tokens: number | null } }
  | { event: "error"; data: { message: string } }
  | { event: "node_updates"; data: { node_id: string; updates: NodeUpdate[] } }
  | { event: "tool_call_start"; data: { tool_call: ToolCall } }
  | { event: "tool_call_progress"; data: { tool_call_id: string; partial_result: string } }
  | { event: "tool_call_done"; data: { tool_call_id: string; result: string | null; error: string | null } }
  | { event: "tool_call_confirmation"; data: { tool_call_id: string } };

export interface ImageUrlData {
  url: string;
}

export type ContentPart =
  | { type: "text"; text: string }
  | { type: "image_url"; image_url: ImageUrlData };

export type MessageContent = string | ContentPart[];

export interface ProcessedMessage {
  role: string;
  content: MessageContent;
}

export interface ImageData {
  data: string;
  media_type: string;
}

export interface AppliedSkill {
  name: string;
  skill_version_id: number;
  input: string;
}

export interface ConversationNodeForExecution {
  node_id: string;
  role: string;
  content: string;
  images: ImageData[];
  text_attachments: string[];
  updates: NodeUpdate[];
  applied_skills: AppliedSkill[];
}
