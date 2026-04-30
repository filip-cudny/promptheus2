import { invoke } from "@tauri-apps/api/core";
import type {
  HistoryEntry,
  HistoryEntryType,
  ImagePayload,
  SerializedConversationNode,
  LastInteractionData,
} from "$lib/types";

export async function getHistory(): Promise<HistoryEntry[]> {
  return invoke("get_history");
}

export async function getConversations(
  offset: number,
  limit: number,
): Promise<HistoryEntry[]> {
  return invoke("get_conversations", { offset, limit });
}

export async function getHistoryEntry(
  entryId: string,
): Promise<HistoryEntry | null> {
  return invoke("get_history_entry", { entryId });
}

export async function addHistoryEntry(params: {
  inputContent: string;
  entryType: HistoryEntryType;
  outputContent: string | null;
  skillId: string | null;
  success: boolean;
  error: string | null;
  isMultiTurn: boolean;
  skillName: string | null;
}): Promise<void> {
  return invoke("add_history_entry", params);
}

export async function addConversationEntry(params: {
  contextText: string;
  skillId: string | null;
  skillName: string | null;
  success: boolean;
  error: string | null;
  nodes: SerializedConversationNode[];
  rootNodeId: string | null;
  currentPath: string[];
  tabId: string | null;
  images: ImagePayload[];
  modelId: string | null;
  reasoningEffort: string | null;
}): Promise<string> {
  return invoke("add_conversation_entry", params);
}

export async function updateConversationEntry(params: {
  entryId: string;
  contextText: string;
  nodes: SerializedConversationNode[];
  rootNodeId: string | null;
  currentPath: string[];
  images: ImagePayload[];
  modelId: string | null;
  reasoningEffort: string | null;
}): Promise<void> {
  return invoke("update_conversation_entry", params);
}

export async function getLastInteraction(): Promise<LastInteractionData> {
  return invoke("get_last_interaction");
}

export async function updateHistoryEntryTitle(
  entryId: string,
  title: string,
): Promise<void> {
  return invoke("update_history_entry_title", { entryId, title });
}

export async function deleteHistoryEntry(
  entryId: string,
): Promise<void> {
  return invoke("delete_history_entry", { entryId });
}

export async function clearHistory(): Promise<void> {
  return invoke("clear_history");
}

export async function copyHistoryContent(content: string): Promise<void> {
  return invoke("copy_history_content", { content });
}
