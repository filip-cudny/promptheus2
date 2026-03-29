import { listen } from "@tauri-apps/api/event";
import { error as logError } from "@tauri-apps/plugin-log";
import { executePrompt } from "$lib/services/promptExecution";

let _isExecuting = $state(false);
let _executionId = $state<string | null>(null);
let _streamedContent = $state("");

let unlistenStarted: (() => void) | null = null;
let unlistenCompleted: (() => void) | null = null;
let unlistenAlternativeExecute: (() => void) | null = null;

function isExecuting(): boolean {
  return _isExecuting;
}

function getExecutionId(): string | null {
  return _executionId;
}

function getStreamedContent(): string {
  return _streamedContent;
}

async function startExecution(
  promptId: string,
  inputOverride?: string,
): Promise<void> {
  _streamedContent = "";

  try {
    await executePrompt(
      promptId,
      {
        onChunk: (_delta, accumulated) => {
          _streamedContent = accumulated;
        },
        onDone: (_fullText) => {
          _streamedContent = "";
        },
        onError: (message) => {
          logError("Prompt execution error: " + message);
          _streamedContent = "";
        },
      },
      inputOverride,
    );
  } catch (e) {
    logError("Failed to execute prompt: " + e);
  }
}

function resetExecution() {
  _isExecuting = false;
  _executionId = null;
  _streamedContent = "";
}

async function init() {
  unlistenStarted = await listen<{ execution_id: string }>(
    "execution-started",
    (event) => {
      _isExecuting = true;
      _executionId = event.payload.execution_id;
    },
  );

  unlistenCompleted = await listen<{
    execution_id: string;
    success: boolean;
    error: string | null;
  }>("execution-completed", () => {
    _isExecuting = false;
    _executionId = null;
    _streamedContent = "";
  });

  unlistenAlternativeExecute = await listen<{
    prompt_id: string;
    text: string;
  }>("speech-alternative-execute", (event) => {
    startExecution(event.payload.prompt_id, event.payload.text);
  });
}

function destroy() {
  if (unlistenStarted) {
    unlistenStarted();
    unlistenStarted = null;
  }
  if (unlistenCompleted) {
    unlistenCompleted();
    unlistenCompleted = null;
  }
  if (unlistenAlternativeExecute) {
    unlistenAlternativeExecute();
    unlistenAlternativeExecute = null;
  }
}

export {
  isExecuting,
  getExecutionId,
  getStreamedContent,
  startExecution,
  resetExecution,
  init,
  destroy,
};
