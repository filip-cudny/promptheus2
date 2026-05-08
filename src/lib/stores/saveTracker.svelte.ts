export type SaveState = "idle" | "dirty" | "saving" | "saved" | "error";

export interface SaveTrackerOptions {
  debounceMs?: number;
  savedTtlMs?: number;
}

export type PersistFn = () => Promise<void>;

const DEFAULT_DEBOUNCE_MS = 200;
const DEFAULT_SAVED_TTL_MS = 2500;

export function useSaveTracker(opts: SaveTrackerOptions = {}) {
  const debounceMs = opts.debounceMs ?? DEFAULT_DEBOUNCE_MS;
  const savedTtlMs = opts.savedTtlMs ?? DEFAULT_SAVED_TTL_MS;

  let dirty = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let lastSavedAt = $state<number | null>(null);
  let now = $state(Date.now());

  let pending: PersistFn | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let nowTimer: ReturnType<typeof setInterval> | null = null;
  let detachKeyboard: (() => void) | null = null;
  let detachBeforeUnload: (() => void) | null = null;

  function ensureNowTimer() {
    if (nowTimer) return;
    nowTimer = setInterval(() => {
      now = Date.now();
    }, 500);
  }

  const recentlySaved = $derived(
    !dirty && lastSavedAt !== null && now - lastSavedAt < savedTtlMs,
  );

  const state = $derived<SaveState>(
    error
      ? "error"
      : saving
        ? "saving"
        : dirty
          ? "dirty"
          : recentlySaved
            ? "saved"
            : "idle",
  );

  const tooltip = $derived.by(() => {
    if (error) return `Failed to save: ${error}`;
    if (saving) return "Saving…";
    if (dirty) return `Unsaved — autosaving in ${debounceMs} ms`;
    if (lastSavedAt !== null) {
      return `Saved ${formatAgo(now - lastSavedAt)} ago — autosaves on edit`;
    }
    return "Autosaves on edit (⌘S to flush)";
  });

  function markDirty() {
    if (!dirty) dirty = true;
    ensureNowTimer();
  }

  function scheduleSave(fn: PersistFn) {
    pending = fn;
    if (!dirty) dirty = true;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      void runPersist();
    }, debounceMs);
    ensureNowTimer();
  }

  function cancel() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    pending = null;
    if (!saving) dirty = false;
  }

  async function flush(fn?: PersistFn): Promise<boolean> {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (fn) pending = fn;
    if (!pending) return !error;
    return await runPersist();
  }

  async function runPersist(): Promise<boolean> {
    if (saving) return !error;
    if (!pending) return !error;
    const fn = pending;
    pending = null;
    saving = true;
    error = null;
    ensureNowTimer();
    try {
      await fn();
      dirty = false;
      lastSavedAt = Date.now();
      now = Date.now();
      return true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      saving = false;
    }
  }

  function attachKeyboard(target: Window | HTMLElement = window) {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void flush();
      }
    };
    target.addEventListener("keydown", onKey as EventListener);
    detachKeyboard = () =>
      target.removeEventListener("keydown", onKey as EventListener);
    return detachKeyboard;
  }

  function attachBeforeUnload(target: Window = window) {
    const onUnload = () => {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      if (pending) void runPersist();
    };
    target.addEventListener("beforeunload", onUnload);
    detachBeforeUnload = () =>
      target.removeEventListener("beforeunload", onUnload);
    return detachBeforeUnload;
  }

  function clearError() {
    error = null;
  }

  function setError(message: string) {
    error = message;
  }

  function destroy() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (nowTimer) {
      clearInterval(nowTimer);
      nowTimer = null;
    }
    if (detachKeyboard) {
      detachKeyboard();
      detachKeyboard = null;
    }
    if (detachBeforeUnload) {
      detachBeforeUnload();
      detachBeforeUnload = null;
    }
    pending = null;
  }

  return {
    get state() {
      return state;
    },
    get tooltip() {
      return tooltip;
    },
    get error() {
      return error;
    },
    get dirty() {
      return dirty;
    },
    get saving() {
      return saving;
    },
    get recentlySaved() {
      return recentlySaved;
    },
    get lastSavedAt() {
      return lastSavedAt;
    },
    get debounceMs() {
      return debounceMs;
    },
    get hasPending() {
      return pending !== null || saveTimer !== null;
    },
    markDirty,
    scheduleSave,
    cancel,
    flush,
    attachKeyboard,
    attachBeforeUnload,
    clearError,
    setError,
    destroy,
  };
}

export type SaveTracker = ReturnType<typeof useSaveTracker>;

function formatAgo(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  return `${h}h`;
}
