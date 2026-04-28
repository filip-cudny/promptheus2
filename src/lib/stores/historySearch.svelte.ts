import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import type {
  HistoryStatusFilter,
  HistoryTypeFilter,
  SearchResponse,
  SkillCount,
  TimeRangePreset,
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
          skill_ids: Array.from(skillFilter),
          date_from: dateFrom,
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

  function trigger() {
    if (debounceTimer) clearTimeout(debounceTimer);
    const wait = query.trim() === "" ? 0 : SEARCH_DEBOUNCE_MS;
    debounceTimer = setTimeout(runQuery, wait);
  }

  $effect(() => {
    query;
    typeFilter;
    statusFilter;
    skillFilter;
    timeRange;
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
    get dateFrom() {
      return dateFrom;
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
    refresh: runQuery,
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
