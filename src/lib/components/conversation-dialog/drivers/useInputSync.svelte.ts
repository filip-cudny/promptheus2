import { untrack } from "svelte";
import type { createConversationStore } from "$lib/stores/conversation.svelte";
import type { ConversationImage } from "$lib/types/conversation";
import type SkillEditable from "$lib/components/ui/SkillEditable.svelte";

type SkillEditableInstance = ReturnType<typeof SkillEditable>;

export function useInputSync(opts: {
  store: ReturnType<typeof createConversationStore>;
  getSkillEditable: () => SkillEditableInstance | undefined;
}) {
  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);
  let localTextAttachments = $state<string[]>([]);
  let syncedTabId = $state("");
  let lastDomText = $state("");

  $effect(() => {
    if (opts.store.activeTabId === syncedTabId) {
      opts.store.updateInputText(localText);
      lastDomText = localText;
    }
  });

  $effect(() => {
    if (opts.store.activeTabId === syncedTabId) {
      opts.store.updateInputImages(localImages);
    }
  });

  $effect(() => {
    if (opts.store.activeTabId === syncedTabId) {
      opts.store.updateInputTextAttachments(localTextAttachments);
    }
  });

  $effect(() => {
    const tabId = opts.store.activeTabId;
    const storeText = opts.store.inputText;
    const storeImages = opts.store.inputImages;
    const storeAttachments = opts.store.inputTextAttachments;

    const tabChanged = tabId !== untrack(() => syncedTabId);
    const textChangedExternally = storeText !== untrack(() => lastDomText);

    localText = storeText;
    localImages = storeImages;
    localTextAttachments = storeAttachments;
    syncedTabId = tabId;

    const editable = opts.getSkillEditable();
    if (!editable) return;

    if (storeText === "") {
      editable.setTextAndHighlight("");
      editable.resetUndoStack("");
      lastDomText = "";
    } else if (tabChanged || textChangedExternally) {
      editable.setTextAndHighlight(storeText);
      editable.resetUndoStack(storeText);
      lastDomText = storeText;
      requestAnimationFrame(() => {
        const el = opts.getSkillEditable();
        el?.focus();
        el?.restoreCursor(storeText.length);
      });
    }
  });

  return {
    get localText() {
      return localText;
    },
    set localText(v: string) {
      localText = v;
    },
    get localImages() {
      return localImages;
    },
    set localImages(v: ConversationImage[]) {
      localImages = v;
    },
    get localTextAttachments() {
      return localTextAttachments;
    },
    set localTextAttachments(v: string[]) {
      localTextAttachments = v;
    },
  };
}

export type InputSync = ReturnType<typeof useInputSync>;
