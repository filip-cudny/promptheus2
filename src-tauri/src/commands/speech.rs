use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::HistoryEntryType;
use crate::models::settings::ModelConfig;
use crate::models::speech::{
    AudioClipInfo, SpeechRecordingStartedEvent, SpeechRecordingStoppedEvent,
    SpeechTranscriptionError, SpeechTranscriptionRetry,
};
use crate::services::config::{ConfigService, KeytermsDoc};
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::speech::{
    self, AudioClipStore, SpeechError, SpeechService, SttOptions,
};
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

const MIN_RECORDING_SAMPLES_SECS: f64 = 1.0;

#[derive(Clone, Serialize)]
struct TranscriptionComplete {
    text: String,
    duration_secs: f64,
    entry_id: Option<String>,
}

#[derive(Clone, Serialize)]
struct AlternativeExecutePayload {
    skill_id: String,
    skill_name: String,
    text: String,
}

#[derive(Serialize)]
pub struct RecordingState {
    is_recording: bool,
    is_transcribing: bool,
    action_id: Option<String>,
}

struct SttRuntime {
    model: ModelConfig,
    options: SttOptions,
    auto_retry_attempts: u32,
    audio_retention_hours: u32,
    keep_audio_on_success: bool,
}

async fn load_stt_runtime(
    config_state: &Arc<Mutex<ConfigService>>,
) -> Result<SttRuntime, SpeechError> {
    let guard = config_state.lock().await;
    let stt_prompt = guard.stt_prompt();
    let keyterms = guard.stt_keyterms();
    let surface = guard.settings().surfaces.speech_to_text.clone();
    let model = guard
        .resolve_stt_model()
        .cloned()
        .ok_or(SpeechError::ApiKeyMissing)?;

    Ok(SttRuntime {
        model,
        options: SttOptions {
            language: surface.language,
            no_verbatim: surface.no_verbatim,
            prompt: stt_prompt,
            keyterms,
        },
        auto_retry_attempts: surface.auto_retry_attempts,
        audio_retention_hours: surface.audio_retention_hours,
        keep_audio_on_success: surface.keep_audio_on_success,
    })
}

