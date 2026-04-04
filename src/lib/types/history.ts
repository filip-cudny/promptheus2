export type HistoryEntryType = "speech" | "text";

export interface SerializedConversationTurn {
  turn_number: number;
  message_text: string;
  message_image_paths: string[];
  output_text: string | null;
  is_complete: boolean;
  output_versions: string[];
  current_version_index: number;
}

import type { NodeUpdate } from "$lib/types/ai";

export interface SerializedConversationNode {
  node_id: string;
  parent_id: string | null;
  role: string;
  content: string;
  image_paths: string[];
  text_attachments: string[];
  timestamp: string;
  children: string[];
  updates: NodeUpdate[];
}

export interface ConversationHistoryData {
  context_text: string;
  context_image_paths: string[];
  turns: SerializedConversationTurn[];
  skill_id: string | null;
  skill_name: string | null;
  nodes: SerializedConversationNode[];
  root_node_id: string | null;
  current_path: string[];
  resolved_environment_section: string | null;
}

export interface LastInteractionData {
  last_text: HistoryEntry | null;
  last_speech: HistoryEntry | null;
}

export interface HistoryEntry {
  id: string;
  timestamp: string;
  input_content: string;
  entry_type: HistoryEntryType;
  output_content: string | null;
  skill_id: string | null;
  success: boolean;
  error: string | null;
  is_multi_turn: boolean;
  skill_name: string | null;
  conversation_data: ConversationHistoryData | null;
  created_at: string | null;
  updated_at: string | null;
  quick_action: boolean;
  title: string | null;
}
