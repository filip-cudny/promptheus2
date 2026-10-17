import { onSettingsChanged } from "$lib/services/events";
import { getSettings, updateSetting } from "$lib/services/settings";

type Theme = "dark" | "light";

const DEFAULT_THEME: Theme = "dark";

let theme = $state<Theme>(DEFAULT_THEME);
let unlisten: (() => void) | null = null;

export function getTheme() {
  return theme;
}

function normalize(value: string | undefined | null): Theme {
  return value === "light" ? "light" : "dark";
}

function applyTheme(t: Theme) {
  theme = t;
  document.documentElement.setAttribute("data-theme", t);
}

export async function setTheme(t: Theme) {
  applyTheme(t);
  await updateSetting("theme", t);
}

export function syncTheme(value: string | undefined | null) {
  applyTheme(normalize(value));
}

export async function initTheme() {
  syncTheme((await getSettings()).theme);

  if (!unlisten) {
    unlisten = await onSettingsChanged(async () => {
      syncTheme((await getSettings()).theme);
    });
  }
}
