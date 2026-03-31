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
  input_content: string;
  entry_type: HistoryEntryType;
  output_content: string | null;
  prompt_id: string | null;
  success: boolean;
  error: string | null;
  is_multi_turn: boolean;
  prompt_name: string | null;
}): Promise<void> {
  return invoke("add_history_entry", params);
}

export async function addConversationEntry(params: {
  turns: SerializedConversationTurn[];
  context_text: string;
  context_image_paths: string[];
  prompt_id: string | null;
  prompt_name: string | null;
  success: boolean;
  error: string | null;
  nodes: SerializedConversationNode[];
  root_node_id: string | null;
  current_path: string[];
}): Promise<string> {
  return invoke("add_conversation_entry", params);
}

export async function updateConversationEntry(params: {
  entry_id: string;
  turns: SerializedConversationTurn[];
  context_text: string;
  context_image_paths: string[];
  nodes: SerializedConversationNode[];
  root_node_id: string | null;
  current_path: string[];
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
