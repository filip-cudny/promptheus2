import {
  getItems,
  getSelectedIndex,
  setSelectedIndex,
  isVisible,
  getOpenTrigger,
  isRecording,
  getRecordingSkillId,
  closeMenu,
  suppressClose,
  isSuppressed,
  resumeClose,
  isInBlurGrace,
  moveSelection,
  executeItem,
  executeSelected,
  startAlternativeExecution,
  handleNumberInput,
  getAllSkillItems,
  getSkillItems,
  isRecordingChat,
  toggleChatRecording,
  openDialogForItem,
  init,
  destroy,
} from "$lib/stores/contextMenu.svelte";

export function useContextMenu() {
  return {
    get items() {
      return getItems();
    },
    get selectedIndex() {
      return getSelectedIndex();
    },
    get visible() {
      return isVisible();
    },
    get openTrigger() {
      return getOpenTrigger();
    },
    get recording() {
      return isRecording();
    },
    get recordingSkillId() {
      return getRecordingSkillId();
    },
    get allSkillItems() {
      return getAllSkillItems();
    },
    get skillItems() {
      return getSkillItems();
    },
    get chatRecording() {
      return isRecordingChat();
    },
    setSelectedIndex,
    closeMenu,
    suppressClose,
    isSuppressed,
    resumeClose,
    isInBlurGrace,
    moveSelection,
    executeItem,
    executeSelected,
    startAlternativeExecution,
    handleNumberInput,
    toggleChatRecording,
    openDialogForItem,
    init,
    destroy,
  };
}

export type ContextMenuStore = ReturnType<typeof useContextMenu>;