fn format_clip_duration(duration_secs: f64) -> String {
    let total = duration_secs.round().max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn failure_input_content(duration_secs: f64) -> String {
    format!(
        "Voice note · {} — transcription failed",
        format_clip_duration(duration_secs)
    )
}

fn emit_retry(
    app: &AppHandle,
    entry_id: Option<String>,
    notice: speech::retry::RetryNotice,
    reason: String,
) {
    let _ = app.emit(
        "speech-transcription-retry",
        SpeechTranscriptionRetry {
            attempt: notice.attempt,
            max_attempts: notice.max_attempts,
            next_in_secs: notice.delay_secs,
            reason,
            entry_id,
        },
    );
}

async fn notify_retry_started(
    app: &AppHandle,
    config_state: &Arc<Mutex<ConfigService>>,
    notice: speech::retry::RetryNotice,
) {
    let notification_settings = config_state.lock().await.settings().notifications.clone();
    let notifications = app.state::<NotificationService>();
    let _ = notifications.notify(
        "speech_transcription_retry",
        NotificationLevel::Warning,
        "Retrying transcription",
        Some(format!(
            "Attempt {} of {} failed — retrying in {}s",
            notice.attempt, notice.max_attempts, notice.delay_secs
        )),
        &notification_settings,
    );
}

#[tauri::command]
pub async fn toggle_speech_recording(
    app: AppHandle,
    action_id: Option<String>,
) -> crate::Result<()> {
    let speech_state = app.state::<Arc<Mutex<SpeechService>>>();
    let config_state = app.state::<Arc<Mutex<ConfigService>>>();
    let notifications = app.state::<NotificationService>();
    let history_state = app.state::<Arc<Mutex<SqliteHistoryService>>>();
    let clip_state = app.state::<Arc<AudioClipStore>>();

    let was_recording;
    let raw_audio;
    let started_action_id;
    let started_session;

    {
        let mut s = speech_state.lock().await;

        if s.is_transcribing() {
            log::debug!("toggle_speech_recording: ignored, transcription in progress");
            let notification_settings =
                config_state.lock().await.settings().notifications.clone();
            let _ = notifications.notify(
                "speech_recording_start",
                NotificationLevel::Warning,
                "Transcription in progress",
                Some("Wait until it finishes"),
                &notification_settings,
            );
            return Ok(());
        }

        was_recording = s.is_recording();

        if !was_recording {
            if s.is_debouncing() {
                log::debug!("toggle_speech_recording: ignored, debounced");
                return Ok(());
            }
            s.mark_toggle();
        }

        if was_recording {
            let audio = s.stop_recording_raw()?;
            s.set_transcribing(true);
            started_action_id = None;
            raw_audio = Some(audio);
            started_session = None;
        } else {
            raw_audio = None;
            s.start_recording(action_id.clone())?;
            started_action_id = action_id;
            started_session = Some(s.session());
        }
    }

    if !was_recording {
        let _ = app.emit(
            "speech-recording-started",
            SpeechRecordingStartedEvent {
                action_id: started_action_id,
            },
        );
        let (notification_settings, shortcut_hint) = {
            let guard = config_state.lock().await;
            let settings = guard.settings();
            (
                settings.notifications.clone(),
                crate::services::hotkeys::shortcut_for_action(settings, "speech_to_text_toggle"),
            )
        };
        let _ = notifications.notify(
            "speech_recording_start",
            NotificationLevel::Info,
            "Recording started",
            Some("Click Speech to Text again to stop."),
            &notification_settings,
        );
        if let Some(session) = started_session {
            speech::reminder::spawn(
                app.clone(),
                session,
                notification_settings.recording_reminder.clone(),
                shortcut_hint,
            );
        }
        return Ok(());
    }

    let (samples, sample_rate) = raw_audio.unwrap();
    let had_audio = !samples.is_empty();
    let sample_count = samples.len();
    let duration_secs = sample_count as f64 / sample_rate.max(1) as f64;

    let _ = app.emit(
        "speech-recording-stopped",
        SpeechRecordingStoppedEvent { had_audio },
    );

    let min_samples = (sample_rate as f64 * MIN_RECORDING_SAMPLES_SECS) as usize;
    if sample_count < min_samples {
        log::debug!(
            "toggle_speech_recording: recording too short ({} samples, need {}), discarding",
            sample_count,
            min_samples,
        );
        let _ = app.emit(
            "speech-transcription-complete",
            TranscriptionComplete { text: String::new(), duration_secs: 0.0, entry_id: None },
        );
        let mut s = speech_state.lock().await;
        s.set_transcribing(false);
        s.set_pending_prompt(None, None);
        return Ok(());
    }

    let wav_bytes = match tokio::task::spawn_blocking(move || speech::encode_wav(&samples, sample_rate))
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    {
        Ok(bytes) => bytes,
        Err(e) => {
            let mut s = speech_state.lock().await;
            s.set_transcribing(false);
            s.set_pending_prompt(None, None);
            return Err(e.into());
        }
    };

    let runtime = match load_stt_runtime(&config_state).await {
        Ok(runtime) => {
            let notification_settings =
                config_state.lock().await.settings().notifications.clone();
            let _ = notifications.notify(
                "speech_recording_stop",
                NotificationLevel::Info,
                "Processing audio",
                Some("Transcribing your speech to text"),
                &notification_settings,
            );
            runtime
        }
        Err(e) => {
            let mut s = speech_state.lock().await;
            s.set_transcribing(false);
            s.set_pending_prompt(None, None);
            return Err(Error::Speech(e));
        }
    };

    let clip_id = if runtime.audio_retention_hours > 0 {
        let history = history_state.lock().await;
        match clip_state.store(
            history.conn(),
            &wav_bytes,
            sample_rate,
            duration_secs,
            runtime.audio_retention_hours,
        ) {
            Ok(id) => Some(id),
            Err(e) => {
                log::warn!("failed to persist audio clip, retry will be unavailable: {e}");
                None
            }
        }
    } else {
        None
    };

    let app_clone = app.clone();
    let speech_inner = Arc::clone(&*speech_state);
    let config_inner = Arc::clone(&*config_state);
    let history_inner = Arc::clone(&*history_state);
    let clip_inner = Arc::clone(&*clip_state);

    tokio::spawn(async move {
        let notifications_inner = app_clone.state::<NotificationService>();
        let start = std::time::Instant::now();

        let mut first_retry: Option<speech::retry::RetryNotice> = None;
        let mut attempts_used = 1u32;

        let result = speech::transcribe_with_retry(
            wav_bytes,
            &runtime.model,
            &runtime.options,
            runtime.auto_retry_attempts,
            |notice, error| {
                attempts_used = notice.attempt + 1;
                if first_retry.is_none() {
                    first_retry = Some(notice);
                }
                emit_retry(&app_clone, None, notice, error.to_string());
            },
        )
        .await;

        if let Some(notice) = first_retry {
            notify_retry_started(&app_clone, &config_inner, notice).await;
        }

        match result {
            Ok(text) => {
                let duration_secs = start.elapsed().as_secs_f64();
                let pending = speech_inner.lock().await.take_pending_prompt();
                let mut created_entry_id = None;

                if let Some(skill_id) = pending.0 {
                    let _ = app_clone.emit(
                        "speech-alternative-execute",
                        AlternativeExecutePayload {
                            skill_id,
                            skill_name: pending.1.unwrap_or_default(),
                            text: text.clone(),
                        },
                    );
                } else {
                    if let Err(e) = crate::services::clipboard::write_text(&app_clone, &text) {
                        log::error!("Failed to copy transcription to clipboard: {e}");
                    }

                    let added_id = history_inner.lock().await.add_entry(
                        text.clone(),
                        HistoryEntryType::Speech,
                        Some(text.clone()),
                        None,
                        true,
                        None,
                        false,
                        None,
                        true,
                    );
                    created_entry_id = added_id.clone();
                    let _ = crate::services::history_events::emit_history_changed(
                        &app_clone,
                        added_id,
                        None,
                    );

                    let notification_settings =
                        config_inner.lock().await.settings().notifications.clone();
                    let duration_display = format!("Processed in {:.1}s", duration_secs);
                    let _ = notifications_inner.notify(
                        "speech_transcription_success",
                        NotificationLevel::Success,
                        "Speech transcribed",
                        Some(duration_display),
                        &notification_settings,
                    );
                }

                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text,
                        duration_secs,
                        entry_id: created_entry_id.clone(),
                    },
                );

                if let Some(clip_id) = clip_id {
                    let history = history_inner.lock().await;
                    match (runtime.keep_audio_on_success, created_entry_id) {
                        (true, Some(entry_id)) => {
                            let _ =
                                clip_inner.attach_to_entry(history.conn(), &clip_id, &entry_id);
                        }
                        _ => {
                            let _ = clip_inner.delete(history.conn(), &clip_id);
                        }
                    }
                }

                speech_inner.lock().await.set_transcribing(false);
            }
            Err(SpeechError::NoSpeechDetected) => {
                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text: String::new(),
                        duration_secs: start.elapsed().as_secs_f64(),
                        entry_id: None,
                    },
                );

                if let Some(clip_id) = clip_id {
                    let history = history_inner.lock().await;
                    let _ = clip_inner.delete(history.conn(), &clip_id);
                }

                let mut s = speech_inner.lock().await;
                let had_pending = s.take_pending_prompt().0.is_some();

                let title = if had_pending {
                    "Speech execution cancelled"
                } else {
                    "No speech detected"
                };
                let body = if had_pending {
                    "No speech detected — prompt execution cancelled"
                } else {
                    "No speech was detected in the recording"
                };

                let notification_settings =
                    config_inner.lock().await.settings().notifications.clone();
                let _ = notifications_inner.notify(
                    "speech_transcription_success",
                    NotificationLevel::Info,
                    title,
                    Some(body),
                    &notification_settings,
                );

                s.set_transcribing(false);
            }
            Err(e) => {
                let message = e.to_string();
                speech_inner.lock().await.set_pending_prompt(None, None);

                let entry_id = {
                    let history = history_inner.lock().await;
                    let entry_id = history.add_entry(
                        failure_input_content(duration_secs),
                        HistoryEntryType::Speech,
                        None,
                        None,
                        false,
                        Some(message.clone()),
                        false,
                        None,
                        true,
                    );

                    if let (Some(clip_id), Some(entry_id)) = (&clip_id, &entry_id) {
                        if let Err(e) =
                            clip_inner.attach_to_entry(history.conn(), clip_id, entry_id)
                        {
                            log::warn!("failed to attach audio clip to history entry: {e}");
                        }
                    }
                    entry_id
                };

                let has_audio = clip_id.is_some() && entry_id.is_some();
                let _ = crate::services::history_events::emit_history_changed(
                    &app_clone,
                    entry_id.clone(),
                    None,
                );

                let _ = app_clone.emit(
                    "speech-transcription-error",
                    SpeechTranscriptionError {
                        message: message.clone(),
                        recoverable: has_audio,
                        entry_id,
                        has_audio,
                        attempts: attempts_used,
                    },
                );

                let notification_settings =
                    config_inner.lock().await.settings().notifications.clone();
                let body = if has_audio {
                    format!(
                        "{message} — audio kept for {}h, retry from the menu or history",
                        runtime.audio_retention_hours
                    )
                } else {
                    message
                };
                let _ = notifications_inner.notify(
                    "speech_transcription_success",
                    NotificationLevel::Error,
                    "Transcription error",
                    Some(body),
                    &notification_settings,
                );

                speech_inner.lock().await.set_transcribing(false);
            }
        }
    });

    Ok(())
}

