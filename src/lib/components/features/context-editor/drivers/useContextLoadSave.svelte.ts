import {
  appendContext,
  appendContextImage,
  clearContext,
  getContextItems,
} from "$lib/services/context";
import type { ConversationImage } from "$lib/types/conversation";

export function useContextLoadSave() {
  let saving = $state(false);
  let errorMessage = $state("");

  async function load(): Promise<{ text: string; images: ConversationImage[] }> {
    let text = "";
    const images: ConversationImage[] = [];
    const items = await getContextItems();
    for (const item of items) {
      if (item.item_type === "text") {
        text += (text ? "\n" : "") + item.content;
      } else if (item.item_type === "image") {
        images.push({ data: item.data, media_type: item.media_type });
      }
    }
    return { text, images };
  }

  async function save(text: string, images: ConversationImage[]): Promise<void> {
    saving = true;
    errorMessage = "";
    try {
      await clearContext();
      for (const img of images) await appendContextImage(img.data, img.media_type);
      if (text.trim()) await appendContext(text);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
      console.error("Failed to save context:", e);
    } finally {
      saving = false;
    }
  }

  return {
    get saving() {
      return saving;
    },
    get errorMessage() {
      return errorMessage;
    },
    load,
    save,
  };
}

export type ContextLoadSave = ReturnType<typeof useContextLoadSave>;
