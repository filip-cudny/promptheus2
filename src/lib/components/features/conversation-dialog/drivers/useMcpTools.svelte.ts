import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listMcpTools } from "$lib/services/mcp";
import { onSettingsChanged } from "$lib/services/events";

export function useMcpTools() {
  let webSearchQualifiedId = $state<string | null>(null);

  function fetchTools() {
    listMcpTools()
      .then((tools) => {
        const ws = tools.find((t) => t.name === "web_search" && !t.disabled);
        webSearchQualifiedId = ws ? `${ws.server}.${ws.name}` : null;
      })
      .catch(() => {});
  }

  async function init(): Promise<() => void> {
    fetchTools();
    const win = getCurrentWebviewWindow();
    const unlistenReady = await win.listen("mcp-ready", () => fetchTools());
    const unlistenSettings = await onSettingsChanged(() => fetchTools());
    return () => {
      unlistenReady();
      unlistenSettings();
    };
  }

  return {
    get webSearchQualifiedId() {
      return webSearchQualifiedId;
    },
    init,
  };
}

export type McpTools = ReturnType<typeof useMcpTools>;