/// Replays a stored recording for an entry whose transcription failed.
/// The result only updates the entry and the clipboard — it is never pasted,
/// because the originating context is long gone by the time a retry happens.
#[tauri::command]
pub async fn retry_transcription(app: AppHandle, entry_id: String) -> crate::Result<()> {
    let speech_state = app.state::<Arc<Mutex<SpeechService>>>();
    let config_state = app.state::<Arc<Mutex<ConfigService>>>();
    let history_state = app.state::<Arc<Mutex<SqliteHistoryService>>>();
    let clip_state = app.state::<Arc<AudioClipStore>>();

    if !speech_state.lock().await.begin_retry(&entry_id) {
        log::debug!("retry_transcription: already retrying {entry_id}");
        return Ok(());
    }

    let clip = {
        let history = history_state.lock().await;
        clip_state.find_by_entry(history.conn(), &entry_id)
    };

    let Some(clip) = clip else {
        speech_state.lock().await.end_retry(&entry_id);
        return Err(Error::Speech(SpeechError::AudioUnavailable));
    };

    let runtime = match load_stt_runtime(&config_state).await {
        Ok(runtime) => runtime,
        Err(e) => {
            speech_state.lock().await.end_retry(&entry_id);
            return Err(Error::Speech(e));
        }
    };

    let clip_for_read = clip.clone();
    let clip_store_for_read = Arc::clone(&*clip_state);
    let wav_bytes = match tokio::task::spawn_blocking(move || {
        clip_store_for_read.read_bytes(&clip_for_read)
    })
    .await
    .map_err(|e| Error::Other(e.to_string()))?
    {
        Ok(bytes) => bytes,
        Err(e) => {
            speech_state.lock().await.end_retry(&entry_id);
            log::warn!("retry_transcription: clip unreadable for {entry_id}: {e}");
            return Err(Error::Speech(SpeechError::AudioUnavailable));
        }
    };

    let app_clone = app.clone();
    let speech_inner = Arc::clone(&*speech_state);
    let config_inner = Arc::clone(&*config_state);
    let history_inner = Arc::clone(&*history_state);
    let clip_inner = Arc::clone(&*clip_state);

    tokio::spawn(async move {
        let notifications = app_clone.state::<NotificationService>();
        let attempts = runtime.auto_retry_attempts.max(1);

        emit_retry(
            &app_clone,
            Some(entry_id.clone()),
            speech::retry::RetryNotice {
                attempt: 0,
                max_attempts: attempts,
                delay_secs: 0,
            },
            "Manual retry started".to_string(),
        );

        let event_entry_id = entry_id.clone();
        let result = speech::transcribe_with_retry(
            wav_bytes,
            &runtime.model,
            &runtime.options,
            attempts,
            |notice, error| {
                emit_retry(
                    &app_clone,
                    Some(event_entry_id.clone()),
                    notice,
                    error.to_string(),
                );
            },
        )
        .await;

        let notification_settings = config_inner.lock().await.settings().notifications.clone();

        match result {
            Ok(text) => {
                if let Err(e) = crate::services::clipboard::write_text(&app_clone, &text) {
                    log::error!("Failed to copy retried transcription to clipboard: {e}");
                }

                {
                    let history = history_inner.lock().await;
                    if let Err(e) = history.update_entry_result(
                        &entry_id,
                        Some(text.clone()),
                        Some(text.clone()),
                        true,
                        None,
                    ) {
                        log::error!("failed to update retried history entry: {e}");
                    }
                    let _ = clip_inner.delete_by_entry(history.conn(), &entry_id);
                }

                let _ = crate::services::history_events::emit_history_changed(
                    &app_clone,
                    None,
                    None,
                );
                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text,
                        duration_secs: clip.duration_secs,
                        entry_id: Some(entry_id.clone()),
                    },
                );

                let _ = notifications.notify(
                    "speech_transcription_success",
                    NotificationLevel::Success,
                    "Transcription recovered",
                    Some("Text copied to clipboard"),
                    &notification_settings,
                );
            }
            Err(e) => {
                let message = e.to_string();
                {
                    let history = history_inner.lock().await;
                    if let Err(e) = history.update_entry_result(
                        &entry_id,
                        None,
                        None,
                        false,
                        Some(message.clone()),
                    ) {
                        log::error!("failed to update failed retry entry: {e}");
                    }
                }
                let _ = crate::services::history_events::emit_history_changed(
                    &app_clone,
                    None,
                    None,
                );
                let _ = app_clone.emit(
                    "speech-transcription-error",
                    SpeechTranscriptionError {
                        message: message.clone(),
                        recoverable: true,
                        entry_id: Some(entry_id.clone()),
                        has_audio: true,
                        attempts,
                    },
                );
                let _ = notifications.notify(
                    "speech_transcription_success",
                    NotificationLevel::Error,
                    "Retry failed",
                    Some(message),
                    &notification_settings,
                );
            }
        }

        speech_inner.lock().await.end_retry(&entry_id);
    });

    Ok(())
}

