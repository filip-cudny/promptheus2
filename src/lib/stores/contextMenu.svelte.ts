import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { error } from "@tauri-apps/plugin-log";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { MenuItem } from "$lib/types/menu";
import { startExecution, isExecuting, getExecutingSkillId, cancelExecution } from "$lib/stores/execution.svelte";
import { openConversationDialogNewWindow } from "$lib/services/conversationDialog";

interface WorkArea {
  cursorX: number;
  cursorY: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
}

let _items = $state<MenuItem[]>([]);
let _selectedIndex = $state(-1);
let _visible = $state(false);
let _isRecording = $state(false);
let _isTranscribing = $state(false);
let _suppressClose = $state(false);
let _recordingSkillId = $state<string | null>(null);
let _openTrigger = $state(0);
let _openedAt = 0;
let _workArea: WorkArea | null = null;

const BLUR_GRACE_MS = 500;
let numberBuffer = "";
let numberTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;
let unlistenContextChanged: (() => void) | null = null;
let unlistenExecutionCompleted: (() => void) | null = null;
let unlistenHistoryChanged: (() => void) | null = null;
let unlistenRecordingStopped: (() => void) | null = null;
let unlistenTranscriptionComplete: (() => void) | null = null;
let unlistenSpeechError: (() => void) | null = null;

const NUMBER_DEBOUNCE_MS = 200;

const _navigableItems = $derived(
  _items.filter((item) => item.enabled),
);

const _allSkillItems = $derived(
  _items.filter((item) => item.item_type === "skill"),
);

const _skillItems = $derived(
  _allSkillItems.filter((item) => item.enabled),
);

function getItems(): MenuItem[] {
  return _items;
}

function getSelectedIndex(): number {
  return _selectedIndex;
}

function setSelectedIndex(index: number) {
  _selectedIndex = index;
}

function isVisible(): boolean {
  return _visible;
}

function getOpenTrigger(): number {
  return _openTrigger;
}

function isRecording(): boolean {
  return _isRecording;
}

function suppressClose() {
  _suppressClose = true;
}

function isSuppressed(): boolean {
  return _suppressClose;
}

function resumeClose() {
  _suppressClose = false;
}

function isInBlurGrace(): boolean {
  return Date.now() - _openedAt < BLUR_GRACE_MS;
}

function getRecordingSkillId(): string | null {
  return _recordingSkillId;
}

function applyItemStates(items: MenuItem[]): MenuItem[] {
  if (isExecuting()) {
    const executingId = getExecutingSkillId();
    return items.map((item) => {
      if (item.item_type !== "skill") return item;
      const data = item.data as { skill_id: string } | null;
      if (data?.skill_id === executingId) return item;
      return { ...item, enabled: false };
    });
  }
  if (_isRecording) {
    if (_recordingSkillId) {
      return items.map((item) => {
        if (item.item_type === "speech") return { ...item, enabled: false };
        if (item.item_type !== "skill") return item;
        const data = item.data as { skill_id: string } | null;
        if (data?.skill_id === _recordingSkillId) return item;
        return { ...item, enabled: false };
      });
    }
    return items.map((item) =>
      item.item_type === "skill" ? { ...item, enabled: false } : item,
    );
  }
  if (_isTranscribing) {
    return items.map((item) =>
      item.item_type === "skill" || item.item_type === "speech"
        ? { ...item, enabled: false }
        : item,
    );
  }
  return items;
}

async function fetchRecordingState(): Promise<void> {
  try {
    const state = await invoke<{
      is_recording: boolean;
      is_transcribing: boolean;
      action_id: string | null;
    }>("get_recording_state");
    _isRecording = state.is_recording;
    _isTranscribing = state.is_transcribing;
    _recordingSkillId = state.action_id;
  } catch (e) {
    error("Failed to fetch recording state: " + e);
  }
}

function clearRecordingState() {
  _isRecording = false;
  _isTranscribing = false;
  _recordingSkillId = null;
}

function getWorkArea(): WorkArea | null {
  return _workArea;
}

async function openMenu(workArea: WorkArea | null) {
  try {
    _workArea = workArea;
    await fetchRecordingState();
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = applyItemStates(fetched);
    _selectedIndex = -1;
    numberBuffer = "";
    _openedAt = Date.now();
    _visible = true;
    _openTrigger++;
  } catch (e) {
    error("Failed to open context menu: " + e);
  }
}

async function closeMenu() {
  if (!_visible) return;
  _visible = false;
  _items = [];
  _selectedIndex = -1;
  numberBuffer = "";

  await invoke("hide_context_menu_panel");
}

