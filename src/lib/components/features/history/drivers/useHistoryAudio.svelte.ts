import { onMount } from "svelte";
import { listen } from "@tauri-apps/api/event";
import { error as logError } from "@tauri-apps/plugin-log";
import {
  discardAudioClip,
  exportAudioClip,
  getAudioClipInfo,
  retryTranscription,
  type TranscriptionErrorEvent,
  type TranscriptionRetryEvent,
} from "$lib/services/speech";
import type { AudioClipInfo, HistoryEntry } from "$lib/types";

export function useHistoryAudio(opts: { entries: () => HistoryEntry[] }) {
  let audioClips = $state<Record<string, AudioClipInfo>>({});
  let retryingIds = $state<Set<string>>(new Set());

  async function loadAudioClip(entryId: string) {
    try {
      const info = await getAudioClipInfo(entryId);
      audioClips = { ...audioClips, [entryId]: info };
      if (info.is_retrying) markRetrying(entryId, true);
    } catch (e) {
      logError(`get_audio_clip_info failed for ${entryId}: ${e}`);
    }
  }

  function markRetrying(entryId: string, active: boolean) {
    const next = new Set(retryingIds);
    if (active) next.add(entryId);
    else next.delete(entryId);
    retryingIds = next;
  }

  $effect(() => {
    for (const entry of opts.entries()) {
      if (entry.entry_type !== "speech") continue;
      if (entry.id in audioClips) continue;
      void loadAudioClip(entry.id);
    }
  });

  async function retry(entry: HistoryEntry) {
    markRetrying(entry.id, true);
    try {
      await retryTranscription(entry.id);
    } catch (e) {
      markRetrying(entry.id, false);
      logError(`retry_transcription failed for ${entry.id}: ${e}`);
    }
  }

  async function exportAudio(entry: HistoryEntry) {
    try {
      await exportAudioClip(entry.id, `voice-note-${entry.id.slice(0, 8)}.wav`);
    } catch (e) {
      logError(`export_audio_clip failed for ${entry.id}: ${e}`);
    }
  }

  async function discardAudio(entry: HistoryEntry) {
    try {
      await discardAudioClip(entry.id);
    } catch (e) {
      logError(`discard_audio_clip failed for ${entry.id}: ${e}`);
    }
    await loadAudioClip(entry.id);
  }

  function onTranscriptionFinished(entryId: string | null) {
    if (!entryId) return;
    markRetrying(entryId, false);
    void loadAudioClip(entryId);
  }

  onMount(() => {
    const pending = Promise.all([
      listen<TranscriptionRetryEvent>("speech-transcription-retry", (event) => {
        const entryId = event.payload.entry_id;
        if (entryId) markRetrying(entryId, true);
      }),
      listen<TranscriptionErrorEvent>("speech-transcription-error", (event) =>
        onTranscriptionFinished(event.payload.entry_id),
      ),
      listen<{ entry_id: string | null }>("speech-transcription-complete", (event) =>
        onTranscriptionFinished(event.payload.entry_id),
      ),
    ]);
    return () => {
      void pending.then((unlistens) => {
        for (const fn of unlistens) fn();
      });
    };
  });

  return {
    get audioClips() {
      return audioClips;
    },
    get retryingIds() {
      return retryingIds;
    },
    retry,
    exportAudio,
    discardAudio,
  };
}
