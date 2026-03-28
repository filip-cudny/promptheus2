import { invoke, Channel } from "@tauri-apps/api/core";
import type { ProcessedMessage } from "$lib/types/ai";
import type { StreamEvent } from "$lib/types/ai";

export async function complete(
  modelId: string,
  messages: ProcessedMessage[],
): Promise<string> {
  return invoke("complete", { modelId, messages });
}

export interface StreamCallbacks {
  onChunk: (delta: string, accumulated: string) => void;
  onDone: (fullText: string) => void;
  onError: (message: string) => void;
}

export async function completeStream(
  modelId: string,
  messages: ProcessedMessage[],
  callbacks: StreamCallbacks,
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
  return invoke("complete_stream", { modelId, messages, onEvent });
}
