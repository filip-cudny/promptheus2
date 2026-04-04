import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  StreamEvent,
  ConversationNodeForExecution,
  ImageData,
} from "$lib/types/ai";

import type { NodeUpdate } from "$lib/types/ai";

export interface TokenUsageData {
  prompt_tokens: number | null;
  completion_tokens: number | null;
}

export interface ExecutionCallbacks {
  onChunk: (delta: string, accumulated: string) => void;
  onDone: (fullText: string, usage?: TokenUsageData) => void;
  onError: (message: string) => void;
  onNodeUpdates?: (nodeId: string, updates: NodeUpdate[]) => void;
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
        callbacks.onDone(event.data.full_text, {
          prompt_tokens: event.data.prompt_tokens,
          completion_tokens: event.data.completion_tokens,
        });
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

export async function releaseConversationContext(
  tabId: string,
): Promise<void> {
  return invoke("release_conversation_context", { tabId });
}

export async function seedConversationContext(
  tabId: string,
  resolvedEnvironmentSection: string,
): Promise<void> {
  return invoke("seed_conversation_context", { tabId, resolvedEnvironmentSection });
}

export async function generateConversationTitle(
  userMessage: string,
): Promise<string> {
  return invoke("generate_conversation_title", { userMessage });
}

export interface ResolveSkillInputResult {
  resolved_text: string;
  had_skills: boolean;
}

export async function resolveSkillInput(
  text: string,
): Promise<ResolveSkillInputResult> {
  return invoke("resolve_skill_input", { text });
}

export async function executeConversationFromTree(
  nodes: ConversationNodeForExecution[],
  callbacks: ExecutionCallbacks,
  options: {
    contextText?: string;
    contextImages?: ImageData[];
    tabId: string;
    skillId?: string;
    skillName?: string;
  },
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = (event) => {
    switch (event.event) {
      case "chunk":
        callbacks.onChunk(event.data.delta, event.data.accumulated);
        break;
      case "done":
        callbacks.onDone(event.data.full_text, {
          prompt_tokens: event.data.prompt_tokens,
          completion_tokens: event.data.completion_tokens,
        });
        break;
      case "error":
        callbacks.onError(event.data.message);
        break;
      case "node_updates":
        callbacks.onNodeUpdates?.(event.data.node_id, event.data.updates);
        break;
    }
  };
  return invoke("execute_conversation_from_tree", {
    nodes,
    contextText: options.contextText ?? null,
    contextImages: options.contextImages ?? [],
    tabId: options.tabId,
    skillId: options.skillId ?? null,
    skillName: options.skillName ?? null,
    onEvent,
  });
}
