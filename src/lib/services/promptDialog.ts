import { invoke } from "@tauri-apps/api/core";

export async function openPromptDialog(
  promptId: string,
  promptName: string,
  historyEntryId?: string,
  lastInteractionOnly?: boolean,
): Promise<void> {
  await invoke("open_prompt_dialog", {
    promptId,
    promptName,
    historyEntryId: historyEntryId ?? null,
    lastInteractionOnly: lastInteractionOnly ?? false,
  });
}
