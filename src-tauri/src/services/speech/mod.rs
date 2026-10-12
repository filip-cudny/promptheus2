mod clip_store;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod pill_previews;
mod recorder;
pub mod reminder;
pub mod retry;
mod transcriber;
pub mod widget;

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};

use recorder::{find_input_device, negotiate_sample_rate};

pub use clip_store::{AudioClipStore, ClipStoreError};
pub use recorder::encode_wav;
pub use retry::transcribe_with_retry;
pub use transcriber::{transcribe, SttOptions};

const TOGGLE_DEBOUNCE_MS: u128 = 250;

pub const DICTATION_ID: &str = "__dictation__";
pub const DICTATION_LABEL: &str = "Dictation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOrigin {
    ShiftClick,
    Pill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPrompt {
    pub id: String,
    pub name: String,
    pub origin: PendingOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetLocked;

/// Whether a failed transcription is worth sending again unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureKind {
    Transient,
    Permanent,
}

#[derive(Debug, thiserror::Error)]
pub enum SpeechError {
    #[error("Already recording")]
    AlreadyRecording,
    #[error("Not recording")]
    NotRecording,
    #[error("No input device found")]
    NoInputDevice,
    #[error("No supported audio configuration found")]
    NoSupportedConfig,
    #[error("Failed to build audio stream: {0}")]
    StreamBuild(String),
    #[error("WAV encoding error: {0}")]
    WavEncode(String),
    #[error("{message}")]
    Transcription {
        message: String,
        kind: FailureKind,
        retry_after_secs: Option<u64>,
    },
    #[error("API key not configured")]
    ApiKeyMissing,
    #[error("No speech detected")]
    NoSpeechDetected,
    #[error("Recording failed: {0}")]
    RecordingFailed(String),
    #[error("No audio available for this transcription")]
    AudioUnavailable,
    #[error("Audio storage error: {0}")]
    ClipStore(#[from] ClipStoreError),
}

impl SpeechError {
    pub fn transient(message: impl Into<String>) -> Self {
        Self::Transcription {
            message: message.into(),
            kind: FailureKind::Transient,
            retry_after_secs: None,
        }
    }

    pub fn transient_after(message: impl Into<String>, retry_after_secs: Option<u64>) -> Self {
        Self::Transcription {
            message: message.into(),
            kind: FailureKind::Transient,
            retry_after_secs,
        }
    }

    pub fn permanent(message: impl Into<String>) -> Self {
        Self::Transcription {
            message: message.into(),
            kind: FailureKind::Permanent,
            retry_after_secs: None,
        }
    }

    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Transcription {
                kind: FailureKind::Transient,
                ..
            }
        )
    }

    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::Transcription {
                retry_after_secs, ..
            } => *retry_after_secs,
            _ => None,
        }
    }
}

pub struct SpeechService {
    is_recording: bool,
    is_transcribing: bool,
    recording_action_id: Option<String>,
    pending: Option<PendingPrompt>,
    taken: Option<PendingPrompt>,
    target_locked: bool,
    stop_chat: bool,
    audio_buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    stop_sender: Option<mpsc::Sender<()>>,
    last_toggle: Option<Instant>,
    session: u64,
    started_at: Option<Instant>,
    paused: Arc<AtomicBool>,
    paused_at: Option<Instant>,
    paused_total: Duration,
    retrying_entries: HashSet<String>,
}

fn append_unless_paused(buffer: &Mutex<Vec<i16>>, paused: &AtomicBool, data: &[i16]) {
    if paused.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut buf) = buffer.lock() {
        buf.extend_from_slice(data);
    }
}

fn elapsed_excluding_pauses(
    now: Instant,
    started_at: Instant,
    paused_total: Duration,
    paused_at: Option<Instant>,
) -> Duration {
    let current_pause = paused_at.map_or(Duration::ZERO, |t| now.saturating_duration_since(t));
    now.saturating_duration_since(started_at)
        .saturating_sub(paused_total)
        .saturating_sub(current_pause)
}

