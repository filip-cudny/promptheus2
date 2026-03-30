use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

struct PendingText {
    text: String,
    index: usize,
    source_window: String,
}

static PENDING: Mutex<Option<PendingText>> = Mutex::new(None);

#[derive(Serialize)]
pub struct TextPayload {
    text: String,
    index: usize,
    source_window: String,
}

#[tauri::command]
pub async fn open_text_preview(
    app: tauri::AppHandle,
    text: String,
    index: usize,
    source_window: String,
) -> Result<(), String> {
    let win = app
        .get_webview_window("text-preview")
        .ok_or("text-preview window not found")?;

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingText {
        text,
        index,
        source_window,
    });

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

    app.emit_to("text-preview", "load-text", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_pending_text() -> Option<TextPayload> {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .map(|p| TextPayload {
            text: p.text,
            index: p.index,
            source_window: p.source_window,
        })
}
