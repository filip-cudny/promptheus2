import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { error } from "@tauri-apps/plugin-log";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { MenuItem } from "$lib/types/menu";
import { startExecution, isExecuting } from "$lib/stores/execution.svelte";
import { openPromptDialog } from "$lib/services/promptDialog";

let _items = $state<MenuItem[]>([]);
let _selectedIndex = $state(-1);
let _visible = $state(false);
let _isRecording = $state(false);
let _recordingPromptId = $state<string | null>(null);
let numberBuffer = "";
let numberTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;
let unlistenContextChanged: (() => void) | null = null;
let unlistenExecutionCompleted: (() => void) | null = null;
let unlistenHistoryChanged: (() => void) | null = null;
let unlistenRecordingStopped: (() => void) | null = null;
let unlistenTranscriptionComplete: (() => void) | null = null;
let unlistenSpeechError: (() => void) | null = null;

const NUMBER_DEBOUNCE_MS = 300;

const _navigableItems = $derived(
  _items.filter((item) => item.enabled),
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

function isRecording(): boolean {
  return _isRecording;
}

function getRecordingPromptId(): string | null {
  return _recordingPromptId;
}

function applyItemStates(items: MenuItem[]): MenuItem[] {
  if (isExecuting()) {
    return items.map((item) =>
      item.item_type === "prompt" ? { ...item, enabled: false } : item,
    );
  }
  if (_isRecording && _recordingPromptId) {
    return items.map((item) => {
      if (item.item_type !== "prompt") return item;
      const data = item.data as { prompt_id: string } | null;
      if (data?.prompt_id === _recordingPromptId) return item;
      return { ...item, enabled: false };
    });
  }
  return items;
}

async function fetchRecordingState(): Promise<void> {
  try {
    const state = await invoke<{ is_recording: boolean; action_id: string | null }>(
      "get_recording_state",
    );
    _isRecording = state.is_recording;
    _recordingPromptId = state.action_id;
  } catch (e) {
    error("Failed to fetch recording state: " + e);
  }
}

function clearRecordingState() {
  _isRecording = false;
  _recordingPromptId = null;
}

async function openMenu() {
  try {
    await fetchRecordingState();
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = applyItemStates(fetched);
    _selectedIndex = -1;
    numberBuffer = "";
    _visible = true;
  } catch (e) {
    error("Failed to open context menu: " + e);
  }
}

async function closeMenu() {
  _visible = false;
  _items = [];
  _selectedIndex = -1;
  numberBuffer = "";

  const win = getCurrentWebviewWindow();
  await win.hide();
}

function moveSelection(direction: 1 | -1) {
  if (_navigableItems.length === 0) return;

  const currentItem = _selectedIndex >= 0 ? _items[_selectedIndex] : null;
  const currentNavIndex = currentItem
    ? _navigableItems.indexOf(currentItem)
    : -1;

  let nextNavIndex: number;
  if (currentNavIndex === -1) {
    nextNavIndex = direction === 1 ? 0 : _navigableItems.length - 1;
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

  if (item.item_type === "prompt") {
    const data = item.data as { prompt_id: string; prompt_name: string } | null;
    if (!data?.prompt_id) return;

    const isRecordingThis = _isRecording && _recordingPromptId === data.prompt_id;
    if (!item.enabled && !isRecordingThis) return;

    if (shiftPressed) {
      if (_isRecording && _recordingPromptId === data.prompt_id) {
        clearRecordingState();
      } else {
        _isRecording = true;
        _recordingPromptId = data.prompt_id;
      }
      try {
        await invoke("execute_menu_item", {
          itemId: item.id,
          shiftPressed: true,
        });
      } catch (e) {
        error("Failed to start speech recording for prompt: " + e);
        clearRecordingState();
      }
      await closeMenu();
      return;
    }

    if (!item.enabled) return;
    await closeMenu();
    startExecution(data.prompt_id);
    return;
  }

  if (!item.enabled) return;

  try {
    await invoke("execute_menu_item", {
      itemId: item.id,
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
  if (!item || item.item_type !== "prompt") return;
  const data = item.data as { prompt_id: string; prompt_name: string } | null;
  if (!data?.prompt_id) return;

  const isRecordingThis = _isRecording && _recordingPromptId === data.prompt_id;
  if (!item.enabled && !isRecordingThis) return;

  if (isRecordingThis) {
    clearRecordingState();
  } else {
    _isRecording = true;
    _recordingPromptId = data.prompt_id;
  }

  try {
    await invoke("execute_menu_item", {
      itemId: item.id,
      shiftPressed: true,
    });
  } catch (e) {
    error("Failed to start speech recording for prompt: " + e);
    clearRecordingState();
  }

  await refreshItems();
}

function handleNumberInput(digit: string) {
  if (numberTimer) clearTimeout(numberTimer);

  numberBuffer += digit;

  numberTimer = setTimeout(() => {
    const num = parseInt(numberBuffer, 10);
    numberBuffer = "";

    if (num >= 1 && num <= _navigableItems.length) {
      const targetItem = _navigableItems[num - 1];
      const targetIndex = _items.indexOf(targetItem);
      _selectedIndex = targetIndex;
      executeItem(targetIndex);
    }
  }, NUMBER_DEBOUNCE_MS);
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
  unlisten = await win.listen("show-context-menu", () => {
    openMenu();
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
    clearRecordingState();
    refreshItems();
  });
  unlistenTranscriptionComplete = await listen("speech-transcription-complete", () => {
    clearRecordingState();
    refreshItems();
  });
  unlistenSpeechError = await listen("speech-error", () => {
    clearRecordingState();
    refreshItems();
  });
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
  if (!item || item.item_type !== "prompt") return;
  const data = item.data as { prompt_id: string; prompt_name: string } | null;
  if (!data?.prompt_id) return;
  await closeMenu();
  await openPromptDialog(data.prompt_id, data.prompt_name ?? item.label);
}

export {
  getItems,
  getSelectedIndex,
  setSelectedIndex,
  isVisible,
  isRecording,
  getRecordingPromptId,
  openMenu,
  closeMenu,
  moveSelection,
  executeItem,
  executeSelected,
  startAlternativeExecution,
  handleNumberInput,
  openDialogForItem,
  init,
  destroy,
};
