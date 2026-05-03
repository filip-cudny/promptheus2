use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpeechTranscriptionError {
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeechRecordingStartedEvent {
    pub action_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeechRecordingStoppedEvent {
    pub had_audio: bool,
}
