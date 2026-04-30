import { listen } from "@tauri-apps/api/event";
import {
  getHistory,
  getLastInteraction,
} from "$lib/services/history";
import type { HistoryEntry, LastInteractionData } from "$lib/types";

let entries = $state.raw<HistoryEntry[]>([]);
let lastInteraction = $state.raw<LastInteractionData>({
  last_text: null,
  last_speech: null,
});
let initialized = $state(false);
let unlisten: (() => void) | null = null;

const count = $derived(entries.length);
const isEmpty = $derived(entries.length === 0);
const lastTextEntry = $derived(lastInteraction.last_text);
const lastSpeechEntry = $derived(lastInteraction.last_speech);

async function refresh() {
  const [history, interaction] = await Promise.all([
    getHistory(),
    getLastInteraction(),
  ]);
  entries = history;
  lastInteraction = interaction;
}

async function init() {
  unlisten = await listen("history-changed", () => {
    refresh();
  });
  await refresh();
  initialized = true;
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

export function getHistoryStore() {
  return {
    get entries() {
      return entries;
    },
    get count() {
      return count;
    },
    get isEmpty() {
      return isEmpty;
    },
    get lastTextEntry() {
      return lastTextEntry;
    },
    get lastSpeechEntry() {
      return lastSpeechEntry;
    },
    get initialized() {
      return initialized;
    },
    init,
    refresh,
    destroy,
  };
}
