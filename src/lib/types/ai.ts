export type NodeUpdate =
  | { type: "environment"; value: string }
  | { type: "context"; content: string; reason: string; image_refs: string[] };

export type StreamEvent =
  | { event: "chunk"; data: { delta: string; accumulated: string; thinking_delta: string | null; accumulated_thinking: string | null } }
  | { event: "done"; data: { full_text: string; full_thinking: string | null; prompt_tokens: number | null; completion_tokens: number | null } }
  | { event: "error"; data: { message: string } }
  | { event: "node_updates"; data: { node_id: string; updates: NodeUpdate[] } };

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

export interface ConversationNodeForExecution {
  node_id: string;
  role: string;
  content: string;
  images: ImageData[];
  text_attachments: string[];
  updates: NodeUpdate[];
}
