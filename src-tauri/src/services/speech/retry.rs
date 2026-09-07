use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::settings::ModelConfig;

use super::{transcribe, SpeechError, SttOptions};

const BASE_BACKOFF_SECS: u64 = 2;
const BACKOFF_FACTOR: u64 = 3;
const MAX_BACKOFF_SECS: u64 = 60;
const JITTER_PERCENT: u64 = 20;

#[derive(Debug, Clone, Copy)]
pub struct RetryNotice {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_secs: u64,
}

/// Runs one transcription, re-sending the same audio while the failure looks
/// transient. Shared by the post-recording path and manual retries so both
/// honour identical backoff and error classification.
pub async fn transcribe_with_retry<F>(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
    max_attempts: u32,
    mut on_retry: F,
) -> Result<String, SpeechError>
where
    F: FnMut(RetryNotice, &SpeechError),
{
    let total = max_attempts.max(1);
    let mut attempt = 1u32;

    loop {
        match transcribe(wav_bytes.clone(), config, options).await {
            Ok(text) => return Ok(text),
            Err(error) => {
                if attempt >= total || !error.is_transient() {
                    if attempt > 1 {
                        log::warn!("STT gave up after {attempt} attempts: {error}");
                    }
                    return Err(error);
                }

                let delay_secs = next_delay_secs(attempt, error.retry_after_secs());
                log::warn!(
                    "STT attempt {attempt}/{total} failed ({error}), retrying in {delay_secs}s"
                );
                on_retry(
                    RetryNotice {
                        attempt,
                        max_attempts: total,
                        delay_secs,
                    },
                    &error,
                );

                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                attempt += 1;
            }
        }
    }
}

fn next_delay_secs(attempt: u32, retry_after_secs: Option<u64>) -> u64 {
    if let Some(secs) = retry_after_secs {
        return secs.clamp(1, MAX_BACKOFF_SECS);
    }

    let exponential = BASE_BACKOFF_SECS
        .saturating_mul(BACKOFF_FACTOR.saturating_pow(attempt.saturating_sub(1)))
        .min(MAX_BACKOFF_SECS);

    apply_jitter(exponential).clamp(1, MAX_BACKOFF_SECS)
}

fn apply_jitter(base_secs: u64) -> u64 {
    let spread = base_secs * JITTER_PERCENT / 100;
    if spread == 0 {
        return base_secs;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let offset = nanos % (spread * 2 + 1);
    (base_secs + offset).saturating_sub(spread).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::ModelType;

    fn config() -> ModelConfig {
        ModelConfig {
            id: "stt-test".into(),
            model: "whisper-1".into(),
            display_name: "Whisper".into(),
            model_type: ModelType::Stt,
            provider: None,
            group: None,
            api_key: None,
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            capabilities: None,
            store: true,
        }
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        assert!((1..=3).contains(&next_delay_secs(1, None)));
        assert!((4..=8).contains(&next_delay_secs(2, None)));
        assert!(next_delay_secs(9, None) <= MAX_BACKOFF_SECS);
    }

    #[test]
    fn retry_after_header_wins_over_backoff() {
        assert_eq!(next_delay_secs(1, Some(42)), 42);
    }

    #[test]
    fn retry_after_is_clamped_to_ceiling() {
        assert_eq!(next_delay_secs(1, Some(9999)), MAX_BACKOFF_SECS);
        assert_eq!(next_delay_secs(1, Some(0)), 1);
    }

    #[tokio::test]
    async fn permanent_failure_is_not_retried() {
        let mut notices = 0;
        let result = transcribe_with_retry(
            vec![0; 44],
            &config(),
            &SttOptions::default(),
            5,
            |_, _| notices += 1,
        )
        .await;

        assert!(matches!(result, Err(SpeechError::ApiKeyMissing)));
        assert_eq!(notices, 0);
    }
}
