import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DictationBusy = { by: "pill" | "chat" | "settings" } | null;

export function startComposerDictation(tabId?: string): Promise<void> {
  return invoke("start_composer_dictation", { tabId });
}

export function finishComposerDictation(): Promise<void> {
  return invoke("finish_composer_dictation");
}

export function cancelComposerDictation(): Promise<void> {
  return invoke("cancel_composer_dictation");
}

export function retryComposerDictation(): Promise<void> {
  return invoke("retry_composer_dictation");
}

export function dismissComposerDictation(): Promise<void> {
  return invoke("dismiss_composer_dictation");
}

export function getDictationBusy(): Promise<DictationBusy> {
  return invoke("get_dictation_busy");
}

export function onDictationBusyChanged(cb: (busy: DictationBusy) => void): Promise<UnlistenFn> {
  return listen<DictationBusy>("dictation-busy-changed", (e) => cb(e.payload));
}

export function onComposerDictationLevel(
  cb: (level: number, elapsedMs: number) => void,
): Promise<UnlistenFn> {
  return listen<{ tabId: string | null; level: number; elapsedMs: number }>(
    "composer-dictation-level",
    (e) => cb(e.payload.level, e.payload.elapsedMs),
  );
}

export function onComposerDictationTranscribing(cb: () => void): Promise<UnlistenFn> {
  return listen("composer-dictation-transcribing", () => cb());
}

export function onComposerDictationTranscript(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<{ tabId: string | null; text: string }>("composer-dictation-transcript", (e) =>
    cb(e.payload.text),
  );
}

export function onComposerDictationError(cb: (message: string) => void): Promise<UnlistenFn> {
  return listen<{ tabId: string | null; message: string }>("composer-dictation-error", (e) =>
    cb(e.payload.message),
  );
}

export function onComposerDictationCancelled(cb: () => void): Promise<UnlistenFn> {
  return listen("composer-dictation-cancelled", () => cb());
}
