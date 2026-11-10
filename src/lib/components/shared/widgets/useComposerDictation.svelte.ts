import { onMount } from "svelte";
import { error as logError } from "@tauri-apps/plugin-log";
import {
  cancelComposerDictation,
  dismissComposerDictation,
  finishComposerDictation,
  getDictationBusy,
  onComposerDictationCancelled,
  onComposerDictationError,
  onComposerDictationLevel,
  onComposerDictationTranscribing,
  onComposerDictationTranscript,
  onDictationBusyChanged,
  retryComposerDictation,
  startComposerDictation,
  type DictationBusy,
} from "$lib/services/composerDictation";

export type DictationOwner = "chat" | "settings";

export type DictationState =
  | { kind: "idle" }
  | { kind: "recording"; level: number; elapsedMs: number }
  | { kind: "transcribing" }
  | { kind: "error"; message: string; retryable: boolean };

type Caret = { start: number; end: number };

const BUSY_TITLES: Record<NonNullable<DictationBusy>["by"], string> = {
  pill: "The pill is recording",
  chat: "Dictation is running in Chat",
  settings: "Dictation is running in Settings",
};

export function useComposerDictation(opts: {
  owner: DictationOwner;
  insert: (tabId: string | undefined, start: number, end: number, text: string) => void;
}) {
  let current = $state<DictationState>({ kind: "idle" });
  let busy = $state<DictationBusy>(null);
  let session: ({ tabId: string | undefined } & Caret) | null = null;

  const blockedTitle = $derived(busy && busy.by !== opts.owner ? BUSY_TITLES[busy.by] : null);

  function fail(action: string, e: unknown, retryable = false) {
    logError(`composer dictation ${action} failed: ${e}`);
    current = { kind: "error", message: String(e), retryable };
  }

  async function start(tabId: string | undefined, caret: Caret) {
    if (current.kind !== "idle") return;
    session = { tabId, ...caret };
    current = { kind: "recording", level: 0, elapsedMs: 0 };
    try {
      await startComposerDictation(tabId);
    } catch (e) {
      fail("start", e);
    }
  }

  async function finish() {
    if (current.kind !== "recording") return;
    current = { kind: "transcribing" };
    try {
      await finishComposerDictation();
    } catch (e) {
      fail("finish", e);
    }
  }

  async function cancel() {
    if (current.kind !== "recording" && current.kind !== "transcribing") return;
    current = { kind: "idle" };
    try {
      await cancelComposerDictation();
    } catch (e) {
      logError(`composer dictation cancel failed: ${e}`);
    }
  }

  async function retry() {
    if (current.kind !== "error" || !current.retryable) return;
    current = { kind: "transcribing" };
    try {
      await retryComposerDictation();
    } catch (e) {
      fail("retry", e);
    }
  }

  async function dismiss() {
    if (current.kind !== "error") return;
    current = { kind: "idle" };
    try {
      await dismissComposerDictation();
    } catch (e) {
      logError(`composer dictation dismiss failed: ${e}`);
    }
  }

  function receive(text: string) {
    if (current.kind === "idle" || !session) return;
    const { tabId, start, end } = session;
    current = { kind: "idle" };
    if (text) opts.insert(tabId, start, end, text);
  }

  onMount(() => {
    let busyEventSeen = false;
    const unlisten = Promise.all([
      onDictationBusyChanged((next) => {
        busyEventSeen = true;
        busy = next;
      }),
      onComposerDictationLevel((level, elapsedMs) => {
        if (current.kind === "recording") current = { kind: "recording", level, elapsedMs };
      }),
      onComposerDictationTranscribing(() => {
        if (current.kind !== "idle") current = { kind: "transcribing" };
      }),
      onComposerDictationTranscript(receive),
      onComposerDictationError((message) => {
        if (current.kind !== "idle") current = { kind: "error", message, retryable: true };
      }),
      onComposerDictationCancelled(() => {
        current = { kind: "idle" };
      }),
    ]);
    unlisten.catch((e) => logError(`composer dictation listen failed: ${e}`));
    getDictationBusy()
      .then((initial) => {
        if (!busyEventSeen) busy = initial;
      })
      .catch((e) => logError(`get_dictation_busy failed: ${e}`));

    return () => {
      unlisten.then((fns) => fns.forEach((fn) => fn()));
      if (current.kind !== "idle") {
        cancelComposerDictation().catch((e) => logError(`composer dictation cancel failed: ${e}`));
      }
    };
  });

  return {
    get state() {
      return current;
    },
    get active() {
      return current.kind !== "idle";
    },
    get blockedTitle() {
      return blockedTitle;
    },
    start,
    finish,
    cancel,
    retry,
    dismiss,
  };
}

export type ComposerDictation = ReturnType<typeof useComposerDictation>;