#[tauri::command]
pub async fn get_audio_clip_info(
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    clips: State<'_, Arc<AudioClipStore>>,
    speech: State<'_, Arc<Mutex<SpeechService>>>,
    entry_id: String,
) -> crate::Result<AudioClipInfo> {
    let clip = {
        let history = history.lock().await;
        clips.find_by_entry(history.conn(), &entry_id)
    };

    let is_retrying = speech.lock().await.is_retrying(&entry_id);

    Ok(match clip {
        Some(clip) if clip.path.exists() => AudioClipInfo {
            has_audio: true,
            expires_at: Some(clip.expires_at),
            duration_secs: Some(clip.duration_secs),
            bytes: Some(clip.bytes),
            is_retrying,
        },
        _ => AudioClipInfo {
            is_retrying,
            ..AudioClipInfo::absent()
        },
    })
}

#[tauri::command]
pub async fn export_audio_clip(
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    clips: State<'_, Arc<AudioClipStore>>,
    entry_id: String,
    destination: String,
) -> crate::Result<()> {
    let clip = {
        let history = history.lock().await;
        clips.find_by_entry(history.conn(), &entry_id)
    };

    let Some(clip) = clip else {
        return Err(Error::Speech(SpeechError::AudioUnavailable));
    };

    let clips = Arc::clone(&*clips);
    tokio::task::spawn_blocking(move || clips.copy_to(&clip, &PathBuf::from(destination)))
        .await
        .map_err(|e| Error::Other(e.to_string()))?
        .map_err(SpeechError::from)?;

    log::info!("audio clip exported for entry {entry_id}");
    Ok(())
}

