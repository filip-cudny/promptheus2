import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { AudioClipInfo } from "$lib/types";

export interface TranscriptionRetryEvent {
  attempt: number;
  max_attempts: number;
  next_in_secs: number;
  reason: string;
  entry_id: string | null;
}

export interface TranscriptionErrorEvent {
  message: string;
  recoverable: boolean;
  entry_id: string | null;
  has_audio: boolean;
  attempts: number;
}

export async function retryTranscription(entryId: string): Promise<void> {
  return invoke("retry_transcription", { entryId });
}

export async function getAudioClipInfo(entryId: string): Promise<AudioClipInfo> {
  return invoke("get_audio_clip_info", { entryId });
}

export async function discardAudioClip(entryId: string): Promise<void> {
  return invoke("discard_audio_clip", { entryId });
}

/** Returns false when the user dismisses the save dialog. */
export async function exportAudioClip(
  entryId: string,
  defaultName = "recording.wav",
): Promise<boolean> {
  const destination = await save({
    defaultPath: defaultName,
    filters: [{ name: "WAV audio", extensions: ["wav"] }],
  });
  if (!destination) return false;
  await invoke("export_audio_clip", { entryId, destination });
  return true;
}
