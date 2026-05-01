import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import type { SearchResponse, SearchResult } from "$lib/types/historySearch";

const DEBOUNCE_MS = 120;
const RESULT_LIMIT = 30;

export function useHistorySearch() {
  let query = $state("");
  let results = $state<SearchResult[]>([]);
  let isLoading = $state(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight: AbortController | null = null;

  async function runSearch() {
    if (inflight) inflight.abort();
    const ac = new AbortController();
    inflight = ac;
    isLoading = true;
    try {
      const response = await invoke<SearchResponse>("search_history", {
        query: {
          query: query.trim(),
          type_filter: "chat",
          status_filter: "all",
          skill_ids: [],
          date_from: null,
          limit: RESULT_LIMIT,
          offset: 0,
        },
      });
      if (!ac.signal.aborted) {
        results = response.results;
      }
    } catch (e) {
      if (!ac.signal.aborted) {
        logError(`search_history (chat-palette) failed: ${e}`);
      }
    } finally {
      if (inflight === ac) inflight = null;
      if (!ac.signal.aborted) isLoading = false;
    }
  }

  function search(q: string) {
    query = q;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(runSearch, q.trim() === "" ? 0 : DEBOUNCE_MS);
  }

  function cancel() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    if (inflight) {
      inflight.abort();
      inflight = null;
    }
  }

  function clear() {
    query = "";
    results = [];
    cancel();
  }

  return {
    get query() {
      return query;
    },
    get results() {
      return results;
    },
    get isLoading() {
      return isLoading;
    },
    search,
    cancel,
    clear,
  };
}

export type HistorySearch = ReturnType<typeof useHistorySearch>;