function moveSelection(direction: 1 | -1) {
  if (_navigableItems.length === 0) return;

  const currentItem = _selectedIndex >= 0 ? _items[_selectedIndex] : null;
  const currentNavIndex = currentItem
    ? _navigableItems.indexOf(currentItem)
    : -1;

  let nextNavIndex: number;
  if (currentNavIndex === -1) {
    const firstSkill = _navigableItems.find((item) => item.item_type === "skill");
    nextNavIndex = firstSkill ? _navigableItems.indexOf(firstSkill) : 0;
  } else {
    nextNavIndex =
      (currentNavIndex + direction + _navigableItems.length) %
      _navigableItems.length;
  }

  const targetItem = _navigableItems[nextNavIndex];
  _selectedIndex = _items.indexOf(targetItem);
}

async function executeItem(index: number, shiftPressed: boolean = false) {
  const item = _items[index];
  if (!item) return;

  const itemId = item.id;
  const itemType = item.item_type;
  const itemEnabled = item.enabled;

  if (itemType === "skill") {
    const data = item.data as { skill_id: string; skill_name: string } | null;
    if (!data?.skill_id) return;

    const skillId = data.skill_id;

    if (isExecuting() && skillId === getExecutingSkillId()) {
      await cancelExecution();
      return;
    }

    const isRecordingThis = _isRecording && _recordingSkillId === skillId;
    if (!itemEnabled && !isRecordingThis) return;

    if (isRecordingThis) {
      clearRecordingState();
      try {
        await invoke("execute_menu_item", {
          itemId,
          shiftPressed: true,
        });
      } catch (e) {
        error("Failed to stop speech recording for prompt: " + e);
      }
      await closeMenu();
      return;
    }

    if (shiftPressed) {
      _isRecording = true;
      _recordingSkillId = skillId;
      try {
        await invoke("execute_menu_item", {
          itemId,
          shiftPressed: true,
        });
      } catch (e) {
        error("Failed to start speech recording for prompt: " + e);
        clearRecordingState();
      }
      await refreshItems();
      return;
    }

    if (!itemEnabled) return;
    await closeMenu();
    startExecution(skillId);
    return;
  }

  if (itemType === "speech") {
    if (!itemEnabled) return;
    if (_isRecording && !_recordingSkillId) {
      clearRecordingState();
      try {
        await invoke("execute_menu_item", { itemId, shiftPressed: false });
      } catch (e) {
        error("Failed to stop speech recording: " + e);
      }
      await closeMenu();
    } else if (!_isRecording) {
      _isRecording = true;
      _recordingSkillId = null;
      try {
        await invoke("execute_menu_item", { itemId, shiftPressed: false });
      } catch (e) {
        error("Failed to start speech recording: " + e);
        clearRecordingState();
      }
      await refreshItems();
    }
    return;
  }

  if (!itemEnabled) return;

  try {
    await invoke("execute_menu_item", {
      itemId,
      shiftPressed,
    });
  } catch (e) {
    error("Failed to execute menu item: " + e);
  }

  await closeMenu();
}

async function executeSelected(shiftPressed: boolean = false) {
  if (_selectedIndex >= 0 && _selectedIndex < _items.length) {
    await executeItem(_selectedIndex, shiftPressed);
  }
}

async function startAlternativeExecution(index: number) {
  const item = _items[index];
  if (!item || item.item_type !== "skill") return;
  const data = item.data as { skill_id: string; skill_name: string } | null;
  if (!data?.skill_id) return;

  const skillId = data.skill_id;
  const itemId = item.id;
  const isRecordingThis = _isRecording && _recordingSkillId === skillId;
  if (!item.enabled && !isRecordingThis) return;

  if (isRecordingThis) {
    clearRecordingState();
    try {
      await invoke("execute_menu_item", {
        itemId,
        shiftPressed: true,
      });
    } catch (e) {
      error("Failed to stop speech recording for prompt: " + e);
    }
    await closeMenu();
    return;
  }

  _isRecording = true;
  _recordingSkillId = skillId;
  try {
    await invoke("execute_menu_item", {
      itemId,
      shiftPressed: true,
    });
  } catch (e) {
    error("Failed to start speech recording for prompt: " + e);
    clearRecordingState();
  }
  await refreshItems();
}

function handleNumberInput(digit: string, isAlternative: boolean) {
  if (numberTimer) clearTimeout(numberTimer);

  numberBuffer += digit;

  numberTimer = setTimeout(() => {
    const num = parseInt(numberBuffer, 10);
    numberBuffer = "";

    if (num >= 1 && num <= _allSkillItems.length) {
      const targetItem = _allSkillItems[num - 1];
      if (!targetItem.enabled && !(_isRecording && _recordingSkillId === (targetItem.data as { skill_id: string } | null)?.skill_id)) return;
      const targetIndex = _items.indexOf(targetItem);
      _selectedIndex = targetIndex;
      if (isAlternative) {
        startAlternativeExecution(targetIndex);
      } else {
        executeItem(targetIndex);
      }
    }
  }, NUMBER_DEBOUNCE_MS);
}

