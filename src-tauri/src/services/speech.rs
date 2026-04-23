use std::io::{Cursor, Seek, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Device;
use serde::Deserialize;

use crate::models::settings::{ModelConfig, Provider};

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

#[derive(Debug, Clone, Default)]
pub struct SttOptions {
    pub language: Option<String>,
    pub no_verbatim: Option<bool>,
    pub prompt: Option<String>,
    pub keyterms: Vec<String>,
}

pub async fn transcribe(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<String, SpeechError> {
    match config.provider {
        Some(Provider::ElevenLabs) => transcribe_elevenlabs(wav_bytes, config, options).await,
        _ => transcribe_openai(wav_bytes, config, options).await,
    }
}

fn build_http_client() -> Result<reqwest::Client, SpeechError> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| SpeechError::TranscriptionFailed(e.to_string()))
}

fn map_send_error(e: reqwest::Error) -> SpeechError {
    if e.is_connect() {
        SpeechError::TranscriptionFailed("Connection failed — check your internet".into())
    } else {
        SpeechError::TranscriptionFailed(e.to_string())
    }
}

async fn map_http_error(status: reqwest::StatusCode, response: reqwest::Response) -> SpeechError {
    let body = response.text().await.unwrap_or_default();
    match status.as_u16() {
        401 => SpeechError::TranscriptionFailed("API key is invalid or expired".into()),
        429 => SpeechError::TranscriptionFailed(
            "Rate limit exceeded — please wait and try again".into(),
        ),
        _ => SpeechError::TranscriptionFailed(format!("API error (status {status}): {body}")),
    }
}

async fn transcribe_openai(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<String, SpeechError> {
    let api_key = config
        .resolved_api_key()
        .ok_or(SpeechError::ApiKeyMissing)?;

    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1")
        .trim_end_matches('/');

    let url = format!("{base_url}/audio/transcriptions");

    log::debug!(
        "STT transcribe provider=openai model={} bytes={} has_prompt={}",
        config.model,
        wav_bytes.len(),
        options.prompt.is_some()
    );

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| SpeechError::TranscriptionFailed(e.to_string()))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model.clone());

    if let Some(ref lang) = options.language {
        form = form.text("language", lang.clone());
    }

    if let Some(ref prompt) = options.prompt {
        form = form.text("prompt", prompt.clone());
    }

    let response = build_http_client()?
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(map_send_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(map_http_error(status, response).await);
    }

    let parsed: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| SpeechError::TranscriptionFailed(format!("Failed to parse response: {e}")))?;

    let text = parsed.text.trim().to_string();
    if text.is_empty() {
        return Err(SpeechError::NoSpeechDetected);
    }

    Ok(text)
}

