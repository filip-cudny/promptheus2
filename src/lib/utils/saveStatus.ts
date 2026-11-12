export type SaveStatusState = "idle" | "dirty" | "saving" | "saved" | "error";

export interface EditorEntry {
  dirty: boolean;
  debounceMs: number;
}

export interface SaveStatusModel {
  writesInFlight: number;
  editors: Readonly<Record<string, EditorEntry>>;
  lastSuccessAt: number | null;
  error: string | null;
}

export function emptySaveStatusModel(): SaveStatusModel {
  return { writesInFlight: 0, editors: {}, lastSuccessAt: null, error: null };
}

export function writeStarted(model: SaveStatusModel): SaveStatusModel {
  return { ...model, writesInFlight: model.writesInFlight + 1 };
}

export function writeSucceeded(
  model: SaveStatusModel,
  now: number,
): SaveStatusModel {
  return {
    ...model,
    writesInFlight: Math.max(0, model.writesInFlight - 1),
    lastSuccessAt: now,
    error: null,
  };
}

export function writeFailed(
  model: SaveStatusModel,
  error: string,
): SaveStatusModel {
  return {
    ...model,
    writesInFlight: Math.max(0, model.writesInFlight - 1),
    error,
  };
}

export function editorRegistered(
  model: SaveStatusModel,
  id: string,
  debounceMs: number,
): SaveStatusModel {
  return {
    ...model,
    editors: { ...model.editors, [id]: { dirty: false, debounceMs } },
  };
}

export function editorDirtyChanged(
  model: SaveStatusModel,
  id: string,
  dirty: boolean,
): SaveStatusModel {
  const entry = model.editors[id];
  if (!entry || entry.dirty === dirty) return model;
  return { ...model, editors: { ...model.editors, [id]: { ...entry, dirty } } };
}

export function editorUnregistered(
  model: SaveStatusModel,
  id: string,
): SaveStatusModel {
  if (!(id in model.editors)) return model;
  const { [id]: _removed, ...editors } = model.editors;
  return { ...model, editors };
}

export function saveStatusState(
  model: SaveStatusModel,
  now: number,
  ttlMs: number,
): SaveStatusState {
  if (model.error) return "error";
  if (model.writesInFlight > 0) return "saving";
  if (Object.values(model.editors).some((e) => e.dirty)) return "dirty";
  if (model.lastSuccessAt !== null && now - model.lastSuccessAt < ttlMs) {
    return "saved";
  }
  return "idle";
}

export function saveStatusTooltip(
  model: SaveStatusModel,
  now: number,
  ttlMs: number,
): string {
  switch (saveStatusState(model, now, ttlMs)) {
    case "error":
      return `Failed to save: ${model.error}`;
    case "saving":
      return "Saving…";
    case "dirty": {
      const dirtyEditor = Object.values(model.editors).find((e) => e.dirty);
      return `Unsaved — autosaving in ${dirtyEditor?.debounceMs} ms`;
    }
    case "saved":
      return `Saved ${formatAgo(now - (model.lastSuccessAt ?? now))} ago — autosaves on edit`;
    case "idle":
      return Object.keys(model.editors).length > 0
        ? "All changes saved (⌘S to flush)"
        : "All changes saved";
  }
}

export function formatAgo(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  return `${h}h`;
}
