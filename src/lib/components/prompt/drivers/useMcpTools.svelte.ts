import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function useMcpTools() {
  let webSearchQualifiedId = $state<string | null>(null);

  function fetchTools() {
    invoke<{ name: string; server: string }[]>("list_mcp_tools")
      .then((tools) => {
        const ws = tools.find((t) => t.name === "web_search");
        webSearchQualifiedId = ws ? `${ws.server}.${ws.name}` : null;
      })
      .catch(() => {});
  }

  async function init(): Promise<() => void> {
    fetchTools();
    const win = getCurrentWebviewWindow();
    const unlisten = await win.listen("mcp-ready", () => fetchTools());
    return () => unlisten();
  }

  return {
    get webSearchQualifiedId() {
      return webSearchQualifiedId;
    },
    init,
  };
}

export type McpTools = ReturnType<typeof useMcpTools>;
