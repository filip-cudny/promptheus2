import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { error as logError } from "@tauri-apps/plugin-log";
import { executeSkill } from "$lib/services/promptExecution";
import type { InvocationSource } from "$lib/types/history";

let _isExecuting = $state(false);
let _executingSkillId = $state<string | null>(null);

let unlistenStarted: (() => void) | null = null;
let unlistenCompleted: (() => void) | null = null;
let unlistenAlternativeExecute: (() => void) | null = null;

function isExecuting(): boolean {
  return _isExecuting;
}

function getExecutingSkillId(): string | null {
  return _executingSkillId;
}

async function startExecution(
  skillName: string,
  invocationSource: InvocationSource,
  inputOverride?: string,
): Promise<void> {
  _executingSkillId = skillName;

  try {
    await executeSkill(
      skillName,
      {
        onChunk: () => {},
        onDone: () => {},
        onError: (message) => {
          logError("Prompt execution error: " + message);
        },
      },
      invocationSource,
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

async function init() {
  unlistenStarted = await listen<{ execution_id: string; skill_id?: string }>(
    "execution-started",
    (event) => {
      _isExecuting = true;
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
    _executingSkillId = null;
  });

  unlistenAlternativeExecute = await listen<{
    skill_id: string;
    skill_name: string;
    text: string;
  }>("speech-alternative-execute", (event) => {
    const { skill_id, text } = event.payload;
    startExecution(skill_id, "alternative_execute", text).catch((e) =>
      logError("Failed to execute skill from voice input: " + e),
    );
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
  getExecutingSkillId,
  startExecution,
  cancelExecution,
  init,
  destroy,
};
