import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import { searchHistory } from "$lib/services/history";
import type {
  HistoryStatusFilter,
  HistoryTypeFilter,
  SearchQuery,
  SearchResponse,
  SkillCount,
  TimeRangePreset,
} from "$lib/types/historySearch";

const SEARCH_DEBOUNCE_MS = 150;

export function createSearchRunner(opts: {
  buildQuery: () => SearchQuery;
  debounceMs: number;
  label: string;
}) {
  let response = $state<SearchResponse>({ results: [], total: 0 });
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight: AbortController | null = null;

  async function run() {
    if (inflight) inflight.abort();
    const ac = new AbortController();
    inflight = ac;
    loading = true;
    try {
      const result = await searchHistory(opts.buildQuery());
      if (!ac.signal.aborted) response = result;
    } catch (e) {
      if (!ac.signal.aborted) {
        logError(`search_history (${opts.label}) failed: ${e}`);
      }
    } finally {
      if (inflight === ac) inflight = null;
      if (!ac.signal.aborted) loading = false;
    }
  }

  function trigger() {
    if (debounceTimer) clearTimeout(debounceTimer);
    const wait = opts.buildQuery().query === "" ? 0 : opts.debounceMs;
    debounceTimer = setTimeout(run, wait);
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

  function clearResults() {
    response = { results: [], total: 0 };
  }

  return {
    get response() {
      return response;
    },
    get loading() {
      return loading;
    },
    run,
    trigger,
    cancel,
    clearResults,
  };
}

export interface HistorySearchStoreOptions {
  pageSize: () => number;
  currentPage: () => number;
}

export function getHistorySearchStore(opts: HistorySearchStoreOptions) {
  let query = $state("");
  let typeFilter = $state<HistoryTypeFilter>("all");
  let statusFilter = $state<HistoryStatusFilter>("all");
  let skillFilter = $state<Set<string>>(new Set());
  let availableSkills = $state<SkillCount[]>([]);
  let timeRange = $state<TimeRangePreset>("all");

  let dateFrom = $derived.by<number | null>(() => {
    const now = Math.floor(Date.now() / 1000);
    switch (timeRange) {
      case "today": {
        const d = new Date();
        d.setHours(0, 0, 0, 0);
        return Math.floor(d.getTime() / 1000);
      }
      case "7d":
        return now - 7 * 86400;
      case "30d":
        return now - 30 * 86400;
      default:
        return null;
    }
  });

  const runner = createSearchRunner({
    buildQuery: () => ({
      query: query.trim(),
      type_filter: typeFilter,
      status_filter: statusFilter,
      skill_ids: Array.from(skillFilter),
      date_from: dateFrom,
      limit: opts.pageSize(),
      offset: opts.currentPage() * opts.pageSize(),
    }),
    debounceMs: SEARCH_DEBOUNCE_MS,
    label: "history-dialog",
  });

  async function refreshSkills() {
    try {
      const list = await invoke<SkillCount[]>("list_history_skills");
      availableSkills = list;
      if (skillFilter.size > 0) {
        const valid = new Set(list.map((s) => s.skill_id));
        let mutated = false;
        const next = new Set<string>();
        for (const id of skillFilter) {
          if (valid.has(id)) next.add(id);
          else mutated = true;
        }
        if (mutated) skillFilter = next;
      }
    } catch (e) {
      logError(`list_history_skills invoke failed: ${e}`);
    }
  }

  $effect(() => {
    query;
    typeFilter;
    statusFilter;
    skillFilter;
    timeRange;
    opts.pageSize();
    opts.currentPage();
    runner.trigger();
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
    get skillFilter() {
      return skillFilter;
    },
    get availableSkills() {
      return availableSkills;
    },
    toggleSkill(id: string) {
      const next = new Set(skillFilter);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      skillFilter = next;
    },
    clearSkills() {
      if (skillFilter.size > 0) skillFilter = new Set();
    },
    get timeRange() {
      return timeRange;
    },
    set timeRange(v: TimeRangePreset) {
      timeRange = v;
    },
    get results() {
      return runner.response.results;
    },
    get total() {
      return runner.response.total;
    },
    get loading() {
      return runner.loading;
    },
    get hasActiveFilters() {
      return (
        query.trim() !== "" ||
        typeFilter !== "all" ||
        statusFilter !== "all" ||
        skillFilter.size > 0 ||
        timeRange !== "all"
      );
    },
    get activeFilterCount() {
      let n = 0;
      if (query.trim() !== "") n++;
      if (typeFilter !== "all") n++;
      if (statusFilter !== "all") n++;
      if (skillFilter.size > 0) n++;
      if (timeRange !== "all") n++;
      return n;
    },
    refresh: runner.run,
    refreshSkills,
    clear() {
      query = "";
      typeFilter = "all";
      statusFilter = "all";
      skillFilter = new Set();
      timeRange = "all";
    },
  };
}

export type HistorySearchStore = ReturnType<typeof getHistorySearchStore>;
