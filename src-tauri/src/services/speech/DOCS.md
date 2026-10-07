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
| `widget.rs` | `RecordingWidget`: widget transitions, 50 ms level task, transport choice, position storage |

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
| `pause_speech_recording` | Pauses the running recording; `NotRecording` error when none runs |
| `resume_speech_recording` | Resumes a paused recording |
| `cancel_speech_recording` | Discards the recording without transcribing; ignored when not recording or while transcribing |
| `retry_transcription` | Spawns a background retry; drives the UI through events, returns immediately |
| `get_audio_clip_info` | `has_audio` / `expires_at` / `duration_secs` / `is_retrying` for one entry |
| `export_audio_clip` | Copies the WAV to a caller-supplied destination (frontend picks it via `plugin-dialog`) |
| `discard_audio_clip` | Manual delete before expiry |

## Events

| Event | Payload | Emitted when |
|-------|---------|--------------|
| `speech-recording-started` | `{ action_id }` | Recording begins |
| `speech-recording-stopped` | `{ had_audio }` | Recording ends, before transcription |
| `recording-widget-state` | `{ state, level, elapsedMs }` | Every widget update (`state`: `recording`/`paused`/`processing`/`done`, `level` 0..1); sent to the `recording-widget` window only |
| `speech-transcription-retry` | `{ attempt, max_attempts, next_in_secs, reason, entry_id }` | Each transient failure; `attempt: 0` marks the start of a manual retry |
| `speech-transcription-complete` | `{ text, duration_secs, entry_id }` | Success (or a discarded too-short recording) |
| `speech-transcription-error` | `{ message, recoverable, entry_id, has_audio, attempts }` | Attempts exhausted or a permanent failure |

Cancel emits `speech-recording-stopped` and then `speech-transcription-complete` with empty `text`, like a discarded too-short recording.

`entry_id` is `null` for the automatic path when no history entry was created (pending-skill executions, no-speech results).

## Recording widget

`widget.rs` (`RecordingWidget`) is called from `commands/speech.rs` at every transition, so one place owns widget state and the setting check. Setting: `surfaces.speech_to_text.show_recording_widget`, default `true`; off keeps the toast-only behavior.

`surfaces.speech_to_text.monochromatic_widget_icon`, default `true`: the "Copied" check of the `done` state uses the widget foreground (`#e5e5e7`); off draws it green (`#62A878`). Read in `show()` with `show_recording_widget`, so a change applies from the next recording. GNOME: 4th argument of `ShowRecordingWidget`, the extension picks `icons/recording-check-mono.svg` or `icons/recording-check.svg`. X11/macOS: `monochromatic` field of every `recording-widget-state` payload.

- **Transports** — GNOME Wayland: the extension draws the widget (`ShowRecordingWidget` / `UpdateRecordingWidget` / `HideRecordingWidget`, see [linux-wayland-gnome-extension.md](../../../../docs/gotchas/linux-wayland-gnome-extension.md)). X11 and macOS: webview window `recording-widget` (`src/windows/recording-widget/`) fed by `recording-widget-state`. Any other Wayland session, or a failed `ShowRecordingWidget`, falls back to toasts for that recording with a `warn` log.
- **Toasts** — while the widget is shown it replaces "Recording started", "Processing audio" and "Speech transcribed". Error and warning toasts and the reminder stay.
- **Transitions** — `recording` ⇄ `paused` → `processing` → `done` (about 1 s, "Copied"), then closed. `processing` shows only a dimmed wave of 3 px bars across the inner width of the pill, travelling left to right once per 1.2 s: no buttons, no timer, no label. GNOME: 35 bars driven by one `GLib.timeout_add` (33 ms), opacity 150. X11/macOS: 35 bars with a staggered CSS `@keyframes` `scaleY` animation, color `#929294` instead of opacity (see [linux-webkit-opacity.md](../../../../docs/gotchas/linux-webkit-opacity.md)). Closed without `done`: start failure (never shown), too-short recording, encode or runtime/API-key error, transcription error, cancel, no speech detected. A pending skill closes the widget right after processing, with no `done`.
- **Pause** — `is_recording` stays `true`; the cpal callback drops samples while paused and the microphone stays open (the OS indicator stays on); `SpeechService::elapsed()` excludes paused time; the reminder skips paused ticks and shifts its timers by the pause length. Stop from pause transcribes.
- **Level** — a session-scoped task every 50 ms computes the RMS of the new buffer tail (`reminder::rms`) and sends a level on a dB scale with state and elapsed time, awaiting each update before the next tick. `level_from_rms`: `dbfs = 20·log10(rms / 32768)`, level = `clamp((dbfs − FLOOR_DB) / (CEIL_DB − FLOOR_DB), 0, 1)` with `FLOOR_DB = −50`, `CEIL_DB = −20` (RMS ≈ 104 → 0, ≈ 1036 → 0.67, ≥ 3277 → 1). A linear `rms / 6000` kept normal speech (RMS 500–3000) at 0.1–0.4, so the bars looked flat. The mapped level then goes through `smooth_level`, an envelope follower stepped once per tick: `level += (target − level) · rate` with `LEVEL_ATTACK = 0.6` when the target is higher and `LEVEL_RELEASE = 0.2` when it is lower, so a rise reaches 60 % of the step in 50 ms and a fall from 1 to silence takes about 11 ticks (550 ms) to drop below 0.1. The smoothed value starts at 0 per session and is reset to 0 while paused. Rendering interpolates between the 20 Hz updates: GNOME calls Clutter `ease` on each bar's height (100 ms `EASE_OUT_QUAD`), so Clutter computes the intermediate frames on the frame clock and JS runs only on the 20 Hz updates (see [linux-wayland-gnome-extension.md](../../../../docs/gotchas/linux-wayland-gnome-extension.md)); X11/macOS bars are `transform: scaleY(…)` with `transition: transform 120ms ease-out`.
- **Position** — `ui_state.json` keys `recording_widget.shell_position` (logical stage pixels) and `recording_widget.window_position` (physical pixels); `{ x, y }`. Separate keys so a session switch never reuses coordinates in the wrong space. Default: bottom center of the work area under the pointer, 24 px above the bottom edge.
