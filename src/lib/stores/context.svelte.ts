import { listen } from "@tauri-apps/api/event";
import { getContextItems } from "$lib/services/context";
import type { ContextItem } from "$lib/types/context";

let items = $state.raw<ContextItem[]>([]);
let initialized = $state(false);
let unlisten: (() => void) | null = null;

const hasText = $derived(items.some((i) => i.item_type === "text"));
const hasImages = $derived(items.some((i) => i.item_type === "image"));
const isEmpty = $derived(items.length === 0);
const itemCount = $derived(items.length);

async function refresh() {
  items = await getContextItems();
}

async function init() {
  unlisten = await listen("context-changed", () => {
    refresh();
  });
  initialized = true;
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

export function getContextStore() {
  return {
    get items() {
      return items;
    },
    get hasText() {
      return hasText;
    },
    get hasImages() {
      return hasImages;
    },
    get isEmpty() {
      return isEmpty;
    },
    get itemCount() {
      return itemCount;
    },
    get initialized() {
      return initialized;
    },
    refresh,
    init,
    destroy,
  };
}
