import { invoke } from "@tauri-apps/api/core";
import type {
  HistoryEntry,
  HistoryEntryType,
  SerializedConversationTurn,
  SerializedConversationNode,
  LastInteractionData,
} from "$lib/types";

export async function getHistory(): Promise<HistoryEntry[]> {
  return invoke("get_history");
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
  promptId: string | null;
  success: boolean;
  error: string | null;
  isMultiTurn: boolean;
  promptName: string | null;
}): Promise<void> {
  return invoke("add_history_entry", params);
}

export async function addConversationEntry(params: {
  turns: SerializedConversationTurn[];
  contextText: string;
  contextImagePaths: string[];
  promptId: string | null;
  promptName: string | null;
  success: boolean;
  error: string | null;
  nodes: SerializedConversationNode[];
  rootNodeId: string | null;
  currentPath: string[];
}): Promise<string> {
  return invoke("add_conversation_entry", params);
}

export async function updateConversationEntry(params: {
  entryId: string;
  turns: SerializedConversationTurn[];
  contextText: string;
  contextImagePaths: string[];
  nodes: SerializedConversationNode[];
  rootNodeId: string | null;
  currentPath: string[];
}): Promise<void> {
  return invoke("update_conversation_entry", params);
}

export async function getLastInteraction(): Promise<LastInteractionData> {
  return invoke("get_last_interaction");
}

export async function clearHistory(): Promise<void> {
  return invoke("clear_history");
}

export async function copyHistoryContent(content: string): Promise<void> {
  return invoke("copy_history_content", { content });
}
