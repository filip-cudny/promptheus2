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

struct TranscriptionRequest {
    url: String,
    auth_header: (&'static str, String),
    fields: Vec<(&'static str, String)>,
}

fn openai_request(
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<TranscriptionRequest, SpeechError> {
    let api_key = config
        .resolved_api_key()
        .ok_or(SpeechError::ApiKeyMissing)?;

    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1")
        .trim_end_matches('/');

    let mut fields = vec![("model", config.model.clone())];

    if let Some(ref lang) = options.language {
        fields.push(("language", lang.clone()));
    }

    if let Some(ref prompt) = options.prompt {
        fields.push(("prompt", prompt.clone()));
    }

    Ok(TranscriptionRequest {
        url: format!("{base_url}/audio/transcriptions"),
        auth_header: ("Authorization", format!("Bearer {api_key}")),
        fields,
    })
}

fn elevenlabs_request(
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<TranscriptionRequest, SpeechError> {
    let api_key = config
        .resolved_api_key()
        .ok_or(SpeechError::ApiKeyMissing)?;

    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.elevenlabs.io/v1")
        .trim_end_matches('/');

    let mut fields = vec![("model_id", config.model.clone())];

    if let Some(ref lang) = options.language {
        fields.push(("language_code", lang.clone()));
    }

    if let Some(flag) = options.no_verbatim {
        fields.push(("no_verbatim", flag.to_string()));
    }

    fields.push(("tag_audio_events", "false".to_string()));

    for term in options.keyterms.iter() {
        fields.push(("keyterms", term.clone()));
    }

    Ok(TranscriptionRequest {
        url: format!("{base_url}/speech-to-text"),
        auth_header: ("xi-api-key", api_key),
        fields,
    })
}

async fn transcribe_openai(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<String, SpeechError> {
    let request = openai_request(config, options)?;

    log::debug!(
        "STT transcribe provider=openai model={} bytes={} has_prompt={}",
        config.model,
        wav_bytes.len(),
        options.prompt.is_some()
    );

    send_transcription(wav_bytes, request).await
}

async fn transcribe_elevenlabs(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
) -> Result<String, SpeechError> {
    let request = elevenlabs_request(config, options)?;

    log::debug!(
        "STT transcribe provider=elevenlabs model={} bytes={} keyterms={}",
        config.model,
        wav_bytes.len(),
        options.keyterms.len()
    );

    send_transcription(wav_bytes, request).await
}

async fn send_transcription(
    wav_bytes: Vec<u8>,
    request: TranscriptionRequest,
) -> Result<String, SpeechError> {
    let payload_bytes = wav_bytes.len();
    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| SpeechError::permanent(e.to_string()))?;

    let form = request.fields.into_iter().fold(
        reqwest::multipart::Form::new().part("file", file_part),
        |form, (name, value)| form.text(name, value),
    );

    let (auth_name, auth_value) = request.auth_header;
    let response = build_http_client(payload_bytes)?
        .post(&request.url)
        .header(auth_name, auth_value)
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

    transcript_text(parsed)
}

fn transcript_text(parsed: TranscriptionResponse) -> Result<String, SpeechError> {
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

    #[test]
    fn openai_request_uses_default_url_bearer_auth_and_model() {
        let config = stt_test_config(Some("sk-test"));
        let request = openai_request(&config, &SttOptions::default()).unwrap();
        assert_eq!(request.url, "https://api.openai.com/v1/audio/transcriptions");
        assert_eq!(request.auth_header, ("Authorization", "Bearer sk-test".to_string()));
        assert_eq!(request.fields, vec![("model", "whisper-1".to_string())]);
    }

    #[test]
    fn openai_request_trims_base_url_and_adds_language_then_prompt() {
        let mut config = stt_test_config(Some("sk-test"));
        config.base_url = Some("http://localhost:8080/v1/".into());
        let options = SttOptions {
            language: Some("pl".into()),
            prompt: Some("hello".into()),
            ..SttOptions::default()
        };
        let request = openai_request(&config, &options).unwrap();
        assert_eq!(request.url, "http://localhost:8080/v1/audio/transcriptions");
        assert_eq!(
            request.fields,
            vec![
                ("model", "whisper-1".to_string()),
                ("language", "pl".to_string()),
                ("prompt", "hello".to_string()),
            ]
        );
    }

    #[test]
    fn elevenlabs_request_uses_xi_api_key_and_ordered_fields() {
        let mut config = stt_test_config(Some("xi-test"));
        config.provider = Some(Provider::ElevenLabs);
        let options = SttOptions {
            language: Some("en".into()),
            no_verbatim: Some(true),
            prompt: Some("ignored".into()),
            keyterms: vec!["Promptheus".into(), "Tauri".into()],
        };
        let request = elevenlabs_request(&config, &options).unwrap();
        assert_eq!(request.url, "https://api.elevenlabs.io/v1/speech-to-text");
        assert_eq!(request.auth_header, ("xi-api-key", "xi-test".to_string()));
        assert_eq!(
            request.fields,
            vec![
                ("model_id", "whisper-1".to_string()),
                ("language_code", "en".to_string()),
                ("no_verbatim", "true".to_string()),
                ("tag_audio_events", "false".to_string()),
                ("keyterms", "Promptheus".to_string()),
                ("keyterms", "Tauri".to_string()),
            ]
        );
    }

    #[test]
    fn transcript_text_trims_and_rejects_blank_as_no_speech() {
        let text = transcript_text(TranscriptionResponse { text: "  hi there \n".into() });
        assert_eq!(text.unwrap(), "hi there");
        let blank = transcript_text(TranscriptionResponse { text: " \n ".into() });
        assert!(matches!(blank, Err(SpeechError::NoSpeechDetected)));
    }
}
