import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function useConversationDialogIpc(opts: {
  selfTarget: string;
  onRestoreHistory: (entryId: string, lastInteractionOnly?: boolean) => void;
  onVoiceInput: (text: string, autoSend: boolean) => void;
  onOpenForSkill: (skillId: string, skillName: string, skillModel: string | null) => void;
  onNewConversation: () => void;
  onActiveProviderCleared: () => void;
  onMenuReloadActive: () => void;
  onContextChanged: () => void;
}) {
  let unlistens: UnlistenFn[] = [];

  async function init() {
    const u1 = await listen<{ entry_id: string; last_interaction_only?: boolean }>(
      "restore-history",
      (event) => opts.onRestoreHistory(event.payload.entry_id, event.payload.last_interaction_only),
      { target: opts.selfTarget },
    );

    const u2 = await listen<{ text: string; auto_send: boolean }>(
      "voice-input",
      (event) => opts.onVoiceInput(event.payload.text, event.payload.auto_send),
      { target: opts.selfTarget },
    );

    const u3 = await listen<{ skill_id: string; skill_name: string; skill_model: string | null }>(
      "open-for-skill",
      (event) =>
        opts.onOpenForSkill(event.payload.skill_id, event.payload.skill_name, event.payload.skill_model),
      { target: opts.selfTarget },
    );

    const u4 = await listen<{ reason: string }>(
      "new-conversation",
      () => opts.onNewConversation(),
      { target: opts.selfTarget },
    );

    const u5 = await listen<{ provider_id: string | null }>(
      "shell:active-changed",
      (event) => {
        if (event.payload.provider_id === null) opts.onActiveProviderCleared();
      },
      { target: opts.selfTarget },
    );

    const u6 = await listen(
      "menu:reload-active",
      () => opts.onMenuReloadActive(),
      { target: opts.selfTarget },
    );

    const u7 = await listen("context-changed", () => opts.onContextChanged());

    unlistens = [u1, u2, u3, u4, u5, u6, u7];
  }

  function destroy() {
    for (const fn of unlistens) fn();
    unlistens = [];
  }

  return { init, destroy };
}

export type ConversationDialogIpc = ReturnType<typeof useConversationDialogIpc>;
