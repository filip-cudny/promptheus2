import { invoke, Channel } from "@tauri-apps/api/core";
import type { StreamEvent } from "$lib/types/ai";

export type PanelMode = "widened" | "overlay";

export function settingsAgentSend(
  message: string,
  onEvent: (event: StreamEvent) => void,
): Promise<void> {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke("settings_agent_send", { message, onEvent: channel });
}

export function settingsAgentCancel(): Promise<void> {
  return invoke("settings_agent_cancel");
}

export function settingsAgentReset(): Promise<void> {
  return invoke("settings_agent_reset");
}

export function settingsAgentPanel(open: boolean): Promise<PanelMode> {
  return invoke("settings_agent_panel", { open });
}
