use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpeechTranscriptionError {
    pub message: String,
    pub recoverable: bool,
    pub entry_id: Option<String>,
    pub has_audio: bool,
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeechTranscriptionRetry {
    pub attempt: u32,
    pub max_attempts: u32,
    pub next_in_secs: u64,
    pub reason: String,
    pub entry_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeechRecordingStartedEvent {
    pub action_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeechRecordingStoppedEvent {
    pub had_audio: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioClipInfo {
    pub has_audio: bool,
    pub expires_at: Option<String>,
    pub duration_secs: Option<f64>,
    pub bytes: Option<u64>,
    pub is_retrying: bool,
}

impl AudioClipInfo {
    pub fn absent() -> Self {
        Self {
            has_audio: false,
            expires_at: None,
            duration_secs: None,
            bytes: None,
            is_retrying: false,
        }
    }
}
