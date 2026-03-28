export type StreamEvent =
  | { event: "chunk"; data: { delta: string; accumulated: string } }
  | { event: "done"; data: { full_text: string } }
  | { event: "error"; data: { message: string } };
