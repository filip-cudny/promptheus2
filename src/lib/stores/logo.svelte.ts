import { onSettingsChanged } from "$lib/services/events";
import { getSettings } from "$lib/services/settings";

type Logo = "color" | "mono";

let logo = $state<Logo>("color");
let unlisten: (() => void) | null = null;

export function getLogo() {
  return logo;
}

function normalize(value: string | undefined | null): Logo {
  return value === "mono" ? "mono" : "color";
}

async function syncLogo() {
  logo = normalize((await getSettings()).appearance?.logo);
}

export async function initLogo() {
  await syncLogo();

  if (!unlisten) {
    unlisten = await onSettingsChanged(syncLogo);
  }
}
