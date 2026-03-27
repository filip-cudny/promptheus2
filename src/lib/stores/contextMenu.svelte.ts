import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { MenuItem } from "$lib/types/menu";

let items = $state<MenuItem[]>([]);
let selectedIndex = $state(-1);
let visible = $state(false);
let numberBuffer = $state("");
let numberTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;

const NUMBER_DEBOUNCE_MS = 300;

const navigableItems = $derived(
  items.filter((item) => item.enabled),
);

async function openMenu() {
  try {
    const fetched = await invoke<MenuItem[]>("get_context_menu_items");
    items = fetched;
    selectedIndex = -1;
    numberBuffer = "";
    visible = true;

    const win = getCurrentWebviewWindow();
    await win.show();
    await win.setFocus();
  } catch (e) {
    console.error("Failed to open context menu:", e);
  }
}

async function closeMenu() {
  visible = false;
  items = [];
  selectedIndex = -1;
  numberBuffer = "";

  const win = getCurrentWebviewWindow();
  await win.hide();
}

function moveSelection(direction: 1 | -1) {
  if (navigableItems.length === 0) return;

  const currentItem = selectedIndex >= 0 ? items[selectedIndex] : null;
  const currentNavIndex = currentItem
    ? navigableItems.indexOf(currentItem)
    : -1;

  let nextNavIndex: number;
  if (currentNavIndex === -1) {
    nextNavIndex = direction === 1 ? 0 : navigableItems.length - 1;
  } else {
    nextNavIndex =
      (currentNavIndex + direction + navigableItems.length) %
      navigableItems.length;
  }

  const targetItem = navigableItems[nextNavIndex];
  selectedIndex = items.indexOf(targetItem);
}

async function executeItem(index: number, shiftPressed: boolean = false) {
  const item = items[index];
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
  if (selectedIndex >= 0 && selectedIndex < items.length) {
    await executeItem(selectedIndex, shiftPressed);
  }
}

function handleNumberInput(digit: string) {
  if (numberTimer) clearTimeout(numberTimer);

  numberBuffer += digit;

  numberTimer = setTimeout(() => {
    const num = parseInt(numberBuffer, 10);
    numberBuffer = "";

    if (num >= 1 && num <= navigableItems.length) {
      const targetItem = navigableItems[num - 1];
      const targetIndex = items.indexOf(targetItem);
      selectedIndex = targetIndex;
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

function setSelectedIndex(index: number) {
  selectedIndex = index;
}

export {
  items,
  selectedIndex,
  visible,
  openMenu,
  closeMenu,
  moveSelection,
  executeItem,
  executeSelected,
  handleNumberInput,
  setSelectedIndex,
  init,
  destroy,
};
