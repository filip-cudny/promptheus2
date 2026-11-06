import type { StreamEvent, ToolCall } from "$lib/types/ai";

export interface AgentReply {
  text: string;
  toolCalls: ToolCall[];
  thinkingParts: string[];
  thinking: string | null;
  isThinking: boolean;
  iterationText: string;
}

export function emptyReply(): AgentReply {
  return { text: "", toolCalls: [], thinkingParts: [], thinking: null, isThinking: false, iterationText: "" };
}

export function applyStreamEvent(reply: AgentReply, event: StreamEvent) {
  switch (event.event) {
    case "chunk":
      reply.text = event.data.accumulated;
      if (event.data.accumulated_thinking != null) reply.thinking = event.data.accumulated_thinking;
      reply.isThinking = event.data.accumulated_thinking != null && event.data.accumulated === reply.iterationText;
      break;
    case "tool_call_start":
      if (reply.thinking != null) {
        reply.thinkingParts.push(reply.thinking);
        reply.thinking = null;
      }
      reply.iterationText = reply.text;
      reply.isThinking = false;
      reply.toolCalls.push({ ...event.data.tool_call });
      break;
    case "tool_call_done": {
      const call = reply.toolCalls.find((c) => c.tool_call_id === event.data.tool_call_id);
      if (call) {
        call.result = event.data.result;
        call.error = event.data.error;
        call.status = event.data.error ? "failed" : "completed";
        call.completed_at = new Date().toISOString();
      }
      break;
    }
    case "done":
      reply.text = event.data.full_text;
      if (event.data.full_thinking != null) reply.thinking = event.data.full_thinking;
      reply.isThinking = false;
      break;
  }
}

export function thinkingText(reply: AgentReply): string {
  const parts = reply.thinking != null ? [...reply.thinkingParts, reply.thinking] : reply.thinkingParts;
  return parts.join("\n\n");
}