#[tauri::command]
pub async fn discard_audio_clip(
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    clips: State<'_, Arc<AudioClipStore>>,
    entry_id: String,
) -> crate::Result<()> {
    let history = history.lock().await;
    clips
        .delete_by_entry(history.conn(), &entry_id)
        .map_err(SpeechError::from)?;
    log::debug!("audio clip discarded for entry {entry_id}");
    Ok(())
}

#[tauri::command]
pub async fn get_recording_state(
    speech: State<'_, Arc<Mutex<SpeechService>>>,
) -> crate::Result<RecordingState> {
    let s = speech.lock().await;
    Ok(RecordingState {
        is_recording: s.is_recording(),
        is_transcribing: s.is_transcribing(),
        action_id: s.recording_action_id().map(String::from),
    })
}

#[tauri::command]
pub async fn get_stt_keyterms(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<KeytermsDoc> {
    let svc = config.lock().await;
    Ok(svc.read_stt_keyterms_file()?)
}

#[tauri::command]
pub async fn save_stt_keyterms(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    content: String,
) -> crate::Result<KeytermsDoc> {
    let mut svc = config.lock().await;
    let path_was_unset = svc.settings().surfaces.speech_to_text.keyterms_file.is_none();
    let doc = svc.write_stt_keyterms_file(&content)?;
    if path_was_unset {
        svc.save()?;
        let _ = app.emit("settings-changed", serde_json::json!({}));
    }
    let _ = app.emit("stt-keyterms-changed", ());
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_duration_is_formatted_as_minutes_and_seconds() {
        assert_eq!(format_clip_duration(0.0), "0:00");
        assert_eq!(format_clip_duration(9.4), "0:09");
        assert_eq!(format_clip_duration(125.0), "2:05");
    }

    #[test]
    fn failure_entry_keeps_duration_visible() {
        let content = failure_input_content(184.0);
        assert!(content.contains("3:04"));
        assert!(content.contains("transcription failed"));
    }
}
