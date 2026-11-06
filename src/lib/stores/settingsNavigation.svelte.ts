import { tick } from "svelte";
import { warn } from "@tauri-apps/plugin-log";
import type { SettingsSection } from "$lib/components/features/settings/SettingsSidebar.svelte";
import { getSettingsCatalogStore } from "$lib/stores/settingsCatalog.svelte";
import { catalogPath, listItemOf, type ListItemRef } from "$lib/utils/settingsSearch";

const HIGHLIGHT_MS = 2000;
const ELEMENT_WAIT_FRAMES = 20;

let activeSection = $state<SettingsSection>("models");
let selectedItem = $state.raw<ListItemRef | null>(null);
let highlighted = $state<string | null>(null);
let highlightTimer: ReturnType<typeof setTimeout> | undefined;

function nextFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function waitForElement(id: string): Promise<HTMLElement | null> {
  await tick();
  for (let i = 0; i < ELEMENT_WAIT_FRAMES; i++) {
    const el = document.getElementById(id);
    if (el) return el;
    await nextFrame();
  }
  return null;
}

async function navigateTo(key: string) {
  const path = catalogPath(key);
  const entry = getSettingsCatalogStore().entry(path);
  const skill = listItemOf(key);
  if (!entry && skill?.list === "skills") {
    activeSection = "skills";
    selectedItem = { ...skill };
    await tick();
    selectedItem = null;
    return;
  }
  if (!entry) {
    warn(`settings navigation: no catalog entry for ${key}`);
    return;
  }
  activeSection = entry.section;
  const item = listItemOf(key);
  selectedItem = item ? { ...item } : null;
  const el = await waitForElement(`setting-${path}`);
  selectedItem = null;
  if (!el) {
    warn(`settings navigation: field setting-${path} did not render for ${key}`);
    return;
  }
  el.scrollIntoView({ block: "center", behavior: "smooth" });
  highlighted = key;
  clearTimeout(highlightTimer);
  highlightTimer = setTimeout(() => (highlighted = null), HIGHLIGHT_MS);
}

export function getSettingsNavigation() {
  return {
    get activeSection() {
      return activeSection;
    },
    set activeSection(section: SettingsSection) {
      activeSection = section;
    },
    get selectedItem() {
      return selectedItem;
    },
    get highlighted() {
      return highlighted;
    },
    navigateTo,
  };
}