async fn transcribe_elevenlabs(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<String, SpeechError> {
    let api_key = config
        .resolved_api_key()
        .ok_or(SpeechError::ApiKeyMissing)?;

    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.elevenlabs.io/v1")
        .trim_end_matches('/');

    let url = format!("{base_url}/speech-to-text");

    log::debug!(
        "STT transcribe provider=elevenlabs model={} bytes={} keyterms={}",
        config.model,
        wav_bytes.len(),
        options.keyterms.len()
    );

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| SpeechError::TranscriptionFailed(e.to_string()))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model_id", config.model.clone());

    if let Some(ref lang) = options.language {
        form = form.text("language_code", lang.clone());
    }

    if let Some(flag) = options.no_verbatim {
        form = form.text("no_verbatim", flag.to_string());
    }

    form = form.text("tag_audio_events", "false");

    for term in options.keyterms.iter() {
        form = form.text("keyterms", term.clone());
    }

    let response = build_http_client()?
        .post(&url)
        .header("xi-api-key", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(map_send_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(map_http_error(status, response).await);
    }

    let parsed: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| SpeechError::TranscriptionFailed(format!("Failed to parse response: {e}")))?;

    let text = parsed.text.trim().to_string();
    if text.is_empty() {
        return Err(SpeechError::NoSpeechDetected);
    }

    Ok(text)
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

fn find_input_device() -> Result<Device, SpeechError> {
    let host = cpal::default_host();

    if let Some(device) = host.default_input_device() {
        return Ok(device);
    }

    let devices = host
        .input_devices()
        .map_err(|_| SpeechError::NoInputDevice)?;

    for device in devices {
        if let Ok(mut configs) = device.supported_input_configs() {
            if configs.any(|c| c.channels() >= 1) {
                return Ok(device);
            }
        }
    }

    Err(SpeechError::NoInputDevice)
}

fn negotiate_sample_rate(device: &Device) -> Result<u32, SpeechError> {
    let preferred_rates: [u32; 4] = [16000, 44100, 48000, 8000];

    let supported_configs: Vec<_> = device
        .supported_input_configs()
        .map_err(|_| SpeechError::NoSupportedConfig)?
        .collect();

    if supported_configs.is_empty() {
        return Err(SpeechError::NoSupportedConfig);
    }

    for rate in preferred_rates {
        let supported = supported_configs
            .iter()
            .any(|c| c.min_sample_rate() <= rate && rate <= c.max_sample_rate());
        if supported {
            return Ok(rate);
        }
    }

    Ok(supported_configs[0].max_sample_rate())
}

pub fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, SpeechError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let buffer = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let writer = SharedWriter(Arc::clone(&buffer));

    let mut wav_writer =
        hound::WavWriter::new(writer, spec).map_err(|e| SpeechError::WavEncode(e.to_string()))?;

    for &sample in samples {
        wav_writer
            .write_sample(sample)
            .map_err(|e| SpeechError::WavEncode(e.to_string()))?;
    }

    wav_writer
        .finalize()
        .map_err(|e| SpeechError::WavEncode(e.to_string()))?;

    let cursor = buffer.lock().unwrap();
    Ok(cursor.get_ref().clone())
}

struct SharedWriter(Arc<Mutex<Cursor<Vec<u8>>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

impl Seek for SharedWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.lock().unwrap().seek(pos)
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
    fn encode_wav_produces_valid_header() {
        let samples: Vec<i16> = vec![0, 100, -100, 200, -200];
        let wav_bytes = encode_wav(&samples, 16000).unwrap();

        assert!(wav_bytes.len() > 44);
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");

        let reader = hound::WavReader::new(Cursor::new(&wav_bytes)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(reader.len(), 5);
    }

    #[test]
    fn encode_wav_empty_samples() {
        let wav_bytes = encode_wav(&[], 44100).unwrap();
        assert_eq!(&wav_bytes[0..4], b"RIFF");

        let reader = hound::WavReader::new(Cursor::new(&wav_bytes)).unwrap();
        assert_eq!(reader.len(), 0);
        assert_eq!(reader.spec().sample_rate, 44100);
    }

    fn stt_test_config(api_key: Option<&str>) -> ModelConfig {
        ModelConfig {
            id: "stt-test".into(),
            model: "whisper-1".into(),
            display_name: "Whisper".into(),
            model_type: crate::models::settings::ModelType::Stt,
            provider: None,
            group: None,
            api_key: api_key.map(|k| k.to_string()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            store: true,
        }
    }

    #[tokio::test]
    async fn transcribe_missing_api_key_returns_error() {
        let config = stt_test_config(None);
        let result = transcribe(vec![0; 44], &config, &SttOptions::default()).await;
        assert!(matches!(result, Err(SpeechError::ApiKeyMissing)));
    }

    #[tokio::test]
    async fn transcribe_empty_api_key_returns_error() {
        let config = stt_test_config(Some(""));
        let result = transcribe(vec![0; 44], &config, &SttOptions::default()).await;
        assert!(matches!(result, Err(SpeechError::ApiKeyMissing)));
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
