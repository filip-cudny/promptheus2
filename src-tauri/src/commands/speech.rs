use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::HistoryEntryType;
use crate::services::notification::NotificationLevel;
use crate::services::speech::{self, SpeechError};
use crate::Error;

use super::settings::AppState;

const MIN_RECORDING_SAMPLES_SECS: f64 = 1.0;

#[derive(Clone, Serialize)]
struct TranscriptionComplete {
    text: String,
    duration_secs: f64,
}

#[derive(Clone, Serialize)]
struct SpeechErrorEvent {
    message: String,
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

#[tauri::command]
pub async fn toggle_speech_recording(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    action_id: Option<String>,
) -> crate::Result<()> {
    let was_recording;
    let raw_audio;

    {
        let mut s = state.lock().await;

        if s.speech.is_transcribing() {
            log::debug!("toggle_speech_recording: ignored, transcription in progress");
            let notification_settings = s.config.settings().notifications.clone();
            let _ = s.notifications.notify(
                "speech_recording_start",
                NotificationLevel::Warning,
                "Transcription in progress",
                Some("Wait until it finishes"),
                &notification_settings,
            );
            return Ok(());
        }

        if !s.speech.is_recording() && s.speech.is_on_cooldown() {
            log::debug!("toggle_speech_recording: ignored, cooldown active");
            let notification_settings = s.config.settings().notifications.clone();
            let _ = s.notifications.notify(
                "speech_recording_start",
                NotificationLevel::Warning,
                "Cooldown active",
                Some("Try again in a moment"),
                &notification_settings,
            );
            return Ok(());
        }

        was_recording = s.speech.is_recording();

        if was_recording {
            match s.speech.stop_recording_raw() {
                Ok(audio) => {
                    s.speech.set_transcribing(true);
                    raw_audio = Some(audio);
                }
                Err(e) => {
                    let _ = app.emit("speech-error", SpeechErrorEvent { message: e.to_string() });
                    return Err(e.into());
                }
            }
        } else {
            raw_audio = None;
            if let Err(e) = s.speech.start_recording(action_id) {
                let _ = app.emit("speech-error", SpeechErrorEvent { message: e.to_string() });
                return Err(e.into());
            }
        }
    }

    if !was_recording {
        let _ = app.emit("speech-recording-started", ());
        {
            let s = state.lock().await;
            let _ = s.notifications.notify(
                "speech_recording_start",
                NotificationLevel::Info,
                "Recording started",
                Some("Click Speech to Text again to stop."),
                &s.config.settings().notifications,
            );
        }
        return Ok(());
    }

    let _ = app.emit("speech-recording-stopped", ());

    let (samples, sample_rate) = raw_audio.unwrap();

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
        let mut s = state.lock().await;
        s.speech.set_transcribing(false);
        s.speech.mark_transcription_finished();
        s.speech.set_pending_prompt(None, None);
        return Ok(());
    }

    let wav_bytes = tokio::task::spawn_blocking(move || {
        speech::encode_wav(&samples, sample_rate)
    })
    .await
    .map_err(|e| Error::Other(e.to_string()))?
    .map_err(|e| {
        let _ = app.emit("speech-error", SpeechErrorEvent { message: e.to_string() });
        Error::from(e)
    });

    let wav_bytes = match wav_bytes {
        Ok(bytes) => bytes,
        Err(e) => {
            let mut s = state.lock().await;
            s.speech.set_transcribing(false);
            s.speech.mark_transcription_finished();
            s.speech.set_pending_prompt(None, None);
            return Err(e);
        }
    };

    let (speech_config, stt_options) = {
        let mut s = state.lock().await;
        let _ = s.notifications.notify(
            "speech_recording_stop",
            NotificationLevel::Info,
            "Processing audio",
            Some("Transcribing your speech to text"),
            &s.config.settings().notifications,
        );

        let stt_prompt = s.config.stt_prompt();
        let keyterms = s.config.stt_keyterms();
        let stt_surface = s.config.settings().surfaces.speech_to_text.clone();

        match s.config.resolve_stt_model().cloned() {
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
                s.speech.set_transcribing(false);
                s.speech.mark_transcription_finished();
                s.speech.set_pending_prompt(None, None);
                let _ = app.emit("speech-error", SpeechErrorEvent {
                    message: "Speech-to-text model not configured".into(),
                });
                return Err(Error::Other("Speech-to-text model not configured".into()));
            }
        }
    };

    let app_clone = app.clone();

    tokio::spawn(async move {
        let state_inner = app_clone.state::<Mutex<AppState>>();
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

                let pending = {
                    let mut s = state_inner.lock().await;
                    s.speech.take_pending_prompt()
                };

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

                    let mut s = state_inner.lock().await;
                    s.history.add_entry(
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

                    let notification_settings = s.config.settings().notifications.clone();
                    let duration_display = format!("Processed in {:.1}s", duration_secs);
                    let _ = s.notifications.notify(
                        "speech_transcription_success",
                        NotificationLevel::Success,
                        "Speech transcribed",
                        Some(duration_display),
                        &notification_settings,
                    );
                }

                let mut s = state_inner.lock().await;
                s.speech.set_transcribing(false);
                s.speech.mark_transcription_finished();
            }
            Err(SpeechError::NoSpeechDetected) => {
                let _ = app_clone.emit(
                    "speech-transcription-complete",
                    TranscriptionComplete {
                        text: String::new(),
                        duration_secs: start.elapsed().as_secs_f64(),
                    },
                );

                let mut s = state_inner.lock().await;
                let had_pending = s.speech.take_pending_prompt().0.is_some();

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

                let notification_settings = s.config.settings().notifications.clone();
                let _ = s.notifications.notify(
                    "speech_transcription_success",
                    NotificationLevel::Info,
                    title,
                    Some(body),
                    &notification_settings,
                );

                s.speech.set_transcribing(false);
                s.speech.mark_transcription_finished();
            }
            Err(e) => {
                let message = e.to_string();
                let _ = app_clone.emit(
                    "speech-error",
                    SpeechErrorEvent {
                        message: message.clone(),
                    },
                );

                let mut s = state_inner.lock().await;
                s.speech.set_pending_prompt(None, None);

                let notification_settings = s.config.settings().notifications.clone();
                let _ = s.notifications.notify(
                    "speech_transcription_success",
                    NotificationLevel::Error,
                    "Transcription error",
                    Some(message),
                    &notification_settings,
                );

                s.speech.set_transcribing(false);
                s.speech.mark_transcription_finished();
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn get_recording_state(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<RecordingState> {
    let s = state.lock().await;
    Ok(RecordingState {
        is_recording: s.speech.is_recording(),
        is_transcribing: s.speech.is_transcribing(),
        action_id: s.speech.recording_action_id().map(String::from),
    })
}
