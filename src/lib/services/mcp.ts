import { invoke } from "@tauri-apps/api/core";

export interface McpServerState {
  name: string;
  state: "running" | "error" | "off";
  error: string | null;
  tools: string[];
}

export interface McpServerConfig {
  command: string;
  args: string[];
  env: Record<string, string>;
  timeout_secs: number;
  tool_timeouts: Record<string, number>;
  disabled?: boolean;
  disabled_tools?: string[];
}

export interface McpToolInfo {
  name: string;
  server: string;
  description: string | null;
  disabled: boolean;
}

export async function listMcpServers(): Promise<McpServerState[]> {
  return invoke("list_mcp_servers");
}

export async function listMcpTools(): Promise<McpToolInfo[]> {
  return invoke("list_mcp_tools");
}