impl SpeechService {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            is_transcribing: false,
            recording_action_id: None,
            pending: None,
            taken: None,
            target_locked: false,
            stop_chat: false,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            stop_sender: None,
            last_toggle: None,
            session: 0,
            started_at: None,
            paused: Arc::new(AtomicBool::new(false)),
            paused_at: None,
            paused_total: Duration::ZERO,
            retrying_entries: HashSet::new(),
        }
    }

    pub fn start_recording(&mut self, action_id: Option<String>) -> Result<u32, SpeechError> {
        if self.is_recording {
            return Err(SpeechError::AlreadyRecording);
        }

        self.reset_target_for_start(action_id.as_deref());

        let device = find_input_device()?;
        let sample_rate = negotiate_sample_rate(&device)?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = Arc::clone(&paused);

        let (stop_tx, stop_rx) = mpsc::channel();

        std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    append_unless_paused(&buffer_clone, &paused_clone, data);
                },
                |err| {
                    log::error!("Audio stream error: {err}");
                },
                None,
            );

            match stream {
                Ok(s) => {
                    if s.play().is_ok() {
                        let _ = stop_rx.recv();
                    }
                    drop(s);
                }
                Err(e) => {
                    log::error!("Failed to build audio stream: {e}");
                }
            }
        });

        self.audio_buffer = buffer;
        self.paused = paused;
        self.paused_at = None;
        self.paused_total = Duration::ZERO;
        self.sample_rate = sample_rate;
        self.is_recording = true;
        self.recording_action_id = action_id;
        self.stop_sender = Some(stop_tx);
        self.session = self.session.wrapping_add(1);
        self.started_at = Some(Instant::now());

        Ok(sample_rate)
    }

    pub fn stop_recording_raw(&mut self) -> Result<(Vec<i16>, u32), SpeechError> {
        if !self.is_recording {
            return Err(SpeechError::NotRecording);
        }

        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }

        let samples = {
            let mut buf = self.audio_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        let sample_rate = self.sample_rate;
        self.is_recording = false;
        self.recording_action_id = None;
        self.started_at = None;
        self.paused.store(false, Ordering::Relaxed);
        self.paused_at = None;
        self.paused_total = Duration::ZERO;

        Ok((samples, sample_rate))
    }

    pub fn cancel_recording(&mut self) -> Result<(), SpeechError> {
        self.stop_recording_raw()?;
        self.clear_pending_prompt();
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), SpeechError> {
        if !self.is_recording {
            return Err(SpeechError::NotRecording);
        }
        if self.paused_at.is_none() {
            self.paused.store(true, Ordering::Relaxed);
            self.paused_at = Some(Instant::now());
        }
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), SpeechError> {
        if !self.is_recording {
            return Err(SpeechError::NotRecording);
        }
        if let Some(paused_at) = self.paused_at.take() {
            self.paused_total += paused_at.elapsed();
            self.paused.store(false, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// Recording time without paused spans.
    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.map(|started_at| {
            elapsed_excluding_pauses(Instant::now(), started_at, self.paused_total, self.paused_at)
        })
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Monotonic id of the current recording, used by background tasks to tell
    /// whether the session they were started for is still the active one.
    pub fn session(&self) -> u64 {
        self.session
    }

    pub fn audio_buffer(&self) -> Arc<Mutex<Vec<i16>>> {
        Arc::clone(&self.audio_buffer)
    }

    pub fn is_transcribing(&self) -> bool {
        self.is_transcribing
    }

    pub fn set_transcribing(&mut self, value: bool) {
        self.is_transcribing = value;
    }

    pub fn recording_action_id(&self) -> Option<&str> {
        self.recording_action_id.as_deref()
    }

    pub fn set_pending_prompt(&mut self, id: String, name: String, origin: PendingOrigin) {
        self.pending = Some(PendingPrompt { id, name, origin });
    }

    pub fn clear_pending_prompt(&mut self) {
        self.pending = None;
    }

    pub fn pending_prompt(&self) -> Option<&PendingPrompt> {
        self.pending.as_ref()
    }

    pub fn shown_target(&self) -> Option<&PendingPrompt> {
        if self.target_locked {
            self.taken.as_ref()
        } else {
            self.pending.as_ref()
        }
    }

    pub fn target_locked(&self) -> bool {
        self.target_locked
    }

    /// Keeps only the Shift+click skill that started this very recording.
    pub fn reset_target_for_start(&mut self, action_id: Option<&str>) {
        let keep = self.pending.as_ref().is_some_and(|pending| {
            pending.origin == PendingOrigin::ShiftClick && Some(pending.id.as_str()) == action_id
        });
        if !keep {
            self.pending = None;
        }
        self.target_locked = false;
        self.stop_chat = false;
    }

    pub fn request_stop_chat(&mut self) -> Result<(), TargetLocked> {
        if self.target_locked {
            return Err(TargetLocked);
        }
        self.stop_chat = true;
        Ok(())
    }

    pub fn select_target(&mut self, id: &str, name: &str) -> Result<(), TargetLocked> {
        if self.target_locked {
            return Err(TargetLocked);
        }
        self.pending = (id != DICTATION_ID).then(|| PendingPrompt {
            id: id.to_string(),
            name: name.to_string(),
            origin: PendingOrigin::Pill,
        });
        Ok(())
    }

    /// Takes the pending skill and locks the target in one step, so a pill
    /// selection cannot slip in between.
    pub fn take_pending_prompt(&mut self) -> (Option<PendingPrompt>, bool) {
        self.target_locked = true;
        self.taken = self.pending.take();
        (self.taken.clone(), self.stop_chat)
    }

    /// Manual retries are tracked per history entry rather than through
    /// `is_transcribing`, so replaying an old clip never blocks a new recording.
    pub fn begin_retry(&mut self, entry_id: &str) -> bool {
        self.retrying_entries.insert(entry_id.to_string())
    }

    pub fn end_retry(&mut self, entry_id: &str) {
        self.retrying_entries.remove(entry_id);
    }

    pub fn is_retrying(&self, entry_id: &str) -> bool {
        self.retrying_entries.contains(entry_id)
    }

    pub fn retrying_entries(&self) -> HashSet<String> {
        self.retrying_entries.clone()
    }

    pub fn mark_toggle(&mut self) {
        self.last_toggle = Some(Instant::now());
    }

    pub fn is_debouncing(&self) -> bool {
        self.last_toggle
            .map(|t| t.elapsed().as_millis() < TOGGLE_DEBOUNCE_MS)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_recording() {
        let service = SpeechService::new();
        assert!(!service.is_recording());
        assert!(service.recording_action_id().is_none());
    }

    #[test]
    fn stop_without_recording_returns_error() {
        let mut service = SpeechService::new();
        let result = service.stop_recording_raw();
        assert!(result.is_err());
    }

    #[test]
    fn append_unless_paused_drops_samples_while_paused() {
        let buffer = Mutex::new(Vec::new());
        let paused = AtomicBool::new(true);
        append_unless_paused(&buffer, &paused, &[1, 2, 3]);
        assert!(buffer.lock().unwrap().is_empty());

        paused.store(false, Ordering::Relaxed);
        append_unless_paused(&buffer, &paused, &[4, 5]);
        assert_eq!(*buffer.lock().unwrap(), vec![4, 5]);
    }

    #[test]
    fn elapsed_excludes_paused_span() {
        let started = Instant::now();
        let now = started + Duration::from_secs(5);
        let elapsed = elapsed_excluding_pauses(now, started, Duration::from_secs(2), None);
        assert_eq!(elapsed, Duration::from_secs(3));
    }

    #[test]
    fn elapsed_excludes_current_pause() {
        let started = Instant::now();
        let now = started + Duration::from_secs(5);
        let paused_at = Some(started + Duration::from_secs(3));
        let elapsed = elapsed_excluding_pauses(now, started, Duration::ZERO, paused_at);
        assert_eq!(elapsed, Duration::from_secs(3));
    }

    #[test]
    fn cancel_clears_recording_and_pending_prompt() {
        let mut service = SpeechService::new();
        service.is_recording = true;
        service.started_at = Some(Instant::now());
        service.set_pending_prompt("prompt-1".into(), "Prompt One".into(), PendingOrigin::ShiftClick);

        assert!(service.cancel_recording().is_ok());
        assert!(!service.is_recording());
        assert!(!service.is_transcribing());
        assert_eq!(service.take_pending_prompt().0, None);
    }

    #[test]
    fn pause_and_resume_without_recording_return_error() {
        let mut service = SpeechService::new();
        assert!(service.pause().is_err());
        assert!(service.resume().is_err());
    }

    #[test]
    fn pause_and_resume_toggle_paused_state() {
        let mut service = SpeechService::new();
        service.is_recording = true;
        service.started_at = Some(Instant::now());

        service.pause().unwrap();
        assert!(service.is_paused());
        assert!(service.is_recording());
        service.pause().unwrap();
        assert!(service.is_paused());

        service.resume().unwrap();
        assert!(!service.is_paused());
        service.resume().unwrap();
        assert!(!service.is_paused());
    }

    #[test]
    fn not_debouncing_on_new_service() {
        let service = SpeechService::new();
        assert!(!service.is_debouncing());
    }

    #[test]
    fn debouncing_right_after_toggle() {
        let mut service = SpeechService::new();
        service.mark_toggle();
        assert!(service.is_debouncing());
    }

    #[test]
    fn retry_guard_is_per_entry() {
        let mut service = SpeechService::new();
        assert!(service.begin_retry("entry-1"));
        assert!(!service.begin_retry("entry-1"));
        assert!(service.begin_retry("entry-2"));
        assert!(service.is_retrying("entry-1"));

        service.end_retry("entry-1");
        assert!(!service.is_retrying("entry-1"));
        assert!(service.is_retrying("entry-2"));
    }

    #[test]
    fn retry_guard_does_not_touch_transcribing_flag() {
        let mut service = SpeechService::new();
        service.begin_retry("entry-1");
        assert!(!service.is_transcribing());
    }

    #[test]
    fn pending_prompt_lifecycle() {
        let mut service = SpeechService::new();
        assert_eq!(service.take_pending_prompt().0, None);

        service.set_pending_prompt("prompt-1".into(), "Prompt One".into(), PendingOrigin::ShiftClick);
        let pending = service.take_pending_prompt().0.unwrap();
        assert_eq!(pending.id, "prompt-1");
        assert_eq!(pending.name, "Prompt One");
        assert_eq!(pending.origin, PendingOrigin::ShiftClick);
        assert_eq!(service.take_pending_prompt().0, None);
    }

    #[test]
    fn selecting_a_skill_sets_the_slot_with_pill_origin() {
        let mut service = SpeechService::new();
        service.select_target("fix", "Fix").unwrap();
        assert_eq!(
            service.pending_prompt(),
            Some(&PendingPrompt {
                id: "fix".into(),
                name: "Fix".into(),
                origin: PendingOrigin::Pill
            })
        );
    }

    #[test]
    fn selecting_plain_dictation_clears_the_slot() {
        let mut service = SpeechService::new();
        service.select_target("fix", "Fix").unwrap();
        service.select_target(DICTATION_ID, DICTATION_LABEL).unwrap();
        assert_eq!(service.pending_prompt(), None);
    }

    #[test]
    fn selection_is_rejected_after_the_take() {
        let mut service = SpeechService::new();
        service.select_target("fix", "Fix").unwrap();
        service.take_pending_prompt();
        assert!(service.target_locked());
        assert_eq!(service.select_target("other", "Other"), Err(TargetLocked));
        assert_eq!(service.pending_prompt(), None);
    }

    #[test]
    fn reset_without_shift_click_drops_the_previous_pending_skill() {
        let mut service = SpeechService::new();
        service.select_target("fix", "Fix").unwrap();
        service.take_pending_prompt();
        service.set_pending_prompt("stale".into(), "Stale".into(), PendingOrigin::Pill);
        service.reset_target_for_start(None);
        assert_eq!(service.pending_prompt(), None);
        assert!(!service.target_locked());
    }

    #[test]
    fn reset_with_the_matching_shift_click_keeps_it() {
        let mut service = SpeechService::new();
        service.set_pending_prompt("fix".into(), "Fix".into(), PendingOrigin::ShiftClick);
        service.reset_target_for_start(Some("fix"));
        assert_eq!(service.pending_prompt().map(|p| p.id.as_str()), Some("fix"));
    }

    #[test]
    fn reset_drops_a_pill_selection_and_a_shift_click_for_another_skill() {
        let mut service = SpeechService::new();
        service.select_target("fix", "Fix").unwrap();
        service.reset_target_for_start(Some("fix"));
        assert_eq!(service.pending_prompt(), None);

        service.set_pending_prompt("fix".into(), "Fix".into(), PendingOrigin::ShiftClick);
        service.reset_target_for_start(Some("other"));
        assert_eq!(service.pending_prompt(), None);
    }

    #[test]
    fn the_stop_chat_flag_is_reset_at_a_start_and_returned_by_the_take() {
        let mut service = SpeechService::new();
        service.request_stop_chat().unwrap();
        service.reset_target_for_start(None);
        assert!(!service.take_pending_prompt().1);

        service.reset_target_for_start(None);
        service.request_stop_chat().unwrap();
        assert!(service.take_pending_prompt().1);
    }

    #[test]
    fn stop_chat_is_rejected_after_the_take() {
        let mut service = SpeechService::new();
        service.take_pending_prompt();
        assert_eq!(service.request_stop_chat(), Err(TargetLocked));
        assert!(!service.take_pending_prompt().1);
    }
}
