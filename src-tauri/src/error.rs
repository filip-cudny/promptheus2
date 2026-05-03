use crate::services::ai::AiError;
use crate::services::clipboard::ClipboardError;
use crate::services::config::ConfigError;
use crate::services::database::DatabaseError;
use crate::services::execution::ExecutionError;
use crate::services::image_storage::ImageStorageError;
use crate::services::mcp::McpError;
use crate::services::notification::NotificationError;
use crate::services::placeholder::PlaceholderError;
use crate::services::skill::SkillError;
use crate::services::speech::SpeechError;
use crate::services::sqlite_history::HistoryError;
use crate::services::ui_state::UiStateError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    Ai(#[from] AiError),

    #[error(transparent)]
    Mcp(#[from] McpError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Database(#[from] DatabaseError),

    #[error(transparent)]
    UiState(#[from] UiStateError),

    #[error(transparent)]
    Skill(#[from] SkillError),

    #[error(transparent)]
    ImageStorage(#[from] ImageStorageError),

    #[error(transparent)]
    History(#[from] HistoryError),

    #[error(transparent)]
    Clipboard(#[from] ClipboardError),

    #[error(transparent)]
    Speech(#[from] SpeechError),

    #[error(transparent)]
    Notification(#[from] NotificationError),

    #[error(transparent)]
    Placeholder(#[from] PlaceholderError),

    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Other(value)
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::Other(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
