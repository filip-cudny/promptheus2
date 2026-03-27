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

export interface SerializedConversationNode {
  node_id: string;
  parent_id: string | null;
  role: string;
  content: string;
  image_paths: string[];
  timestamp: string;
  children: string[];
}

export interface ConversationHistoryData {
  context_text: string;
  context_image_paths: string[];
  turns: SerializedConversationTurn[];
  prompt_id: string | null;
  prompt_name: string | null;
  nodes: SerializedConversationNode[];
  root_node_id: string | null;
  current_path: string[];
}

export interface HistoryEntry {
  id: string;
  timestamp: string;
  input_content: string;
  entry_type: HistoryEntryType;
  output_content: string | null;
  prompt_id: string | null;
  success: boolean;
  error: string | null;
  is_conversation: boolean;
  prompt_name: string | null;
  conversation_data: ConversationHistoryData | null;
  created_at: string | null;
  updated_at: string | null;
}
