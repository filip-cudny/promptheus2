use serde::Deserialize;

use crate::models::settings::{ModelConfig, Provider};

use super::SpeechError;

const MIN_UPLOAD_SECS: u64 = 30;
const MAX_UPLOAD_SECS: u64 = 300;
const ASSUMED_UPLOAD_BYTES_PER_SEC: u64 = 50_000;

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

/// A three-minute recording is several megabytes; a fixed 60s ceiling turns a
/// slow uplink into a self-inflicted timeout, so the budget scales with payload.
fn upload_timeout(payload_bytes: usize) -> std::time::Duration {
    let secs = MIN_UPLOAD_SECS + payload_bytes as u64 / ASSUMED_UPLOAD_BYTES_PER_SEC;
    std::time::Duration::from_secs(secs.min(MAX_UPLOAD_SECS))
}

fn build_http_client(payload_bytes: usize) -> Result<reqwest::Client, SpeechError> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(upload_timeout(payload_bytes))
        .build()
        .map_err(|e| SpeechError::permanent(e.to_string()))
}

fn map_send_error(e: reqwest::Error) -> SpeechError {
    if e.is_connect() {
        SpeechError::transient("Connection failed — check your internet")
    } else if e.is_timeout() {
        SpeechError::transient("Request timed out")
    } else if e.is_request() {
        SpeechError::transient(e.to_string())
    } else {
        SpeechError::permanent(e.to_string())
    }
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    match raw.trim().parse::<u64>() {
        Ok(secs) => Some(secs),
        Err(_) => {
            log::debug!("ignoring non-numeric Retry-After header: {raw}");
            None
        }
    }
}

async fn map_http_error(status: reqwest::StatusCode, response: reqwest::Response) -> SpeechError {
    let retry_after = parse_retry_after(response.headers());
    let body = response.text().await.unwrap_or_default();

    match status.as_u16() {
        401 | 403 => SpeechError::permanent("API key is invalid or expired"),
        429 => SpeechError::transient_after(
            "Rate limit exceeded — retrying shortly",
            retry_after,
        ),
        408 => SpeechError::transient_after("Request timed out on the server", retry_after),
        500..=599 => SpeechError::transient_after(
            format!("Provider is unavailable (status {status})"),
            retry_after,
        ),
        _ => SpeechError::permanent(format!("API error (status {status}): {body}")),
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

    let payload_bytes = wav_bytes.len();
    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| SpeechError::permanent(e.to_string()))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model.clone());

    if let Some(ref lang) = options.language {
        form = form.text("language", lang.clone());
    }

    if let Some(ref prompt) = options.prompt {
        form = form.text("prompt", prompt.clone());
    }

    let response = build_http_client(payload_bytes)?
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
        .map_err(|e| SpeechError::transient(format!("Failed to parse response: {e}")))?;

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

    let payload_bytes = wav_bytes.len();
    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| SpeechError::permanent(e.to_string()))?;

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

    let response = build_http_client(payload_bytes)?
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
        .map_err(|e| SpeechError::transient(format!("Failed to parse response: {e}")))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{ModelConfig, ModelType};

    fn stt_test_config(api_key: Option<&str>) -> ModelConfig {
        ModelConfig {
            id: "stt-test".into(),
            model: "whisper-1".into(),
            display_name: "Whisper".into(),
            model_type: ModelType::Stt,
            provider: None,
            group: None,
            api_key: api_key.map(|k| k.to_string()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            capabilities: None,
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

    #[tokio::test]
    async fn transcribe_elevenlabs_missing_api_key_returns_error() {
        let mut config = stt_test_config(None);
        config.provider = Some(Provider::ElevenLabs);
        let result = transcribe(vec![0; 44], &config, &SttOptions::default()).await;
        assert!(matches!(result, Err(SpeechError::ApiKeyMissing)));
    }
}
