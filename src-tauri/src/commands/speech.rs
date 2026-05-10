use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::HistoryEntryType;
use crate::models::speech::{
    SpeechRecordingStartedEvent, SpeechRecordingStoppedEvent, SpeechTranscriptionError,
};
use crate::services::config::{ConfigService, KeytermsDoc};
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::speech::{self, SpeechError, SpeechService};
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

const MIN_RECORDING_SAMPLES_SECS: f64 = 1.0;

#[derive(Clone, Serialize)]
struct TranscriptionComplete {
    text: String,
    duration_secs: f64,
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

fn transcription_recoverable(error: &SpeechError) -> bool {
    !matches!(
        error,
        SpeechError::ApiKeyMissing | SpeechError::NoInputDevice | SpeechError::NoSupportedConfig
    )
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

    let was_recording;
    let raw_audio;
    let started_action_id;

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

        if !s.is_recording() && s.is_on_cooldown() {
            log::debug!("toggle_speech_recording: ignored, cooldown active");
            let notification_settings =
                config_state.lock().await.settings().notifications.clone();
            let _ = notifications.notify(
                "speech_recording_start",
                NotificationLevel::Warning,
                "Cooldown active",
                Some("Try again in a moment"),
                &notification_settings,
            );
            return Ok(());
        }

        was_recording = s.is_recording();

        if was_recording {
            let audio = s.stop_recording_raw()?;
            s.set_transcribing(true);
            started_action_id = None;
            raw_audio = Some(audio);
        } else {
            raw_audio = None;
            s.start_recording(action_id.clone())?;
            started_action_id = action_id;
        }
    }

    if !was_recording {
        let _ = app.emit(
            "speech-recording-started",
            SpeechRecordingStartedEvent {
                action_id: started_action_id,
            },
        );
        let notification_settings = config_state.lock().await.settings().notifications.clone();
        let _ = notifications.notify(
            "speech_recording_start",
            NotificationLevel::Info,
            "Recording started",
            Some("Click Speech to Text again to stop."),
            &notification_settings,
        );
        return Ok(());
    }

    let (samples, sample_rate) = raw_audio.unwrap();
    let had_audio = !samples.is_empty();

    let _ = app.emit(
        "speech-recording-stopped",
        SpeechRecordingStoppedEvent { had_audio },
    );

    let min_samples = (sample_rate as f64 * MIN_RECORDING_SAMPLES_SECS) as usize;
    if samples.len() < min_samples {
        log::debug!(
            "toggle_speech_recording: recording too short ({} samples, need {}), discarding",
            samples.len(),
            min_samples,
        );
        let _ = app.emit(
            "speech-transcription-complete",
            TranscriptionComplete { text: String::new(), duration_secs: 0.0 },
        );
        let mut s = speech_state.lock().await;
        s.set_transcribing(false);
        s.mark_transcription_finished();
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
            s.mark_transcription_finished();
            s.set_pending_prompt(None, None);
            return Err(e.into());
        }
    };

    let (speech_config, stt_options) = {
        let config_guard = config_state.lock().await;
        let _ = notifications.notify(
            "speech_recording_stop",
            NotificationLevel::Info,
            "Processing audio",
            Some("Transcribing your speech to text"),
            &config_guard.settings().notifications,
        );

        let stt_prompt = config_guard.stt_prompt();
        let keyterms = config_guard.stt_keyterms();
        let stt_surface = config_guard.settings().surfaces.speech_to_text.clone();

        match config_guard.resolve_stt_model().cloned() {
            Some(config) => {
                let options = crate::services::speech::SttOptions {
                    language: stt_surface.language,
                    no_verbatim: stt_surface.no_verbatim,
                    prompt: stt_prompt,
                    keyterms,
                };
                (config, options)
            }
            None => {
                drop(config_guard);
                let mut s = speech_state.lock().await;
                s.set_transcribing(false);
                s.mark_transcription_finished();
                s.set_pending_prompt(None, None);
                return Err(Error::Speech(SpeechError::ApiKeyMissing));
            }
        }
    };

    let app_clone = app.clone();
    let speech_inner = Arc::clone(&*speech_state);
    let config_inner = Arc::clone(&*config_state);
    let history_inner = Arc::clone(&*history_state);

    tokio::spawn(async move {
        let notifications_inner = app_clone.state::<NotificationService>();
        let start = std::time::Instant::now();
        match speech::transcribe(wav_bytes, &speech_config, &stt_options).await {
            Ok(text) => {
                let duration_secs = start.elapsed().as_secs_f64();
                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text: text.clone(),
                        duration_secs,
                    },
                );

                let pending = speech_inner.lock().await.take_pending_prompt();

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
                        Some(text),
                        None,
                        true,
                        None,
                        false,
                        None,
                        true,
                    );
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

                let mut s = speech_inner.lock().await;
                s.set_transcribing(false);
                s.mark_transcription_finished();
            }
            Err(SpeechError::NoSpeechDetected) => {
                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text: String::new(),
                        duration_secs: start.elapsed().as_secs_f64(),
                    },
                );

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
                s.mark_transcription_finished();
            }
            Err(e) => {
                let message = e.to_string();
                let recoverable = transcription_recoverable(&e);
                let _ = app_clone.emit(
                    "speech-transcription-error",
                    SpeechTranscriptionError {
                        message: message.clone(),
                        recoverable,
                    },
                );

                let mut s = speech_inner.lock().await;
                s.set_pending_prompt(None, None);

                let notification_settings =
                    config_inner.lock().await.settings().notifications.clone();
                let _ = notifications_inner.notify(
                    "speech_transcription_success",
                    NotificationLevel::Error,
                    "Transcription error",
                    Some(message),
                    &notification_settings,
                );

                s.set_transcribing(false);
                s.mark_transcription_finished();
            }
        }
    });

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
