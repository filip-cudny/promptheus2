export type HistoryEntryType = "speech" | "text";

import type { NodeUpdate, ToolCall } from "$lib/types/ai";
import type { ConversationImage } from "$lib/types/conversation";

export interface SerializedConversationNode {
  node_id: string;
  parent_id: string | null;
  role: string;
  content: string;
  text_attachments: string[];
  timestamp: string;
  children: string[];
  updates: NodeUpdate[];
  prompt_tokens?: number | null;
  completion_tokens?: number | null;
  thinking?: string | null;
  error?: string | null;
  cancelled?: boolean;
  tool_calls?: ToolCall[];
}

export interface ConversationHistoryData {
  context_text: string;
  nodes: SerializedConversationNode[];
  root_node_id: string | null;
  current_path: string[];
  resolved_environment_section: string | null;
  node_images: Record<string, ConversationImage[]>;
  context_images: ConversationImage[];
  model_id: string | null;
  reasoning_effort: string | null;
}

export interface ImagePayload {
  node_id: string | null;
  image_index: number;
  data: string;
  media_type: string;
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
