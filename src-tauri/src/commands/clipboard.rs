use tauri::State;

use crate::services::clipboard::ClipboardService;
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

#[tauri::command(async)]
pub fn get_clipboard_image(
    clipboard: State<'_, ClipboardService>,
) -> crate::Result<(String, String)> {
    Ok(clipboard.get_image_base64()?)
}
