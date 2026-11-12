import { untrack } from "svelte";
import {
  editorDirtyChanged,
  editorRegistered,
  editorUnregistered,
  emptySaveStatusModel,
  saveStatusState,
  saveStatusTooltip,
  writeFailed,
  writeStarted,
  writeSucceeded,
  type SaveStatusModel,
} from "$lib/utils/saveStatus";

const SAVED_TTL_MS = 2500;
const TICK_MS = 500;

let model = $state.raw<SaveStatusModel>(emptySaveStatusModel());
let now = $state(Date.now());
let tickTimer: ReturnType<typeof setInterval> | null = null;
let nextEditorId = 0;

const state = $derived(saveStatusState(model, now, SAVED_TTL_MS));
const tooltip = $derived(saveStatusTooltip(model, now, SAVED_TTL_MS));

function update(next: (current: SaveStatusModel) => SaveStatusModel) {
  model = next(untrack(() => model));
}

function savedTtlElapsed(): boolean {
  const lastSuccessAt = untrack(() => model.lastSuccessAt);
  return lastSuccessAt === null || now - lastSuccessAt >= SAVED_TTL_MS;
}

function startTicker() {
  if (tickTimer) return;
  tickTimer = setInterval(() => {
    now = Date.now();
    if (savedTtlElapsed() && tickTimer) {
      clearInterval(tickTimer);
      tickTimer = null;
    }
  }, TICK_MS);
}

function writeStartedEvent() {
  update(writeStarted);
}

function writeSucceededEvent() {
  now = Date.now();
  update((current) => writeSucceeded(current, now));
  startTicker();
}

function writeFailedEvent(cause: unknown) {
  const text = cause instanceof Error ? cause.message : String(cause);
  update((current) => writeFailed(current, text));
}

function registerEditor(debounceMs: number): string {
  const id = `editor-${nextEditorId++}`;
  update((current) => editorRegistered(current, id, debounceMs));
  return id;
}

function editorDirtyChangedEvent(id: string, dirty: boolean) {
  update((current) => editorDirtyChanged(current, id, dirty));
}

function unregisterEditor(id: string) {
  update((current) => editorUnregistered(current, id));
}

export function getSaveStatus() {
  return {
    get state() {
      return state;
    },
    get tooltip() {
      return tooltip;
    },
    get error() {
      return model.error;
    },
    writeStarted: writeStartedEvent,
    writeSucceeded: writeSucceededEvent,
    writeFailed: writeFailedEvent,
    registerEditor,
    editorDirtyChanged: editorDirtyChangedEvent,
    unregisterEditor,
  };
}
