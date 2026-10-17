use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::services::dock::DockManager;
use crate::services::dialog::window;
use crate::services::monitor::{cursor_work_area, CursorWorkArea};
use crate::Error;

pub const IMAGE_PREVIEW_TITLE: &str = "Promptheus Image Preview";

static PENDING: Mutex<Option<ImagePayload>> = Mutex::new(None);
static WORK_AREA: Mutex<Option<CursorWorkArea>> = Mutex::new(None);

#[derive(Serialize)]
pub struct ImagePayload {
    data: String,
    media_type: String,
}

#[derive(Serialize, Clone)]
struct LoadImageEvent {
    media_type: String,
}

#[tauri::command]
pub async fn open_image_preview(
    app: tauri::AppHandle,
    data: String,
    media_type: String,
) -> crate::Result<()> {
    let win = window(&app, "image-preview")?;
    let work_area = cursor_work_area(&win)?;

    let media_type_event = media_type.clone();
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(ImagePayload { data, media_type });
    *WORK_AREA.lock().unwrap_or_else(|e| e.into_inner()) = Some(work_area);

    let already_visible = win.is_visible().unwrap_or(false);
    if !already_visible {
        let dock = app.state::<DockManager>();
        dock.dialog_opened(&app);
    }

    #[cfg(target_os = "macos")]
    app.show()?;

    app.emit_to(
        "image-preview",
        "load-image",
        LoadImageEvent {
            media_type: media_type_event,
        },
    )?;

    Ok(())
}

#[tauri::command]
pub fn get_image_preview_work_area() -> crate::Result<CursorWorkArea> {
    WORK_AREA
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or_else(|| Error::Other("no work area stored".into()))
}

#[tauri::command]
pub fn get_pending_image() -> Option<ImagePayload> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take()
}
