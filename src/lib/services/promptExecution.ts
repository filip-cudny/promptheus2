import { invoke, Channel } from "@tauri-apps/api/core";
import type { StreamEvent } from "$lib/types/ai";

export interface ExecutionCallbacks {
  onChunk: (delta: string, accumulated: string) => void;
  onDone: (fullText: string) => void;
  onError: (message: string) => void;
}

export async function executePrompt(
  promptId: string,
  callbacks: ExecutionCallbacks,
  inputOverride?: string,
): Promise<void> {
  const onEvent = new Channel<StreamEvent>();
  onEvent.onmessage = (event) => {
    switch (event.event) {
      case "chunk":
        callbacks.onChunk(event.data.delta, event.data.accumulated);
        break;
      case "done":
        callbacks.onDone(event.data.full_text);
        break;
      case "error":
        callbacks.onError(event.data.message);
        break;
    }
  };
  return invoke("execute_prompt", {
    promptId,
    inputOverride: inputOverride ?? null,
    onEvent,
  });
}

export async function getExecutionState(): Promise<{
  is_executing: boolean;
  execution_id: string | null;
}> {
  return invoke("get_execution_state");
}
