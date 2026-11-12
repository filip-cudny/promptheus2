import { invoke } from "@tauri-apps/api/core";
import { getSaveStatus } from "$lib/stores/saveStatus.svelte";

export async function trackedInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const status = getSaveStatus();
  status.writeStarted();
  try {
    const result = await invoke<T>(command, args);
    status.writeSucceeded();
    return result;
  } catch (e) {
    status.writeFailed(e);
    throw e;
  }
}
