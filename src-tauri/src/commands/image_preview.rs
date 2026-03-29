use std::sync::Mutex;

use serde::Serialize;
use tauri::Manager;

struct PendingImage {
    data: String,
    media_type: String,
}

static PENDING: Mutex<Option<PendingImage>> = Mutex::new(None);

#[derive(Serialize)]
pub struct ImagePayload {
    data: String,
    media_type: String,
}

#[tauri::command]
pub async fn open_image_preview(
    app: tauri::AppHandle,
    data: String,
    media_type: String,
) -> Result<(), String> {
    let win = app
        .get_webview_window("image-preview")
        .ok_or("image-preview window not found")?;

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(PendingImage { data, media_type });

    if let Ok(pos) = win.cursor_position() {
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: pos.x as i32,
            y: pos.y as i32,
        }));
    }

    #[cfg(target_os = "macos")]
    app.show().map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_pending_image() -> Option<ImagePayload> {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .map(|p| ImagePayload {
            data: p.data,
            media_type: p.media_type,
        })
}
