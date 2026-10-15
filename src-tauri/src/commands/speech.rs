use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::{HistoryEntryType, InvocationSource, OutputRoute};
use crate::models::settings::ModelConfig;
use crate::models::speech::{
    AudioClipInfo, SpeechRecordingStartedEvent, SpeechRecordingStoppedEvent,
    SpeechTranscriptionError, SpeechTranscriptionRetry,
};
use crate::commands::settings::apply_settings_reload;
use crate::services::ai::AiService;
use crate::commands::clipboard::store_pending_copy;
use crate::services::clipboard::{ClipboardFingerprint, ClipboardService};
use crate::services::config::{ConfigService, KeytermsDoc};
use crate::services::context::ContextManagerService;
use crate::services::execution::skill_executor::run_skill_stream;
use crate::services::execution::PromptExecutionService;
use crate::services::recent_apps::RecentAppsState;
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::skill::SkillService;
use crate::services::speech::widget::{configured_providers, pill_targets, RecordingWidget};
use crate::services::speech::{
    self, webview_provider_of, AudioClipStore, PendingOrigin, PendingPrompt, SpeechError,
    SpeechService, SttOptions,
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

#[derive(Debug, PartialEq)]
struct ChatRequest {
    skill_id: String,
    initial_input: String,
    skill_model: Option<String>,
    auto_send: bool,
    source: InvocationSource,
}

#[derive(Debug, PartialEq)]
enum TranscriptPlan {
    AlternativeExecute,
    NoSpeech,
    SkillRun { skill_id: String },
    Dictation,
    Chat(ChatRequest),
    Webview { provider_id: String },
}

fn transcript_plan(
    pending: Option<&PendingPrompt>,
    stop_chat: bool,
    text: &str,
    skill_model: Option<String>,
) -> TranscriptPlan {
    let chat = |skill_id: String, skill_model| {
        TranscriptPlan::Chat(ChatRequest {
            skill_id,
            initial_input: text.to_string(),
            skill_model,
            auto_send: true,
            source: InvocationSource::Pill,
        })
    };
    match pending {
        Some(p) if p.origin == PendingOrigin::ShiftClick => TranscriptPlan::AlternativeExecute,
        Some(_) if text.trim().is_empty() => TranscriptPlan::NoSpeech,
        Some(p) if webview_provider_of(&p.id).is_some() => TranscriptPlan::Webview {
            provider_id: webview_provider_of(&p.id).unwrap_or_default().to_string(),
        },
        Some(p) if stop_chat => chat(p.id.clone(), skill_model),
        Some(p) => TranscriptPlan::SkillRun { skill_id: p.id.clone() },
        None if stop_chat => chat(String::new(), None),
        None => TranscriptPlan::Dictation,
    }
}

async fn open_chat(app: &AppHandle, request: ChatRequest, copy_token: String) {
    log::debug!("stop chat: opening the chat, skill={}", request.skill_id);
    if let Err(e) = super::conversation_dialog::open_conversation_dialog(
        app.clone(),
        request.skill_id,
        String::new(),
        request.skill_model,
        None,
        None,
        Some(request.initial_input),
        Some(request.auto_send),
        None,
        Some(request.source),
        Some(copy_token),
    )
    .await
    {
        log::warn!("stop chat: opening the chat failed: {e}");
    }
}

async fn snapshot_clipboard(app: &AppHandle) -> String {
    let service = (*app.state::<ClipboardService>()).clone();
    let fingerprint = tokio::task::spawn_blocking(move || service.fingerprint())
        .await
        .unwrap_or(ClipboardFingerprint::Unreadable);
    store_pending_copy(fingerprint)
}

async fn record_speech_entry(
    app: &AppHandle,
    history: &Arc<Mutex<SqliteHistoryService>>,
    text: &str,
) -> Option<String> {
    let added_id = history.lock().await.add_entry(
        text.to_string(),
        HistoryEntryType::Speech,
        Some(text.to_string()),
        None,
        true,
        None,
        false,
        None,
        true,
    );
    let _ = crate::services::history_events::emit_history_changed(app, added_id.clone(), None);
    added_id
}

async fn run_pill_skill(app: &AppHandle, skill_id: String, text: String, generation: u64) {
    let widget = app.state::<RecordingWidget>();
    log::debug!(
        "pill skill run: start skill={skill_id} route={} generation={generation}",
        OutputRoute::Clipboard.as_str()
    );
    let started = std::time::Instant::now();
    let result = run_skill_stream(
        app.clone(),
        Arc::clone(&*app.state::<Arc<Mutex<ConfigService>>>()),
        (*app.state::<ClipboardService>()).clone(),
        Arc::clone(&*app.state::<Arc<Mutex<AiService>>>()),
        Arc::clone(&*app.state::<Arc<Mutex<SkillService>>>()),
        Arc::clone(&*app.state::<Arc<Mutex<ContextManagerService>>>()),
        Arc::clone(&*app.state::<Arc<Mutex<SqliteHistoryService>>>()),
        (*app.state::<NotificationService>()).clone(),
        Arc::clone(&*app.state::<Arc<Mutex<PromptExecutionService>>>()),
        Arc::clone(&*app.state::<Arc<RecentAppsState>>()),
        skill_id.clone(),
        Some(text),
        InvocationSource::Pill,
        OutputRoute::Clipboard,
        false,
        Channel::new(|_| Ok(())),
    )
    .await;
    log::debug!(
        "pill skill run: end skill={skill_id} ok={} elapsed_ms={} generation={generation}",
        matches!(result, Ok(Some(_))),
        started.elapsed().as_millis()
    );

    match result {
        Ok(Some(_)) => widget.done_if_current(app, generation).await,
        Ok(None) => widget.close_if_current(app, generation).await,
        Err(e) => {
            log::warn!("pill skill run failed: {e}");
            widget.close_if_current(app, generation).await;
        }
    }
}

async fn notify_no_speech(app: &AppHandle, config: &Arc<Mutex<ConfigService>>) {
    let notification_settings = config.lock().await.settings().notifications.clone();
    let _ = app.state::<NotificationService>().notify(
        "speech_transcription_success",
        NotificationLevel::Error,
        "No speech detected",
        Some("No speech was detected in the recording"),
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
    let widget = app.state::<RecordingWidget>();

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
        let widget_shown = match started_session {
            Some(session) => widget.show(&app, session).await,
            None => false,
        };
        if !widget_shown {
            let _ = notifications.notify(
                "speech_recording_start",
                NotificationLevel::Info,
                "Recording started",
                Some("Click Speech to Text again to stop."),
                &notification_settings,
            );
        }
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
    widget.processing(&app).await;
    let generation = widget.generation().await;
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
        widget.close(&app).await;
        let mut s = speech_state.lock().await;
        s.set_transcribing(false);
        s.clear_pending_prompt();
        return Ok(());
    }

    let wav_bytes = match tokio::task::spawn_blocking(move || speech::encode_wav(&samples, sample_rate))
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    {
        Ok(bytes) => bytes,
        Err(e) => {
            widget.close(&app).await;
            let mut s = speech_state.lock().await;
            s.set_transcribing(false);
            s.clear_pending_prompt();
            return Err(e.into());
        }
    };

    let runtime = match load_stt_runtime(&config_state).await {
        Ok(runtime) => {
            if !widget.is_shown().await {
                let notification_settings =
                    config_state.lock().await.settings().notifications.clone();
                let _ = notifications.notify(
                    "speech_recording_stop",
                    NotificationLevel::Info,
                    "Processing audio",
                    Some("Transcribing your speech to text"),
                    &notification_settings,
                );
            }
            runtime
        }
        Err(e) => {
            widget.close(&app).await;
            let mut s = speech_state.lock().await;
            s.set_transcribing(false);
            s.clear_pending_prompt();
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
        let widget_inner = app_clone.state::<RecordingWidget>();
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
                let (pending, stop_chat) = speech_inner.lock().await.take_pending_prompt();
                widget_inner.refresh_model(&app_clone).await;
                let mut created_entry_id = None;

                let skill_model = match pending.as_ref() {
                    Some(p) if p.origin == PendingOrigin::Pill => {
                        let skills = app_clone.state::<Arc<Mutex<SkillService>>>();
                        let model = skills.lock().await.get_skill(&p.id).and_then(|s| s.model.clone());
                        model
                    }
                    _ => None,
                };

                match transcript_plan(pending.as_ref(), stop_chat, &text, skill_model) {
                    TranscriptPlan::AlternativeExecute => {
                        let pending = pending.expect("plan implies a pending prompt");
                        widget_inner.close(&app_clone).await;
                        let _ = app_clone.emit(
                            "speech-alternative-execute",
                            AlternativeExecutePayload {
                                skill_id: pending.id,
                                skill_name: pending.name,
                                text: text.clone(),
                            },
                        );
                    }
                    TranscriptPlan::NoSpeech => {
                        widget_inner.close_if_current(&app_clone, generation).await;
                        notify_no_speech(&app_clone, &config_inner).await;
                    }
                    TranscriptPlan::SkillRun { skill_id } => {
                        run_pill_skill(&app_clone, skill_id, text.clone(), generation).await;
                    }
                    TranscriptPlan::Dictation => {
                        if let Err(e) = crate::services::clipboard::write_text(&app_clone, &text) {
                            log::error!("Failed to copy transcription to clipboard: {e}");
                        }

                        created_entry_id = record_speech_entry(&app_clone, &history_inner, &text).await;

                        if widget_inner.is_shown().await {
                            widget_inner.done(&app_clone).await;
                        } else {
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
                    }
                    TranscriptPlan::Chat(request) => {
                        created_entry_id = record_speech_entry(&app_clone, &history_inner, &text).await;
                        widget_inner.close_if_current(&app_clone, generation).await;
                        let copy_token = snapshot_clipboard(&app_clone).await;
                        open_chat(&app_clone, request, copy_token).await;
                    }
                    TranscriptPlan::Webview { provider_id } => {
                        created_entry_id = record_speech_entry(&app_clone, &history_inner, &text).await;
                        widget_inner.close_if_current(&app_clone, generation).await;
                        log::debug!("transcript: sending to the provider chat, provider={provider_id}");
                        crate::services::ai_webview::send_to_provider_chat(
                            &app_clone,
                            &provider_id,
                            text.clone(),
                        )
                        .await;
                    }
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
                widget_inner.close(&app_clone).await;
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
                widget_inner.close(&app_clone).await;
                let message = e.to_string();
                speech_inner.lock().await.clear_pending_prompt();

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
pub async fn pause_speech_recording(app: AppHandle) -> crate::Result<()> {
    let speech_state = app.state::<Arc<Mutex<SpeechService>>>();
    {
        let mut s = speech_state.lock().await;
        s.pause()?;
        log::info!("pause_speech_recording: session={}", s.session());
    }
    app.state::<RecordingWidget>().pause(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn resume_speech_recording(app: AppHandle) -> crate::Result<()> {
    let speech_state = app.state::<Arc<Mutex<SpeechService>>>();
    {
        let mut s = speech_state.lock().await;
        s.resume()?;
        log::info!("resume_speech_recording: session={}", s.session());
    }
    app.state::<RecordingWidget>().resume(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_speech_recording(app: AppHandle) -> crate::Result<()> {
    let speech_state = app.state::<Arc<Mutex<SpeechService>>>();
    {
        let mut s = speech_state.lock().await;
        if !s.is_recording() || s.is_transcribing() {
            log::debug!("cancel_speech_recording: ignored, not recording");
            return Ok(());
        }
        s.cancel_recording()?;
        log::info!("cancel_speech_recording: session={}", s.session());
    }

    app.state::<RecordingWidget>().close(&app).await;
    let _ = app.emit(
        "speech-recording-stopped",
        SpeechRecordingStoppedEvent { had_audio: false },
    );
    let _ = app.emit(
        "speech-transcription-complete",
        TranscriptionComplete { text: String::new(), duration_secs: 0.0, entry_id: None },
    );
    Ok(())
}

async fn recording_widget_shown(app: &AppHandle, intent: &str) -> bool {
    let shown = app.state::<RecordingWidget>().is_shown().await;
    if !shown {
        log::debug!("recording widget intent {intent} ignored: widget not shown");
    }
    shown
}

#[tauri::command]
pub async fn select_recording_widget_target(app: AppHandle, id: String) -> crate::Result<()> {
    log::debug!("recording widget intent: select target id={id}");
    if !recording_widget_shown(&app, "select target").await {
        return Ok(());
    }
    let providers = configured_providers(&app).await;
    let label = match app.try_state::<Arc<Mutex<SkillService>>>() {
        Some(skills) => pill_targets(skills.lock().await.list_skills(), &providers)
            .into_iter()
            .find(|(target_id, _)| *target_id == id)
            .map(|(_, label)| label),
        None => None,
    };
    let Some(label) = label else {
        log::debug!("recording widget intent select target ignored: unknown id={id}");
        return Ok(());
    };
    let selected = app
        .state::<Arc<Mutex<SpeechService>>>()
        .lock()
        .await
        .select_target(&id, &label);
    match selected {
        Ok(()) => app.state::<RecordingWidget>().refresh_model(&app).await,
        Err(_) => log::debug!("recording widget intent select target ignored: target locked"),
    }
    Ok(())
}

#[tauri::command]
pub async fn set_recording_widget_dropdown(
    app: AppHandle,
    open: bool,
    rows: u32,
) -> crate::Result<&'static str> {
    log::debug!("recording widget intent: dropdown open={open} rows={rows}");
    let widget = app.state::<RecordingWidget>();
    if !open {
        widget.close_dropdown(&app).await;
        return Ok("down");
    }
    let up = widget.open_dropdown(&app, rows).await?;
    Ok(if up { "up" } else { "down" })
}

#[tauri::command]
pub async fn set_recording_widget_preview(app: AppHandle, open: bool) -> crate::Result<&'static str> {
    log::debug!("recording widget intent: preview open={open}");
    let widget = app.state::<RecordingWidget>();
    if !open {
        widget.close_preview(&app).await;
        return Ok("down");
    }
    let up = widget.open_preview(&app).await?;
    Ok(if up { "up" } else { "down" })
}

#[tauri::command]
pub async fn remove_recording_widget_item(
    app: AppHandle,
    index: u32,
    context_version: u64,
) -> crate::Result<()> {
    log::debug!("recording widget intent: remove item index={index} version={context_version}");
    if !recording_widget_shown(&app, "remove item").await {
        return Ok(());
    }
    super::context::remove_item_for_pill(&app, index as usize, context_version).await
}

#[tauri::command]
pub async fn stop_chat_speech_recording(app: AppHandle) -> crate::Result<()> {
    log::debug!("recording widget intent: stop chat");
    if !recording_widget_shown(&app, "stop chat").await {
        return Ok(());
    }
    let (recording, requested) = {
        let speech = app.state::<Arc<Mutex<SpeechService>>>();
        let mut s = speech.lock().await;
        (s.is_recording(), s.request_stop_chat())
    };
    if requested.is_err() {
        log::debug!("recording widget intent stop chat ignored: target locked");
        return Ok(());
    }
    log::debug!("stop chat requested, recording={recording}");
    if recording {
        toggle_speech_recording(app, None).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn paste_into_recording_widget(app: AppHandle) -> crate::Result<()> {
    log::debug!("recording widget intent: paste");
    if !recording_widget_shown(&app, "paste").await {
        return Ok(());
    }
    super::context::paste_item_for_pill(&app).await
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
    let mutated = config.lock().await.mutate(|c| {
        let path_was_unset = c.settings().surfaces.speech_to_text.keyterms_file.is_none();
        c.write_stt_keyterms_file(&content)
            .map(|doc| (doc, path_was_unset))
    })?;
    if mutated.reloaded {
        apply_settings_reload(&app, "reload").await?;
    }
    let (doc, path_was_unset) = mutated.value?;
    if path_was_unset {
        let _ = app.emit("settings-changed", serde_json::json!({}));
    }
    let _ = app.emit("stt-keyterms-changed", ());
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pill_target() -> PendingPrompt {
        PendingPrompt {
            id: "summarize".into(),
            name: "Summarize".into(),
            origin: PendingOrigin::Pill,
        }
    }

    fn pill_chat(skill_id: &str, model: Option<&str>) -> TranscriptPlan {
        TranscriptPlan::Chat(ChatRequest {
            skill_id: skill_id.into(),
            initial_input: "hello".into(),
            skill_model: model.map(String::from),
            auto_send: true,
            source: InvocationSource::Pill,
        })
    }

    #[test]
    fn plan_skill_target_with_stop_chat_opens_chat_without_skill_run() {
        let plan = transcript_plan(Some(&pill_target()), true, "hello", Some("m1".into()));
        assert_eq!(plan, pill_chat("summarize", Some("m1")));
    }

    #[test]
    fn plan_dictation_with_stop_chat_opens_chat_without_clipboard_write() {
        assert_eq!(transcript_plan(None, true, "hello", None), pill_chat("", None));
    }

    #[test]
    fn plan_skill_target_without_stop_chat_runs_skill() {
        assert_eq!(
            transcript_plan(Some(&pill_target()), false, "hello", None),
            TranscriptPlan::SkillRun { skill_id: "summarize".into() }
        );
    }

    #[test]
    fn plan_dictation_without_stop_chat_writes_clipboard_and_done() {
        assert_eq!(
            transcript_plan(None, false, "hello", None),
            TranscriptPlan::Dictation
        );
    }

    #[test]
    fn plan_empty_transcript_with_skill_and_stop_chat_opens_no_chat() {
        assert_eq!(
            transcript_plan(Some(&pill_target()), true, "  ", None),
            TranscriptPlan::NoSpeech
        );
    }

    fn webview_target() -> PendingPrompt {
        PendingPrompt { id: "webview:claude".into(), name: "Claude".into(), ..pill_target() }
    }

    #[test]
    fn plan_webview_target_routes_to_the_provider_with_and_without_stop_chat() {
        for stop_chat in [true, false] {
            assert_eq!(
                transcript_plan(Some(&webview_target()), stop_chat, "hello", None),
                TranscriptPlan::Webview { provider_id: "claude".into() }
            );
        }
    }

    #[test]
    fn plan_webview_target_with_an_empty_transcript_is_no_speech() {
        for text in ["", "  "] {
            assert_eq!(
                transcript_plan(Some(&webview_target()), true, text, None),
                TranscriptPlan::NoSpeech
            );
        }
    }

    #[test]
    fn plan_shift_click_emits_alternative_execute() {
        let pending = PendingPrompt { origin: PendingOrigin::ShiftClick, ..pill_target() };
        assert_eq!(
            transcript_plan(Some(&pending), true, "hello", None),
            TranscriptPlan::AlternativeExecute
        );
    }

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
