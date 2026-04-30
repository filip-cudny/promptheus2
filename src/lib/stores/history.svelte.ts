import { listen } from "@tauri-apps/api/event";
import { info as logInfo, warn as logWarn } from "@tauri-apps/plugin-log";
import {
  getHistory,
  getHistoryEntry,
  getLastInteraction,
  updateHistoryRendered,
} from "$lib/services/history";
import { buildUserNodeDisplay, isSkillXml } from "$lib/utils/skillDisplay";
import type { HistoryEntry, LastInteractionData } from "$lib/types";

const BACKFILL_BATCH_LIMIT = 50;

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
  void backfillRenderedContent();
}

const BARE_SLASH_COMMAND_RE = /^\/[a-z0-9-]+\s*$/;

async function backfillRenderedContent(): Promise<void> {
  const candidates = entries.filter(
    (e) =>
      e.skill_id !== null &&
      (e.input_content_rendered === null ||
        isSkillXml(e.input_content_rendered) ||
        BARE_SLASH_COMMAND_RE.test(e.input_content_rendered)),
  );
  if (candidates.length === 0) return;

  const batch = candidates.slice(0, BACKFILL_BATCH_LIMIT);
  let migrated = 0;

  for (const entry of batch) {
    try {
      const { inputRendered, outputRendered } = await renderForEntry(entry);
      await updateHistoryRendered(entry.id, inputRendered, outputRendered);
      migrated++;
    } catch (e) {
      logWarn(`backfill failed for entry ${entry.id}: ${e}`);
    }
  }

  if (migrated > 0) {
    logInfo(`history rendered backfill: migrated ${migrated} entries`);
  }
}

async function renderForEntry(
  entry: HistoryEntry,
): Promise<{ inputRendered: string | null; outputRendered: string | null }> {
  if (!entry.is_multi_turn) {
    return {
      inputRendered: buildUserNodeDisplay(entry.input_content, []),
      outputRendered: entry.output_content,
    };
  }

  const full = await getHistoryEntry(entry.id);
  const nodes = full?.conversation_data?.nodes ?? [];
  const lastUser = [...nodes].reverse().find((n) => n.role === "user");
  const lastAssistant = [...nodes].reverse().find((n) => n.role === "assistant");
  return {
    inputRendered: lastUser
      ? buildUserNodeDisplay(lastUser.content, lastUser.text_attachments)
      : buildUserNodeDisplay(entry.input_content, []),
    outputRendered: lastAssistant?.content ?? entry.output_content,
  };
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