function clearNumberBuffer() {
  if (numberTimer) clearTimeout(numberTimer);
  numberBuffer = "";
}

const CHAT_RECORDING_ID = "__chat__";

function isRecordingChat(): boolean {
  return _isRecording && _recordingSkillId === CHAT_RECORDING_ID;
}

async function toggleChatRecording() {
  if (isRecordingChat()) {
    clearRecordingState();
    try {
      await invoke("execute_menu_item", {
        itemId: CHAT_RECORDING_ID,
        shiftPressed: true,
      });
    } catch (e) {
      error("Failed to stop chat speech recording: " + e);
    }
    await closeMenu();
    return;
  }

  _isRecording = true;
  _recordingSkillId = CHAT_RECORDING_ID;
  try {
    await invoke("execute_menu_item", {
      itemId: CHAT_RECORDING_ID,
      shiftPressed: true,
    });
  } catch (e) {
    error("Failed to start chat speech recording: " + e);
    clearRecordingState();
  }
  await refreshItems();
}

function getAllSkillItems(): MenuItem[] {
  return _allSkillItems;
}

function getSkillItems(): MenuItem[] {
  return _skillItems;
}

async function refreshItems() {
  if (!_visible) return;
  try {
    await fetchRecordingState();
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = applyItemStates(fetched);
  } catch (e) {
    error("Failed to refresh context menu: " + e);
  }
}

async function init() {
  const win = getCurrentWebviewWindow();
  unlisten = await win.listen<{
    cursor_x: number;
    cursor_y: number;
    work_x: number;
    work_y: number;
    work_width: number;
    work_height: number;
  }>("show-context-menu", (event) => {
    const p = event.payload;
    openMenu({
      cursorX: p.cursor_x,
      cursorY: p.cursor_y,
      workX: p.work_x,
      workY: p.work_y,
      workWidth: p.work_width,
      workHeight: p.work_height,
    });
  });
  unlistenContextChanged = await listen("context-changed", () => {
    refreshItems();
  });
  unlistenExecutionCompleted = await listen("execution-completed", () => {
    refreshItems();
  });
  unlistenHistoryChanged = await listen("history-changed", () => {
    refreshItems();
  });
  unlistenRecordingStopped = await listen("speech-recording-stopped", () => {
    _isRecording = false;
    _recordingSkillId = null;
    _isTranscribing = true;
    refreshItems();
  });
  unlistenTranscriptionComplete = await listen("speech-transcription-complete", () => {
    clearRecordingState();
    refreshItems();
  });
  unlistenSpeechError = await listen("speech-transcription-error", () => {
    clearRecordingState();
    refreshItems();
  });

  const prefetched = await invoke<MenuItem[]>("get_context_menu_items");
  _items = prefetched;
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  if (unlistenContextChanged) {
    unlistenContextChanged();
    unlistenContextChanged = null;
  }
  if (unlistenExecutionCompleted) {
    unlistenExecutionCompleted();
    unlistenExecutionCompleted = null;
  }
  if (unlistenHistoryChanged) {
    unlistenHistoryChanged();
    unlistenHistoryChanged = null;
  }
  if (unlistenRecordingStopped) {
    unlistenRecordingStopped();
    unlistenRecordingStopped = null;
  }
  if (unlistenTranscriptionComplete) {
    unlistenTranscriptionComplete();
    unlistenTranscriptionComplete = null;
  }
  if (unlistenSpeechError) {
    unlistenSpeechError();
    unlistenSpeechError = null;
  }
}

async function openDialogForItem(index: number) {
  const item = _items[index];
  if (!item || item.item_type !== "skill") return;
  const data = item.data as { skill_id: string; skill_name: string } | null;
  if (!data?.skill_id) return;
  const skillId = data.skill_id;
  const skillName = data.skill_name ?? item.label;
  let skillModel: string | null = null;
  try {
    const skill = await invoke<{ model?: string | null }>("get_skill", { name: skillId });
    skillModel = skill?.model ?? null;
  } catch (e) {
    error(`get_skill failed for ${skillId}: ${e}`);
  }
  await closeMenu();
  await openConversationDialogNewWindow(undefined, undefined, skillId, skillName, skillModel);
}

export {
  getItems,
  getSelectedIndex,
  setSelectedIndex,
  isVisible,
  isRecording,
  getRecordingSkillId,
  getWorkArea,
  getOpenTrigger,
  openMenu,
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
  clearNumberBuffer,
  getAllSkillItems,
  getSkillItems,
  isRecordingChat,
  toggleChatRecording,
  openDialogForItem,
  init,
  destroy,
};
