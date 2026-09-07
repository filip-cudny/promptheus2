# Speech service

Recording, transcription, and the retry path that keeps a failed voice note recoverable.

## Files

| File | Role |
|------|------|
| `mod.rs` | `SpeechService` state machine (recording session, debounce, pending skill, per-entry retry guard) + `SpeechError` |
| `recorder.rs` | cpal input device negotiation, WAV encoding (`hound`) |
| `transcriber.rs` | Provider calls (OpenAI / ElevenLabs), HTTP error classification, `Retry-After` parsing |
| `retry.rs` | Shared backoff loop used by both the automatic and manual paths |
| `clip_store.rs` | Short-lived WAV storage on disk, indexed by the `audio_clips` table |
| `reminder.rs` | "Still recording" nudge while a session runs |

## Failure model

`SpeechError::Transcription` carries a `FailureKind`:

- **Transient** — 408, 429, 5xx, connect failures, timeouts, unparseable responses. Re-sent automatically.
- **Permanent** — 401/403, 4xx request rejections, missing API key, no speech detected. Never re-sent.

`retry.rs` runs at most `surfaces.speech_to_text.auto_retry_attempts` attempts (default 3, `1` disables retrying) with exponential backoff — 2s, 6s, 18s, capped at 60s, ±20% jitter. A numeric `Retry-After` header overrides the computed delay. Every intermediate failure emits `speech-transcription-retry`.

The HTTP timeout scales with payload size (`30s + bytes / 50 kB/s`, capped at 300s). A fixed ceiling turned slow uplinks into self-inflicted timeouts on multi-minute recordings.

## Audio clips

A recording is written to `audio_clips/<uuid>.wav` under the app data dir before the first transcription attempt, with a row in `audio_clips` (schema v6) holding the path, duration, and `expires_at`. Retention is `surfaces.speech_to_text.audio_retention_hours` (default 3, `0` disables storage entirely).

Lifecycle:

- **Success** — clip deleted immediately, unless `keep_audio_on_success` is set, in which case it is attached to the new history entry.
- **Failure** — a history entry is created with `success = false` and the clip is attached to it. This is the anchor for "Retry transcription" and "Export audio".
- **Expiry** — `spawn_audio_clip_sweeper` runs every 15 minutes (plus once at startup) and drops expired rows with their files.
- **Orphans** — the same sweep deletes any `.wav` no row points at. This is what reclaims clips lost to `ON DELETE CASCADE`, history retention pruning, and `clear()`; there is no file cleanup in the delete paths themselves.

Clips are on disk rather than in memory deliberately: a transcription usually fails because the network or the provider is down, and the user may well quit the app before trying again.

## Retry concurrency

`is_transcribing` guards the freshly recorded audio only. Manual retries use `SpeechService::begin_retry` / `end_retry`, a per-entry-id set, so replaying an hour-old clip never blocks a new recording and two retries of the same entry cannot overlap.

A manual retry **updates the history entry and the clipboard, and never pastes**. By the time someone retries, the app that had focus during dictation is long gone — pasting into whatever happens to be focused now would be worse than useless. For the same reason a retry does not re-run a pending skill, even if the original recording was started as "run skill with transcription".

Nothing is retried automatically at startup. Failed entries with a live clip wait for a click, which keeps the lifecycle free of a replay queue and avoids a burst of requests when the machine comes back online.

## Commands

| Command | Notes |
|---------|-------|
| `toggle_speech_recording` | Start/stop; on stop runs the retry loop in a background task |
| `retry_transcription` | Spawns a background retry; drives the UI through events, returns immediately |
| `get_audio_clip_info` | `has_audio` / `expires_at` / `duration_secs` / `is_retrying` for one entry |
| `export_audio_clip` | Copies the WAV to a caller-supplied destination (frontend picks it via `plugin-dialog`) |
| `discard_audio_clip` | Manual delete before expiry |

## Events

| Event | Payload | Emitted when |
|-------|---------|--------------|
| `speech-recording-started` | `{ action_id }` | Recording begins |
| `speech-recording-stopped` | `{ had_audio }` | Recording ends, before transcription |
| `speech-transcription-retry` | `{ attempt, max_attempts, next_in_secs, reason, entry_id }` | Each transient failure; `attempt: 0` marks the start of a manual retry |
| `speech-transcription-complete` | `{ text, duration_secs, entry_id }` | Success (or a discarded too-short recording) |
| `speech-transcription-error` | `{ message, recoverable, entry_id, has_audio, attempts }` | Attempts exhausted or a permanent failure |

`entry_id` is `null` for the automatic path when no history entry was created (pending-skill executions, no-speech results).
