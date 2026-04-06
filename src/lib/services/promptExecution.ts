import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  StreamEvent,
  ConversationNodeForExecution,
  ImageData,
  ToolCall,
} from "$lib/types/ai";

import type { NodeUpdate } from "$lib/types/ai";

export interface TokenUsageData {
  prompt_tokens: number | null;
  completion_tokens: number | null;
}

export interface ExecutionCallbacks {
  onChunk: (delta: string, accumulated: string, thinkingDelta: string | null, accumulatedThinking: string | null) => void;
  onDone: (fullText: string, usage?: TokenUsageData, fullThinking?: string | null) => void;
  onError: (message: string) => void;
  onNodeUpdates?: (nodeId: string, updates: NodeUpdate[]) => void;
  onToolCallStart?: (toolCall: ToolCall) => void;
  onToolCallProgress?: (toolCallId: string, partialResult: string) => void;
  onToolCallDone?: (toolCallId: string, result: string | null, error: string | null) => void;
  onToolCallConfirmation?: (toolCallId: string) => void;
}

function routeStreamEvent(callbacks: ExecutionCallbacks): (event: StreamEvent) => void {
  return (event) => {
    switch (event.event) {
      case "chunk":
        callbacks.onChunk(event.data.delta, event.data.accumulated, event.data.thinking_delta, event.data.accumulated_thinking);
        break;
      case "done":
        callbacks.onDone(event.data.full_text, {
          prompt_tokens: event.data.prompt_tokens,
          completion_tokens: event.data.completion_tokens,
        }, event.data.full_thinking);
        break;
      case "error":
        callbacks.onError(event.data.message);
        break;
      case "node_updates":
        callbacks.onNodeUpdates?.(event.data.node_id, event.data.updates);
        break;
      case "tool_call_start":
        callbacks.onToolCallStart?.(event.data.tool_call);
        break;
      case "tool_call_progress":
        callbacks.onToolCallProgress?.(event.data.tool_call_id, event.data.partial_result);
        break;
      case "tool_call_done":
        callbacks.onToolCallDone?.(event.data.tool_call_id, event.data.result, event.data.error);
        break;
      case "tool_call_confirmation":
        callbacks.onToolCallConfirmation?.(event.data.tool_call_id);
        break;
    }
  };
}

export async function executeSkill(
  skillName: string,
  callbacks: ExecutionCallbacks,
  inputOverride?: string,
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = routeStreamEvent(callbacks);
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
    modelId?: string;
    reasoningEffort?: string;
    toolsOverride?: string[];
  },
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = routeStreamEvent(callbacks);
  return invoke("execute_conversation_from_tree", {
    nodes,
    contextText: options.contextText ?? null,
    contextImages: options.contextImages ?? [],
    tabId: options.tabId,
    skillId: options.skillId ?? null,
    skillName: options.skillName ?? null,
    modelId: options.modelId ?? null,
    reasoningEffort: options.reasoningEffort ?? null,
    toolsOverride: options.toolsOverride ?? null,
    onEvent,
  });
}

export interface ExecutionSnapshot {
  execution_id: string;
  user_message: string;
  accumulated_text: string;
  accumulated_thinking: string | null;
  tool_calls: ToolCall[];
  is_thinking: boolean;
  finished: boolean;
  error: string | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
}

export async function reconnectToExecution(
  callbacks: ExecutionCallbacks,
): Promise<ExecutionSnapshot | null> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = routeStreamEvent(callbacks);
  return invoke("reconnect_to_execution", { onEvent });
}
