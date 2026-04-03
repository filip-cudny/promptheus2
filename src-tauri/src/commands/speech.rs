use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::HistoryEntryType;
use crate::services::notification::NotificationLevel;
use crate::services::speech::{self, SpeechError};

use super::settings::AppState;

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
) -> Result<(), String> {
    let was_recording;
    let recording_result;

    {
        let mut s = state.lock().await;
        was_recording = s.speech.is_recording();

        if was_recording {
            recording_result = Some(s.speech.stop_recording());
        } else {
            recording_result = None;
            if let Err(e) = s.speech.start_recording(action_id) {
                let _ = app.emit("speech-error", SpeechErrorEvent { message: e.to_string() });
                return Err(e.to_string());
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
                "Recording Started",
                Some("Click Speech to Text again to stop."),
                &s.config.settings().notifications,
            );
        }
        return Ok(());
    }

    let stop_result = recording_result.unwrap();
    let _ = app.emit("speech-recording-stopped", ());

    match stop_result {
        Ok((wav_bytes, _sample_rate)) => {
            let speech_config = {
                let mut s = state.lock().await;
                let _ = s.notifications.notify(
                    "speech_recording_stop",
                    NotificationLevel::Info,
                    "Processing Audio",
                    Some("Transcribing your speech to text"),
                    &s.config.settings().notifications,
                );

                match s.config.settings().speech_to_text_model.clone() {
                    Some(config) => {
                        s.speech.set_transcribing(true);
                        config
                    }
                    None => {
                        let _ = app.emit("speech-error", SpeechErrorEvent {
                            message: "Speech-to-text model not configured".into(),
                        });
                        return Err("Speech-to-text model not configured".into());
                    }
                }
            };

            let app_clone = app.clone();

            tokio::spawn(async move {
                let state_inner = app_clone.state::<Mutex<AppState>>();
                let start = std::time::Instant::now();
                match speech::transcribe(wav_bytes, &speech_config).await {
                    Ok(text) => {
                        let duration_secs = start.elapsed().as_secs_f64();
                        let _ = app_clone.emit(
                            "speech-transcription-complete",
                            TranscriptionComplete {
                                text: text.clone(),
                                duration_secs,
                            },
                        );

                        let mut s = state_inner.lock().await;
                        s.speech.set_transcribing(false);
                        let (pending_id, pending_name) = s.speech.take_pending_prompt();

                        if let Some(skill_id) = pending_id {
                            let _ = app_clone.emit(
                                "speech-alternative-execute",
                                AlternativeExecutePayload {
                                    skill_id,
                                    skill_name: pending_name.unwrap_or_default(),
                                    text: text.clone(),
                                },
                            );
                        } else {
                            if let Err(e) = s.clipboard.set_text(&text) {
                                log::error!("Failed to copy transcription to clipboard: {e}");
                            }

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
                                "Transcription completed",
                                Some(duration_display),
                                &notification_settings,
                            );
                        }
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
                        s.speech.set_transcribing(false);
                        let had_pending = s.speech.take_pending_prompt().0.is_some();

                        let title = if had_pending {
                            "Speech Execution Cancelled"
                        } else {
                            "No Speech Detected"
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
                        s.speech.set_transcribing(false);
                        s.speech.set_pending_prompt(None, None);

                        let notification_settings = s.config.settings().notifications.clone();
                        let _ = s.notifications.notify(
                            "speech_transcription_success",
                            NotificationLevel::Error,
                            "Transcription Error",
                            Some(message),
                            &notification_settings,
                        );
                    }
                }
            });

            Ok(())
        }
        Err(e) => {
            let _ = app.emit("speech-error", SpeechErrorEvent { message: e.to_string() });
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_recording_state(
    state: State<'_, Mutex<AppState>>,
) -> Result<RecordingState, String> {
    let s = state.lock().await;
    Ok(RecordingState {
        is_recording: s.speech.is_recording(),
        is_transcribing: s.speech.is_transcribing(),
        action_id: s.speech.recording_action_id().map(String::from),
    })
}
