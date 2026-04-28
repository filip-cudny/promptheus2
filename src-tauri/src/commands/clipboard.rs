use tauri::State;
use tokio::sync::Mutex;

use super::settings::AppState;

#[tauri::command(async)]
pub fn get_clipboard_text() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let text = clipboard.get_text().map_err(|e| e.to_string())?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err("clipboard is empty".into());
    }
    Ok(trimmed)
}

#[tauri::command]
pub fn set_clipboard_text(
    state: State<'_, Mutex<AppState>>,
    content: String,
) -> Result<(), String> {
    let state = state.blocking_lock();
    state.clipboard.set_text(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clipboard_is_empty(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.blocking_lock();
    Ok(state.clipboard.is_empty())
}

#[tauri::command]
pub fn clipboard_has_image(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.blocking_lock();
    Ok(state.clipboard.has_image())
}

#[tauri::command(async)]
pub fn get_clipboard_image(
    state: State<'_, Mutex<AppState>>,
) -> Result<(String, String), String> {
    let state = state.blocking_lock();
    state
        .clipboard
        .get_image_base64()
        .map_err(|e| e.to_string())
}
