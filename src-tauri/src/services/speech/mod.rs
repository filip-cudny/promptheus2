mod recorder;
mod transcriber;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, StreamTrait};

use recorder::{find_input_device, negotiate_sample_rate};

pub use recorder::encode_wav;
pub use transcriber::{transcribe, SttOptions};

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
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("API key not configured")]
    ApiKeyMissing,
    #[error("No speech detected")]
    NoSpeechDetected,
    #[error("Recording failed: {0}")]
    RecordingFailed(String),
}

pub struct SpeechService {
    is_recording: bool,
    is_transcribing: bool,
    recording_action_id: Option<String>,
    pending_skill_id: Option<String>,
    pending_skill_name: Option<String>,
    audio_buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    stop_sender: Option<mpsc::Sender<()>>,
    last_transcription_finished: Option<Instant>,
}

impl SpeechService {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            is_transcribing: false,
            recording_action_id: None,
            pending_skill_id: None,
            pending_skill_name: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            stop_sender: None,
            last_transcription_finished: None,
        }
    }

    pub fn start_recording(&mut self, action_id: Option<String>) -> Result<u32, SpeechError> {
        if self.is_recording {
            return Err(SpeechError::AlreadyRecording);
        }

        let device = find_input_device()?;
        let sample_rate = negotiate_sample_rate(&device)?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        let (stop_tx, stop_rx) = mpsc::channel();

        std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.extend_from_slice(data);
                    }
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
        self.sample_rate = sample_rate;
        self.is_recording = true;
        self.recording_action_id = action_id;
        self.stop_sender = Some(stop_tx);

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

        Ok((samples, sample_rate))
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
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

    pub fn set_pending_prompt(&mut self, id: Option<String>, name: Option<String>) {
        self.pending_skill_id = id;
        self.pending_skill_name = name;
    }

    pub fn take_pending_prompt(&mut self) -> (Option<String>, Option<String>) {
        (self.pending_skill_id.take(), self.pending_skill_name.take())
    }

    pub fn mark_transcription_finished(&mut self) {
        self.last_transcription_finished = Some(Instant::now());
    }

    pub fn is_on_cooldown(&self) -> bool {
        self.last_transcription_finished
            .map(|t| t.elapsed().as_secs_f64() < 2.0)
            .unwrap_or(false)
    }

    pub fn clear_cooldown(&mut self) {
        self.last_transcription_finished = None;
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
    fn cooldown_inactive_on_new_service() {
        let service = SpeechService::new();
        assert!(!service.is_on_cooldown());
    }

    #[test]
    fn cooldown_active_after_transcription_finished() {
        let mut service = SpeechService::new();
        service.mark_transcription_finished();
        assert!(service.is_on_cooldown());
    }

    #[test]
    fn cooldown_cleared_explicitly() {
        let mut service = SpeechService::new();
        service.mark_transcription_finished();
        service.clear_cooldown();
        assert!(!service.is_on_cooldown());
    }

    #[test]
    fn pending_prompt_lifecycle() {
        let mut service = SpeechService::new();
        assert_eq!(service.take_pending_prompt(), (None, None));

        service.set_pending_prompt(Some("prompt-1".to_string()), Some("Prompt One".to_string()));
        let (id, name) = service.take_pending_prompt();
        assert_eq!(id, Some("prompt-1".to_string()));
        assert_eq!(name, Some("Prompt One".to_string()));
        assert_eq!(service.take_pending_prompt(), (None, None));
    }
}
