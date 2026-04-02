import { invoke, Channel } from "@tauri-apps/api/core";
import type { StreamEvent, ProcessedMessage } from "$lib/types/ai";

export interface ExecutionCallbacks {
  onChunk: (delta: string, accumulated: string) => void;
  onDone: (fullText: string) => void;
  onError: (message: string) => void;
}

export async function executeConversationTurn(
  messages: ProcessedMessage[],
  callbacks: ExecutionCallbacks,
  options?: {
    modelId?: string;
    promptId?: string;
    promptName?: string;
    skipClipboardCopy?: boolean;
  },
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = (event) => {
    switch (event.event) {
      case "chunk":
        callbacks.onChunk(event.data.delta, event.data.accumulated);
        break;
      case "done":
        callbacks.onDone(event.data.full_text);
        break;
      case "error":
        callbacks.onError(event.data.message);
        break;
    }
  };
  return invoke("execute_conversation_turn", {
    messages,
    modelId: options?.modelId ?? null,
    promptId: options?.promptId ?? null,
    promptName: options?.promptName ?? null,
    skipClipboardCopy: options?.skipClipboardCopy ?? false,
    onEvent,
  });
}

export async function executeSkill(
  skillName: string,
  callbacks: ExecutionCallbacks,
  inputOverride?: string,
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = (event) => {
    switch (event.event) {
      case "chunk":
        callbacks.onChunk(event.data.delta, event.data.accumulated);
        break;
      case "done":
        callbacks.onDone(event.data.full_text);
        break;
      case "error":
        callbacks.onError(event.data.message);
        break;
    }
  };
  return invoke("execute_skill", {
    skillName,
    inputOverride: inputOverride ?? null,
    onEvent,
  });
}

export async function processSkillTemplate(
  skillName: string,
  contextText?: string,
): Promise<ProcessedMessage[]> {
  return invoke("process_skill_template", {
    skillName,
    contextText: contextText ?? null,
  });
}

export interface SystemPromptResult {
  messages: ProcessedMessage[];
  time_update: string | null;
}

export async function getSystemPrompt(
  contextText?: string,
  tabId?: string,
): Promise<SystemPromptResult> {
  return invoke("get_system_prompt", {
    contextText: contextText ?? null,
    tabId: tabId ?? null,
  });
}

export async function releaseConversationContext(
  tabId: string,
): Promise<void> {
  return invoke("release_conversation_context", { tabId });
}

export async function generateConversationTitle(
  userMessage: string,
): Promise<string> {
  return invoke("generate_conversation_title", { userMessage });
}

export async function getExecutionState(): Promise<{
  is_executing: boolean;
  execution_id: string | null;
}> {
  return invoke("get_execution_state");
}
