import { listen } from "@tauri-apps/api/event";
import { listSkills, listSkillsFull } from "$lib/services/skills";
import type { SkillFull, SkillSummary } from "$lib/types";

let summaries = $state.raw<SkillSummary[]>([]);
let full = $state.raw<SkillFull[]>([]);
let initialized = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let unlisten: (() => void) | null = null;
let refreshInFlight: Promise<void> | null = null;

const nameSet = $derived(new Set(summaries.map((s) => s.name)));

async function refresh(): Promise<void> {
  if (refreshInFlight) return refreshInFlight;
  loading = true;
  refreshInFlight = (async () => {
    try {
      const [s, f] = await Promise.all([listSkills(), listSkillsFull()]);
      summaries = s;
      full = f;
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
      refreshInFlight = null;
    }
  })();
  return refreshInFlight;
}

async function init(): Promise<void> {
  if (initialized) return;
  initialized = true;
  if (!unlisten) {
    unlisten = await listen("skills-changed", () => {
      refresh();
    });
  }
  await refresh();
}

function destroy(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  initialized = false;
}

export function getSkillsStore() {
  return {
    get items() {
      return summaries;
    },
    get full() {
      return full;
    },
    get nameSet() {
      return nameSet;
    },
    get initialized() {
      return initialized;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },
    init,
    refresh,
    destroy,
  };
}
