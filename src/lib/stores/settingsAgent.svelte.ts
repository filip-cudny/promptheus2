import type { StreamEvent, ToolCallStatus } from "$lib/types/ai";
import {
  settingsAgentCancel,
  settingsAgentPanel,
  settingsAgentReset,
  settingsAgentSend,
  type PanelMode,
} from "$lib/services/settingsAgent";

export interface AgentToolCall {
  id: string;
  name: string;
  status: ToolCallStatus;
}

export type AgentMessage =
  | { id: number; role: "user"; text: string }
  | { id: number; role: "assistant"; text: string; toolCalls: AgentToolCall[] };

let nextId = 0;
let messages = $state<AgentMessage[]>([]);
let running = $state(false);
let error = $state<string | null>(null);
let panelOpen = $state(false);
let panelMode = $state<PanelMode>("overlay");

settingsAgentReset().catch((e) => (error = String(e)));

function lastAssistant() {
  const last = messages[messages.length - 1];
  return last?.role === "assistant" ? last : null;
}

function onEvent(event: StreamEvent) {
  const reply = lastAssistant();
  if (!reply) return;
  switch (event.event) {
    case "chunk":
      reply.text = event.data.accumulated;
      break;
    case "done":
      reply.text = event.data.full_text;
      break;
    case "error":
      error = event.data.message;
      break;
    case "tool_call_start":
      reply.toolCalls.push({
        id: event.data.tool_call.tool_call_id,
        name: event.data.tool_call.tool_display_name || event.data.tool_call.tool_name,
        status: "in_progress",
      });
      break;
    case "tool_call_done": {
      const call = reply.toolCalls.find((c) => c.id === event.data.tool_call_id);
      if (call) call.status = event.data.error ? "failed" : "completed";
      break;
    }
  }
}

async function send(text: string) {
  const message = text.trim();
  if (!message || running) return;
  error = null;
  messages.push({ id: nextId++, role: "user", text: message });
  messages.push({ id: nextId++, role: "assistant", text: "", toolCalls: [] });
  running = true;
  try {
    await settingsAgentSend(message, onEvent);
  } catch (e) {
    error = String(e);
  } finally {
    running = false;
  }
}

async function cancel() {
  await settingsAgentCancel();
}

async function reset() {
  await settingsAgentReset();
  messages = [];
  error = null;
}

async function setPanelOpen(open: boolean) {
  panelOpen = open;
  panelMode = open ? await settingsAgentPanel(true) : "overlay";
  if (!open) await settingsAgentPanel(false);
}

export function getSettingsAgent() {
  return {
    get messages() { return messages; },
    get running() { return running; },
    get error() { return error; },
    get panelOpen() { return panelOpen; },
    get panelMode() { return panelMode; },
    send,
    cancel,
    reset,
    setPanelOpen,
  };
}
