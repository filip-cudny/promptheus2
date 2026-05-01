import { debug as logDebug } from "@tauri-apps/plugin-log";

type Hooks = {
  onSettingsClose?: () => void;
  onActionMenuClose?: () => void;
};

export function useFloatingPanelMutex(hooks: Hooks = {}) {
  let settingsOpen = $state(false);
  let settingsAnchor = $state<HTMLElement | undefined>();
  let actionMenuId = $state("");
  let actionMenuAnchor = $state<HTMLElement | undefined>();
  let chatProvidersOpen = $state(false);

  function closeSettings() {
    if (!settingsOpen) return;
    logDebug("[ctx-menu] closing settings panel");
    settingsOpen = false;
    settingsAnchor = undefined;
    hooks.onSettingsClose?.();
  }

  function closeActionMenu() {
    if (!actionMenuId) return;
    logDebug(`[ctx-menu] closing action menu: ${actionMenuId}`);
    actionMenuId = "";
    actionMenuAnchor = undefined;
    hooks.onActionMenuClose?.();
  }

  function closeChatProviders() {
    if (!chatProvidersOpen) return;
    chatProvidersOpen = false;
  }

  function closeAll() {
    closeSettings();
    closeActionMenu();
    closeChatProviders();
  }

  return {
    get settingsOpen() { return settingsOpen; },
    get settingsAnchor() { return settingsAnchor; },
    get actionMenuId() { return actionMenuId; },
    get actionMenuAnchor() { return actionMenuAnchor; },
    get chatProvidersOpen() { return chatProvidersOpen; },
    get hasAny() {
      return settingsOpen || actionMenuId !== "" || chatProvidersOpen;
    },
    openSettings(anchor: HTMLElement | undefined) {
      if (settingsOpen) {
        closeSettings();
        return;
      }
      closeAll();
      logDebug("[ctx-menu] opening settings panel");
      settingsAnchor = anchor;
      settingsOpen = true;
    },
    openActionMenu(id: string, anchor: HTMLElement) {
      if (actionMenuId === id) {
        closeActionMenu();
        return;
      }
      closeAll();
      actionMenuId = id;
      actionMenuAnchor = anchor;
    },
    openChatProviders() {
      if (chatProvidersOpen) {
        closeChatProviders();
        return;
      }
      closeAll();
      chatProvidersOpen = true;
    },
    closeSettings,
    closeActionMenu,
    closeChatProviders,
    closeAll,
  };
}

export type FloatingPanelMutex = ReturnType<typeof useFloatingPanelMutex>;
