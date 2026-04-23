import { invoke } from "@tauri-apps/api/core";

export async function openConversationDialog(
  skillId: string,
  skillName: string,
  historyEntryId?: string,
  lastInteractionOnly?: boolean,
  initialInput?: string,
  autoSendInput?: boolean,
  newChat?: boolean,
): Promise<void> {
  await invoke("open_conversation_dialog", {
    skillId,
    skillName,
    historyEntryId: historyEntryId ?? null,
    lastInteractionOnly: lastInteractionOnly ?? false,
    initialInput: initialInput ?? null,
    autoSendInput: autoSendInput ?? false,
    newChat: newChat ?? false,
  });
}
