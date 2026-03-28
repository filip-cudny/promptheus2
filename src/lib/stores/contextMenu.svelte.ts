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
let numberBuffer = "";
let numberTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;
let unlistenContextChanged: (() => void) | null = null;
let unlistenExecutionCompleted: (() => void) | null = null;
let unlistenHistoryChanged: (() => void) | null = null;

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

function applyExecutionState(items: MenuItem[]): MenuItem[] {
  if (!isExecuting()) return items;
  return items.map((item) =>
    item.item_type === "prompt" ? { ...item, enabled: false } : item,
  );
}

async function openMenu() {
  try {
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = applyExecutionState(fetched);
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
  if (!item || !item.enabled) return;

  if (item.item_type === "prompt") {
    const data = item.data as { prompt_id: string; prompt_name: string } | null;
    if (data?.prompt_id) {
      await closeMenu();
      if (shiftPressed) {
        await openPromptDialog(data.prompt_id, data.prompt_name ?? item.label);
      } else {
        startExecution(data.prompt_id);
      }
      return;
    }
  }

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
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = applyExecutionState(fetched);
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
  openMenu,
  closeMenu,
  moveSelection,
  executeItem,
  executeSelected,
  handleNumberInput,
  openDialogForItem,
  init,
  destroy,
};
