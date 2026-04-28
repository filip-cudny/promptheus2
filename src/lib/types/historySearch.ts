import type { HistoryEntry } from "$lib/types/history";

export type HistoryTypeFilter = "all" | "chat" | "quick_action" | "speech";
export type HistoryStatusFilter = "all" | "success" | "error";
export type SearchField = "title" | "skill_name" | "input_content" | "output_content";

export interface SearchQuery {
  query: string;
  type_filter: HistoryTypeFilter;
  status_filter: HistoryStatusFilter;
  limit: number;
  offset: number;
}

export interface FieldMatch {
  field: SearchField;
  indices: number[];
}

export interface SearchResult {
  entry: HistoryEntry;
  matches: FieldMatch[];
  score: number;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
}
