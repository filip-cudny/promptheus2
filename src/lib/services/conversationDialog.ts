import { invoke } from "@tauri-apps/api/core";

export async function openConversationDialog(
  skillId: string,
  skillName: string,
  historyEntryId?: string,
  lastInteractionOnly?: boolean,
  initialInput?: string,
  autoSendInput?: boolean,
  newChat?: boolean,
  skillModel?: string | null,
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
  });
}

export async function focusOrOpenChat(): Promise<void> {
  await invoke("focus_or_open_chat");
}

export async function openConversationDialogNewWindow(
  sourceLabel?: string,
  providerId?: string,
  skillId?: string,
  skillName?: string,
  skillModel?: string | null,
): Promise<void> {
  await invoke("open_conversation_dialog_new_window", {
    sourceLabel: sourceLabel ?? null,
    providerId: providerId ?? null,
    skillId: skillId ?? null,
    skillName: skillName ?? null,
    skillModel: skillModel ?? null,
  });
}
