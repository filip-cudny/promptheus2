import { createSearchRunner } from "$lib/stores/historySearch.svelte";

const DEBOUNCE_MS = 120;
const RESULT_LIMIT = 30;

export function useHistorySearch() {
  let query = $state("");

  const runner = createSearchRunner({
    buildQuery: () => ({
      query: query.trim(),
      type_filter: "chat",
      status_filter: "all",
      skill_ids: [],
      date_from: null,
      limit: RESULT_LIMIT,
      offset: 0,
    }),
    debounceMs: DEBOUNCE_MS,
    label: "chat-palette",
  });

  function search(q: string) {
    query = q;
    runner.trigger();
  }

  function clear() {
    query = "";
    runner.clearResults();
    runner.cancel();
  }

  return {
    get query() {
      return query;
    },
    get results() {
      return runner.response.results;
    },
    get isLoading() {
      return runner.loading;
    },
    search,
    cancel: runner.cancel,
    clear,
  };
}

export type HistorySearch = ReturnType<typeof useHistorySearch>;
