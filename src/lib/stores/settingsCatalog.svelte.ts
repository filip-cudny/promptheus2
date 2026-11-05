import { getSettingsCatalog } from "$lib/services/settings";
import type { CatalogEntry } from "$lib/types";
import { error as logError } from "@tauri-apps/plugin-log";

let entries = $state.raw<CatalogEntry[]>([]);
let loaded = $state(false);
let started = false;

function start() {
  if (started) return;
  started = true;
  getSettingsCatalog()
    .then((list) => {
      entries = list;
      loaded = true;
    })
    .catch((e) => {
      started = false;
      logError(`get_settings_catalog failed: ${e}`);
    });
}

export function catalogPath(concrete: string): string {
  const path = concrete.replace(/\[[^\]]*\]/g, "[]");
  const tail = path.startsWith("mcp_servers.") ? path.slice("mcp_servers.".length) : null;
  if (tail === null) return path;
  const dot = tail.indexOf(".");
  return dot < 0 ? "mcp_servers.*" : `mcp_servers.*.${tail.slice(dot + 1)}`;
}

export function getSettingsCatalogStore() {
  start();
  return {
    get entries() {
      return entries;
    },
    get loaded() {
      return loaded;
    },
    entry(key: string): CatalogEntry | undefined {
      return entries.find((e) => e.key === key);
    },
  };
}
