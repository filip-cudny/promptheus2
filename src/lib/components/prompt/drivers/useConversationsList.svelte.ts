import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getConversations } from "$lib/services/history";
import type { HistoryEntry } from "$lib/types";

export function useConversationsList(pageSize: number) {
  let conversations = $state<HistoryEntry[]>([]);
  let hasMore = $state(true);
  let loading = $state(false);
  let unlisten: UnlistenFn | undefined;

  async function fetchPage(offset: number): Promise<HistoryEntry[]> {
    return getConversations(offset, pageSize);
  }

  async function reload() {
    if (loading) return;
    loading = true;
    try {
      const page = await fetchPage(0);
      conversations = page;
      hasMore = page.length >= pageSize;
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (loading || !hasMore) return;
    loading = true;
    try {
      const page = await fetchPage(conversations.length);
      conversations = [...conversations, ...page];
      hasMore = page.length >= pageSize;
    } finally {
      loading = false;
    }
  }

  async function init(): Promise<() => void> {
    await reload();
    unlisten = await listen("history-changed", () => {
      if (loading) return;
      reload();
    });
    return () => {
      unlisten?.();
      unlisten = undefined;
    };
  }

  return {
    get conversations() {
      return conversations;
    },
    get hasMore() {
      return hasMore;
    },
    get loading() {
      return loading;
    },
    init,
    loadMore,
    reload,
  };
}

export type ConversationsList = ReturnType<typeof useConversationsList>;
