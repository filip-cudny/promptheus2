use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::watch;

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
/// honour identical backoff and error classification. `on_retry` is awaited
/// after each retryable failure, before the backoff starts; a `true` on
/// `abort` ends the run with `SpeechError::Cancelled`.
pub async fn transcribe_with_retry<R, RF>(
    wav_bytes: Vec<u8>,
    config: &ModelConfig,
    options: &SttOptions,
    max_attempts: u32,
    abort: Option<watch::Receiver<bool>>,
    on_retry: R,
) -> Result<String, SpeechError>
where
    R: FnMut(RetryNotice, &SpeechError) -> RF,
    RF: Future<Output = ()>,
{
    retry_loop(
        || transcribe(wav_bytes.clone(), config, options),
        max_attempts,
        abort,
        on_retry,
    )
    .await
}

async fn retry_loop<A, AF, R, RF>(
    mut attempt_fn: A,
    max_attempts: u32,
    mut abort: Option<watch::Receiver<bool>>,
    mut on_retry: R,
) -> Result<String, SpeechError>
where
    A: FnMut() -> AF,
    AF: Future<Output = Result<String, SpeechError>>,
    R: FnMut(RetryNotice, &SpeechError) -> RF,
    RF: Future<Output = ()>,
{
    let total = max_attempts.max(1);
    let mut attempt = 1u32;

    loop {
        if abort.as_ref().is_some_and(|rx| *rx.borrow()) {
            return Err(cancelled(attempt));
        }

        let outcome = tokio::select! {
            biased;
            _ = abort_requested(&mut abort) => return Err(cancelled(attempt)),
            outcome = attempt_fn() => outcome,
        };

        match outcome {
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
                )
                .await;

                tokio::select! {
                    biased;
                    _ = abort_requested(&mut abort) => return Err(cancelled(attempt)),
                    _ = tokio::time::sleep(Duration::from_secs(delay_secs)) => {}
                }
                attempt += 1;
            }
        }
    }
}

/// Resolves only when `true` is seen; no signal, or a dropped sender, never resolves.
async fn abort_requested(abort: &mut Option<watch::Receiver<bool>>) {
    let Some(rx) = abort else {
        return std::future::pending().await;
    };
    loop {
        if *rx.borrow_and_update() {
            return;
        }
        if rx.changed().await.is_err() {
            return std::future::pending().await;
        }
    }
}

fn cancelled(attempt: u32) -> SpeechError {
    log::info!("STT cancelled at attempt {attempt}");
    SpeechError::Cancelled
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
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

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
            None,
            |_, _| {
                notices += 1;
                async {}
            },
        )
        .await;

        assert!(matches!(result, Err(SpeechError::ApiKeyMissing)));
        assert_eq!(notices, 0);
    }

    fn transient_failure() -> SpeechError {
        SpeechError::transient_after("boom", Some(1))
    }

    #[tokio::test]
    async fn retry_callback_completes_before_next_attempt() {
        let flag = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicU32::new(0));

        let attempt_flag = Arc::clone(&flag);
        let attempt_calls = Arc::clone(&calls);
        let result = retry_loop(
            move || {
                let n = attempt_calls.fetch_add(1, Ordering::SeqCst) + 1;
                let flag = Arc::clone(&attempt_flag);
                async move {
                    if n == 1 {
                        Err(transient_failure())
                    } else {
                        assert!(flag.load(Ordering::SeqCst));
                        Ok("text".to_string())
                    }
                }
            },
            3,
            None,
            move |_, _| {
                let flag = Arc::clone(&flag);
                async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    flag.store(true, Ordering::SeqCst);
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), "text");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn abort_during_backoff_cancels_without_next_attempt() {
        let (tx, rx) = watch::channel(false);
        let calls = Arc::new(AtomicU32::new(0));

        let attempt_calls = Arc::clone(&calls);
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            retry_loop(
                move || {
                    attempt_calls.fetch_add(1, Ordering::SeqCst);
                    async { Err(SpeechError::transient_after("boom", Some(60))) }
                },
                3,
                Some(rx),
                move |_, _| {
                    tx.send(true).unwrap();
                    async {}
                },
            ),
        )
        .await
        .expect("abort must end the backoff");

        assert!(matches!(result, Err(SpeechError::Cancelled)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn abort_during_pending_attempt_cancels_without_waiting() {
        let (tx, rx) = watch::channel(false);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            tx.send(true).unwrap();
        });

        let result = tokio::time::timeout(
            Duration::from_millis(500),
            retry_loop(
                || std::future::pending::<Result<String, SpeechError>>(),
                3,
                Some(rx),
                |_, _| async {},
            ),
        )
        .await
        .expect("abort must end the attempt");

        assert!(matches!(result, Err(SpeechError::Cancelled)));
    }

    #[tokio::test]
    async fn dropped_sender_neither_aborts_nor_restarts_attempt() {
        let (tx, rx) = watch::channel(false);
        let calls = Arc::new(AtomicU32::new(0));
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            drop(tx);
        });

        let attempt_calls = Arc::clone(&calls);
        let result = retry_loop(
            move || {
                attempt_calls.fetch_add(1, Ordering::SeqCst);
                async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok("text".to_string())
                }
            },
            3,
            Some(rx),
            |_, _| async {},
        )
        .await;

        assert_eq!(result.unwrap(), "text");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn no_receiver_runs_unchanged() {
        let calls = Arc::new(AtomicU32::new(0));
        let mut notices = Vec::new();

        let attempt_calls = Arc::clone(&calls);
        let result = retry_loop(
            move || {
                let n = attempt_calls.fetch_add(1, Ordering::SeqCst) + 1;
                async move {
                    if n == 1 {
                        Err(transient_failure())
                    } else {
                        Ok("text".to_string())
                    }
                }
            },
            3,
            None,
            |notice, _| {
                notices.push(notice);
                async {}
            },
        )
        .await;

        assert_eq!(result.unwrap(), "text");
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].attempt, 1);
        assert_eq!(notices[0].max_attempts, 3);
        assert_eq!(notices[0].delay_secs, 1);
    }

    #[tokio::test]
    async fn abort_sent_before_start_runs_no_attempt() {
        let (tx, rx) = watch::channel(false);
        tx.send(true).unwrap();
        let calls = Arc::new(AtomicU32::new(0));

        let attempt_calls = Arc::clone(&calls);
        let result = retry_loop(
            move || {
                attempt_calls.fetch_add(1, Ordering::SeqCst);
                async { Ok("text".to_string()) }
            },
            3,
            Some(rx),
            |_, _| async {},
        )
        .await;

        assert!(matches!(result, Err(SpeechError::Cancelled)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
