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

#[tauri::command]
pub fn set_clipboard_text(
    clipboard: State<'_, ClipboardService>,
    content: String,
) -> crate::Result<()> {
    clipboard.set_text(&content)?;
    Ok(())
}

#[tauri::command]
pub fn clipboard_is_empty(clipboard: State<'_, ClipboardService>) -> crate::Result<bool> {
    Ok(clipboard.is_empty())
}

#[tauri::command]
pub fn clipboard_has_image(clipboard: State<'_, ClipboardService>) -> crate::Result<bool> {
    Ok(clipboard.has_image())
}

#[tauri::command(async)]
pub fn get_clipboard_image(
    clipboard: State<'_, ClipboardService>,
) -> crate::Result<(String, String)> {
    Ok(clipboard.get_image_base64()?)
}
