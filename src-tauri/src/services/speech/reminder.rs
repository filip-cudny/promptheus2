use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::Manager;
use tokio::sync::Mutex;

use crate::models::settings::RecordingReminderSettings;
use crate::services::config::ConfigService;
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::speech::SpeechService;

const TICK: Duration = Duration::from_secs(1);
const SILENCE_RMS_THRESHOLD: f64 = 400.0;

/// Watches an active recording and warns the user that it is still running —
/// either because the microphone went quiet for a while (the user most likely
/// stopped dictating) or because the hard interval ceiling was reached.
pub fn spawn(
    app: tauri::AppHandle,
    session: u64,
    config: RecordingReminderSettings,
    shortcut_hint: Option<String>,
) {
    if !config.enabled {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let Some(speech) = app
            .try_state::<Arc<Mutex<SpeechService>>>()
            .map(|s| s.inner().clone())
        else {
            return;
        };
        let buffer = {
            let s = speech.lock().await;
            if !s.is_recording() || s.session() != session {
                return;
            }
            s.audio_buffer()
        };

        let silence_window = Duration::from_secs(config.silence_window_secs.max(1));
        let silence_after = Duration::from_secs(config.silence_after_secs);
        let max_interval = Duration::from_secs(config.max_interval_secs.max(1));

        let mut cursor: usize = 0;
        let mut silence_since: Option<Instant> = None;
        let mut last_reminder_at = Instant::now();

        loop {
            tokio::time::sleep(TICK).await;

            let elapsed = match recording_elapsed(&speech, session).await {
                Some(elapsed) => elapsed,
                None => return,
            };

            let rms = {
                let buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
                if buf.len() < cursor {
                    cursor = buf.len();
                }
                let tail = &buf[cursor..];
                cursor = buf.len();
                rms(tail)
            };

            let now = Instant::now();
            if rms < SILENCE_RMS_THRESHOLD {
                silence_since.get_or_insert(now);
            } else {
                silence_since = None;
            }

            let silent_for = silence_since.map_or(Duration::ZERO, |t| now.duration_since(t));
            let since_last_reminder = now.duration_since(last_reminder_at);

            let hard_due = since_last_reminder >= max_interval;
            let silence_due = elapsed >= silence_after
                && silent_for >= silence_window
                && since_last_reminder >= silence_after;

            if !hard_due && !silence_due {
                continue;
            }

            let Some(config_state) = app.try_state::<Arc<Mutex<ConfigService>>>() else {
                return;
            };
            let notification_settings =
                config_state.lock().await.settings().notifications.clone();

            if !notification_settings.recording_reminder.enabled {
                return;
            }

            if recording_elapsed(&speech, session).await.is_none() {
                return;
            }

            let message = match &shortcut_hint {
                Some(hint) => format!("{} elapsed — press {} to stop", format_elapsed(elapsed), hint),
                None => format!("{} elapsed", format_elapsed(elapsed)),
            };

            log::debug!(
                target: "app_lib::speech_reminder",
                "recording reminder: session={session} elapsed={elapsed:?} silence_due={silence_due}",
            );

            let Some(notifications) = app.try_state::<NotificationService>() else {
                return;
            };
            let _ = notifications.notify(
                "speech_recording_reminder",
                NotificationLevel::Warning,
                "Still recording",
                Some(message),
                &notification_settings,
            );

            silence_since = None;
            last_reminder_at = Instant::now();
        }
    });
}

async fn recording_elapsed(speech: &Arc<Mutex<SpeechService>>, session: u64) -> Option<Duration> {
    let s = speech.lock().await;
    if !s.is_recording() || s.session() != session {
        return None;
    }
    s.started_at().map(|t| t.elapsed())
}

fn rms(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (sum / samples.len() as f64).sqrt()
}

fn format_elapsed(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_empty_slice_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn rms_of_silence_is_below_threshold() {
        assert!(rms(&[0, 1, -2, 3]) < SILENCE_RMS_THRESHOLD);
    }

    #[test]
    fn rms_of_speech_level_audio_is_above_threshold() {
        let loud: Vec<i16> = (0..100).map(|i| if i % 2 == 0 { 8000 } else { -8000 }).collect();
        assert!(rms(&loud) > SILENCE_RMS_THRESHOLD);
    }

    #[test]
    fn format_elapsed_pads_seconds() {
        assert_eq!(format_elapsed(Duration::from_secs(9)), "0:09");
        assert_eq!(format_elapsed(Duration::from_secs(200)), "3:20");
    }
}
