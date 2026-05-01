import { listen } from "@tauri-apps/api/event";
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

export async function initTheme() {
  const settings = await getSettings();
  applyTheme(normalize(settings.theme));

  if (!unlisten) {
    unlisten = await listen("settings-changed", async () => {
      const settings = await getSettings();
      const next = normalize(settings.theme);
      if (next !== theme) applyTheme(next);
    });
  }
}
