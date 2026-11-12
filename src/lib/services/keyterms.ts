import { invoke } from "@tauri-apps/api/core";
import { trackedInvoke } from "$lib/services/trackedInvoke";
import type { SttKeytermsDoc } from "$lib/types";

export async function getSttKeyterms(): Promise<SttKeytermsDoc> {
  return invoke("get_stt_keyterms");
}

export async function saveSttKeyterms(content: string): Promise<SttKeytermsDoc> {
  return trackedInvoke("save_stt_keyterms", { content });
}
