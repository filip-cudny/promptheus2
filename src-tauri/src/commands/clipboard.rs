use tauri::State;
use tokio::sync::Mutex;

use super::settings::AppState;
use crate::Error;

#[tauri::command(async)]
pub fn get_clipboard_text() -> crate::Result<String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| Error::Other(e.to_string()))?;
    let text = clipboard
        .get_text()
        .map_err(|e| Error::Other(e.to_string()))?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err(Error::Other("clipboard is empty".into()));
    }
    Ok(trimmed)
}

#[tauri::command]
pub fn set_clipboard_text(
    state: State<'_, Mutex<AppState>>,
    content: String,
) -> crate::Result<()> {
    let state = state.blocking_lock();
    state.clipboard.set_text(&content)?;
    Ok(())
}

#[tauri::command]
pub fn clipboard_is_empty(state: State<'_, Mutex<AppState>>) -> crate::Result<bool> {
    let state = state.blocking_lock();
    Ok(state.clipboard.is_empty())
}

#[tauri::command]
pub fn clipboard_has_image(state: State<'_, Mutex<AppState>>) -> crate::Result<bool> {
    let state = state.blocking_lock();
    Ok(state.clipboard.has_image())
}

#[tauri::command(async)]
pub fn get_clipboard_image(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<(String, String)> {
    let state = state.blocking_lock();
    Ok(state.clipboard.get_image_base64()?)
}
