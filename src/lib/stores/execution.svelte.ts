import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { error as logError } from "@tauri-apps/plugin-log";
import { executeSkill } from "$lib/services/promptExecution";
import { openConversationDialog } from "$lib/services/conversationDialog";

let _isExecuting = $state(false);
let _executionId = $state<string | null>(null);
let _executingSkillId = $state<string | null>(null);
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

function getExecutingSkillId(): string | null {
  return _executingSkillId;
}

function getStreamedContent(): string {
  return _streamedContent;
}

async function startExecution(
  skillName: string,
  inputOverride?: string,
): Promise<void> {
  _executingSkillId = skillName;
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

async function cancelExecution(): Promise<boolean> {
  try {
    return await invoke<boolean>("cancel_skill_execution");
  } catch (e) {
    logError("Failed to cancel execution: " + e);
    return false;
  }
}

function resetExecution() {
  _isExecuting = false;
  _executionId = null;
  _executingSkillId = null;
  _streamedContent = "";
}

async function init() {
  unlistenStarted = await listen<{ execution_id: string; skill_id?: string }>(
    "execution-started",
    (event) => {
      _isExecuting = true;
      _executionId = event.payload.execution_id;
      if (event.payload.skill_id) {
        _executingSkillId = event.payload.skill_id;
      }
    },
  );

  unlistenCompleted = await listen<{
    execution_id: string;
    success: boolean;
    error: string | null;
    cancelled: boolean;
  }>("execution-completed", () => {
    _isExecuting = false;
    _executionId = null;
    _executingSkillId = null;
    _streamedContent = "";
  });

  unlistenAlternativeExecute = await listen<{
    skill_id: string;
    skill_name: string;
    text: string;
  }>("speech-alternative-execute", (event) => {
    const { skill_id, skill_name, text } = event.payload;
    const isChat = skill_id === "__chat__";
    if (isChat) {
      openConversationDialog("", "", undefined, undefined, text, false).catch(
        (e) => logError("Failed to open dialog for voice input: " + e),
      );
    } else {
      startExecution(skill_id, text).catch((e) =>
        logError("Failed to execute skill from voice input: " + e),
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
  getExecutingSkillId,
  getStreamedContent,
  startExecution,
  cancelExecution,
  resetExecution,
  init,
  destroy,
};
