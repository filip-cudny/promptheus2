import { invoke } from "@tauri-apps/api/core";
import type { InvocationSource } from "$lib/types/history";

export async function openConversationDialog(
  skillId: string,
  skillName: string,
  historyEntryId?: string,
  lastInteractionOnly?: boolean,
  initialInput?: string,
  autoSendInput?: boolean,
  newChat?: boolean,
  skillModel?: string | null,
  invocationSource?: InvocationSource,
): Promise<void> {
  await invoke("open_conversation_dialog", {
    skillId,
    skillName,
    skillModel: skillModel ?? null,
    historyEntryId: historyEntryId ?? null,
    lastInteractionOnly: lastInteractionOnly ?? false,
    initialInput: initialInput ?? null,
    autoSendInput: autoSendInput ?? false,
    newChat: newChat ?? false,
    invocationSource: invocationSource ?? null,
  });
}

export async function focusOrOpenChat(): Promise<void> {
  await invoke("focus_or_open_chat");
}
