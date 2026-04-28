import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import type {
  HistoryStatusFilter,
  HistoryTypeFilter,
  SearchResponse,
} from "$lib/types/historySearch";

const SEARCH_DEBOUNCE_MS = 150;

export interface HistorySearchStoreOptions {
  pageSize: () => number;
  currentPage: () => number;
}

export function getHistorySearchStore(opts: HistorySearchStoreOptions) {
  let query = $state("");
  let typeFilter = $state<HistoryTypeFilter>("all");
  let statusFilter = $state<HistoryStatusFilter>("all");

  let response = $state<SearchResponse>({ results: [], total: 0 });
  let loading = $state(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight: AbortController | null = null;

  async function runQuery() {
    if (inflight) inflight.abort();
    const ac = new AbortController();
    inflight = ac;
    loading = true;
    try {
      const offset = opts.currentPage() * opts.pageSize();
      const result = await invoke<SearchResponse>("search_history", {
        query: {
          query: query.trim(),
          type_filter: typeFilter,
          status_filter: statusFilter,
          limit: opts.pageSize(),
          offset,
        },
      });
      if (!ac.signal.aborted) {
        response = result;
      }
    } catch (e) {
      if (!ac.signal.aborted) {
        logError(`search_history invoke failed: ${e}`);
      }
    } finally {
      if (inflight === ac) inflight = null;
      if (!ac.signal.aborted) loading = false;
    }
  }

  function trigger() {
    if (debounceTimer) clearTimeout(debounceTimer);
    const wait = query.trim() === "" ? 0 : SEARCH_DEBOUNCE_MS;
    debounceTimer = setTimeout(runQuery, wait);
  }

  $effect(() => {
    query;
    typeFilter;
    statusFilter;
    opts.pageSize();
    opts.currentPage();
    trigger();
  });

  return {
    get query() {
      return query;
    },
    set query(v: string) {
      query = v;
    },
    get typeFilter() {
      return typeFilter;
    },
    set typeFilter(v: HistoryTypeFilter) {
      typeFilter = v;
    },
    get statusFilter() {
      return statusFilter;
    },
    set statusFilter(v: HistoryStatusFilter) {
      statusFilter = v;
    },
    get results() {
      return response.results;
    },
    get total() {
      return response.total;
    },
    get loading() {
      return loading;
    },
    get hasActiveFilters() {
      return query.trim() !== "" || typeFilter !== "all" || statusFilter !== "all";
    },
    refresh: runQuery,
    clear() {
      query = "";
      typeFilter = "all";
      statusFilter = "all";
    },
  };
}

export type HistorySearchStore = ReturnType<typeof getHistorySearchStore>;
