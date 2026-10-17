import { invoke } from "@tauri-apps/api/core";

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

export function countTokensDebounced(
  text: string,
  callback: (count: number) => void,
  imageCount = 0,
  delay = 300,
): void {
  if (debounceTimer) clearTimeout(debounceTimer);

  if (!text.trim() && imageCount === 0) {
    callback(0);
    return;
  }

  debounceTimer = setTimeout(async () => {
    try {
      const count = await invoke<number>("count_tokens", { text, imageCount });
      callback(count);
    } catch {
      callback(0);
    }
  }, delay);
}
