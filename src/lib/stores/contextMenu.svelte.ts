import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { MenuItem } from "$lib/types/menu";

let _items = $state<MenuItem[]>([]);
let _selectedIndex = $state(-1);
let _visible = $state(false);
let numberBuffer = "";
let numberTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;

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

async function openMenu() {
  try {
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    _items = fetched;
    _selectedIndex = -1;
    numberBuffer = "";
    _visible = true;

    const win = getCurrentWebviewWindow();
    await win.show();
    await win.setFocus();
  } catch (e) {
    console.error("Failed to open context menu:", e);
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

  try {
    await invoke("execute_menu_item", {
      itemId: item.id,
      shiftPressed,
    });
  } catch (e) {
    console.error("Failed to execute menu item:", e);
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

async function init() {
  unlisten = await listen("show-context-menu", () => {
    openMenu();
  });
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
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
  init,
  destroy,
};
