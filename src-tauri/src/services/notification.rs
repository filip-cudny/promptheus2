use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::models::settings::NotificationEvents;
use crate::services::config::ConfigService;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationPayload {
    pub id: String,
    pub level: NotificationLevel,
    pub title: String,
    pub message: Option<String>,
    pub monochromatic: bool,
}

#[derive(Clone)]
pub struct NotificationService {
    handle: AppHandle,
}

impl NotificationService {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    pub fn notify(
        &self,
        event_name: &str,
        level: NotificationLevel,
        title: impl Into<String>,
        message: Option<impl Into<String>>,
        settings: &crate::models::settings::NotificationSettings,
    ) {
        if !matches!(level, NotificationLevel::Error)
            && !is_event_enabled(event_name, &settings.events)
        {
            log::debug!("notification event '{event_name}' is disabled, skipping");
            return;
        }

        let id = format!("n-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let payload = NotificationPayload {
            id,
            level,
            title: title.into(),
            message: message.map(|m| m.into()),
            monochromatic: settings.monochromatic_notification_icons,
        };

        log::debug!(
            "notification: event={event_name} level={:?} title={}",
            payload.level,
            payload.title
        );

        crate::services::notification_window::show_notification(&self.handle, payload);
    }

    pub async fn notify_now(
        app: &AppHandle,
        event_name: &str,
        level: NotificationLevel,
        title: impl Into<String>,
        message: Option<impl Into<String>>,
    ) {
        let settings = app
            .state::<Arc<Mutex<ConfigService>>>()
            .lock()
            .await
            .settings()
            .notifications
            .clone();
        app.state::<NotificationService>()
            .notify(event_name, level, title, message, &settings);
    }
}

pub fn is_event_enabled(event_name: &str, events: &NotificationEvents) -> bool {
    match event_name {
        "prompt_execution_success" => events.prompt_execution_success,
        "prompt_execution_cancel" => events.prompt_execution_cancel,
        "speech_recording_start" => events.speech_recording_start,
        "speech_recording_stop" => events.speech_recording_stop,
        "speech_transcription_success" => events.speech_transcription_success,
        "speech_transcription_retry" => events.speech_transcription_retry,
        "context_set" => events.context_set,
        "context_append" => events.context_append,
        "context_cleared" => events.context_cleared,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::NotificationEvents;

    #[test]
    fn test_is_event_enabled_known_events() {
        let mut events = NotificationEvents::default();
        assert!(is_event_enabled("prompt_execution_success", &events));
        assert!(is_event_enabled("context_set", &events));

        events.prompt_execution_success = false;
        assert!(!is_event_enabled("prompt_execution_success", &events));

        events.context_set = false;
        assert!(!is_event_enabled("context_set", &events));
    }

    #[test]
    fn test_is_event_enabled_all_nine_events() {
        let mut events = NotificationEvents::default();
        let event_names = [
            "prompt_execution_success",
            "prompt_execution_cancel",
            "speech_recording_start",
            "speech_recording_stop",
            "speech_transcription_success",
            "context_set",
            "context_append",
            "context_cleared",
        ];

        for name in &event_names {
            assert!(is_event_enabled(name, &events), "{name} should be enabled by default");
        }

        events.prompt_execution_success = false;
        events.prompt_execution_cancel = false;
        events.speech_recording_start = false;
        events.speech_recording_stop = false;
        events.speech_transcription_success = false;
        events.context_set = false;
        events.context_append = false;
        events.context_cleared = false;

        for name in &event_names {
            assert!(!is_event_enabled(name, &events), "{name} should be disabled");
        }
    }

    #[test]
    fn test_is_event_enabled_unknown_events() {
        let events = NotificationEvents::default();
        assert!(is_event_enabled("unknown_event", &events));
        assert!(is_event_enabled("", &events));
        assert!(is_event_enabled("some_future_event", &events));
    }

    #[test]
    fn test_notification_level_serializes_lowercase() {
        let json = serde_json::to_string(&NotificationLevel::Success).unwrap();
        assert_eq!(json, r#""success""#);

        let json = serde_json::to_string(&NotificationLevel::Error).unwrap();
        assert_eq!(json, r#""error""#);

        let json = serde_json::to_string(&NotificationLevel::Info).unwrap();
        assert_eq!(json, r#""info""#);

        let json = serde_json::to_string(&NotificationLevel::Warning).unwrap();
        assert_eq!(json, r#""warning""#);
    }

    #[test]
    fn test_notification_payload_serialization() {
        let payload = NotificationPayload {
            id: "n-1".to_string(),
            level: NotificationLevel::Success,
            title: "Done".to_string(),
            message: Some("Task completed".to_string()),
            monochromatic: true,
        };
        let json: serde_json::Value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["id"], "n-1");
        assert_eq!(json["level"], "success");
        assert_eq!(json["title"], "Done");
        assert_eq!(json["message"], "Task completed");
        assert_eq!(json["monochromatic"], true);

        let payload_no_msg = NotificationPayload {
            id: "n-2".to_string(),
            level: NotificationLevel::Error,
            title: "Failed".to_string(),
            message: None,
            monochromatic: false,
        };
        let json: serde_json::Value = serde_json::to_value(&payload_no_msg).unwrap();
        assert_eq!(json["id"], "n-2");
        assert_eq!(json["level"], "error");
        assert_eq!(json["title"], "Failed");
        assert!(json["message"].is_null());
        assert_eq!(json["monochromatic"], false);
    }
}
