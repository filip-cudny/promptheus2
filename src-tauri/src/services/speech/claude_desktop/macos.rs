use std::path::PathBuf;

use tauri::AppHandle;

use super::Outcome;

// Placeholder replaced by the macOS paste step; until then every macOS delivery ends in the clipboard fallback.
pub fn claude_running() -> bool {
    false
}

pub async fn paste_into_claude(_app: &AppHandle, _files: &[PathBuf], _warm: bool) -> Outcome {
    Outcome::Failed("macOS paste not built".into())
}
