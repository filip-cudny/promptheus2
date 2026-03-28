export type StreamEvent =
  | { event: "chunk"; data: { delta: string; accumulated: string } }
  | { event: "done"; data: { full_text: string } }
  | { event: "error"; data: { message: string } };

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

export interface ConversationMessage {
  role: string;
  content: MessageContent;
}
