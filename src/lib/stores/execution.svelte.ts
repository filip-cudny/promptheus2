import { listen } from "@tauri-apps/api/event";
import { error as logError } from "@tauri-apps/plugin-log";
import { executeSkill } from "$lib/services/promptExecution";
import { openPromptDialog } from "$lib/services/promptDialog";

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
  skillName: string,
  inputOverride?: string,
): Promise<void> {
  _streamedContent = "";

  try {
    await executeSkill(
      skillName,
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
    prompt_name: string;
    text: string;
  }>("speech-alternative-execute", (event) => {
    const { prompt_id, prompt_name, text } = event.payload;
    const isChat = prompt_id === "__chat__";
    if (isChat) {
      openPromptDialog("", prompt_name || prompt_id, undefined, undefined, text, false).catch(
        (e) => logError("Failed to open dialog for voice input: " + e),
      );
    } else {
      startExecution(prompt_id, text).catch((e) =>
        logError("Failed to execute prompt from voice input: " + e),
      );
    }
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
