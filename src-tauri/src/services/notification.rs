use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::models::settings::NotificationEvents;

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
    pub level: NotificationLevel,
    pub title: String,
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Failed to emit notification event: {0}")]
    EmitFailed(String),
}

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
    ) -> Result<(), NotificationError> {
        if !matches!(level, NotificationLevel::Error)
            && !is_event_enabled(event_name, &settings.events)
        {
            log::debug!("notification event '{event_name}' is disabled, skipping");
            return Ok(());
        }

        let payload = NotificationPayload {
            level,
            title: title.into(),
            message: message.map(|m| m.into()),
        };

        log::debug!(
            "emitting notification: event={event_name} level={:?} title={}",
            payload.level,
            payload.title
        );

        self.handle
            .emit("notification", &payload)
            .map_err(|e| {
                log::error!("failed to emit notification event: {e}");
                NotificationError::EmitFailed(e.to_string())
            })
    }
}

pub fn is_event_enabled(event_name: &str, events: &NotificationEvents) -> bool {
    match event_name {
        "prompt_execution_success" => events.prompt_execution_success,
        "prompt_execution_cancel" => events.prompt_execution_cancel,
        "prompt_execution_in_progress" => events.prompt_execution_in_progress,
        "speech_recording_start" => events.speech_recording_start,
        "speech_recording_stop" => events.speech_recording_stop,
        "speech_transcription_success" => events.speech_transcription_success,
        "context_saved" => events.context_saved,
        "context_set" => events.context_set,
        "context_append" => events.context_append,
        "context_cleared" => events.context_cleared,
        "clipboard_copy" => events.clipboard_copy,
        "image_added" => events.image_added,
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
        assert!(is_event_enabled("clipboard_copy", &events));

        events.prompt_execution_success = false;
        assert!(!is_event_enabled("prompt_execution_success", &events));

        events.clipboard_copy = false;
        assert!(!is_event_enabled("clipboard_copy", &events));
    }

    #[test]
    fn test_is_event_enabled_all_twelve_events() {
        let mut events = NotificationEvents::default();
        let event_names = [
            "prompt_execution_success",
            "prompt_execution_cancel",
            "prompt_execution_in_progress",
            "speech_recording_start",
            "speech_recording_stop",
            "speech_transcription_success",
            "context_saved",
            "context_set",
            "context_append",
            "context_cleared",
            "clipboard_copy",
            "image_added",
        ];

        for name in &event_names {
            assert!(is_event_enabled(name, &events), "{name} should be enabled by default");
        }

        events.prompt_execution_success = false;
        events.prompt_execution_cancel = false;
        events.prompt_execution_in_progress = false;
        events.speech_recording_start = false;
        events.speech_recording_stop = false;
        events.speech_transcription_success = false;
        events.context_saved = false;
        events.context_set = false;
        events.context_append = false;
        events.context_cleared = false;
        events.clipboard_copy = false;
        events.image_added = false;

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
            level: NotificationLevel::Success,
            title: "Done".to_string(),
            message: Some("Task completed".to_string()),
        };
        let json: serde_json::Value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["level"], "success");
        assert_eq!(json["title"], "Done");
        assert_eq!(json["message"], "Task completed");

        let payload_no_msg = NotificationPayload {
            level: NotificationLevel::Error,
            title: "Failed".to_string(),
            message: None,
        };
        let json: serde_json::Value = serde_json::to_value(&payload_no_msg).unwrap();
        assert_eq!(json["level"], "error");
        assert_eq!(json["title"], "Failed");
        assert!(json["message"].is_null());
    }
}
