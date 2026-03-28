import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getSettings } from "./settings";
import { executePrompt } from "./promptExecution";

export function setupHotkeyListener(): Promise<UnlistenFn> {
  return listen<string>("hotkey-action", (event) => {
    switch (event.payload) {
      case "open_context_menu":
        invoke("show_context_menu_window");
        break;
      case "execute_active_prompt":
        executeActivePrompt();
        break;
      case "speech_to_text_toggle":
        console.warn("Speech-to-text toggle is not yet implemented");
        break;
      default:
        console.warn(`Unknown hotkey action: ${event.payload}`);
    }
  });
}

async function executeActivePrompt() {
  const settings = await getSettings();
  const prompt = settings.prompts[0];
  if (!prompt) {
    console.warn("No prompts configured — cannot execute active prompt");
    return;
  }
  await executePrompt(prompt.id, {
    onChunk: () => {},
    onDone: () => {},
    onError: (msg) => console.error("Hotkey prompt execution failed:", msg),
  });
}
