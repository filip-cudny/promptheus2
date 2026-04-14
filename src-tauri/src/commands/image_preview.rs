use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::services::dock::DockManager;
use crate::services::monitor::find_monitor_at;

struct PendingImage {
    data: String,
    media_type: String,
}

struct StoredWorkArea {
    cursor_x: f64,
    cursor_y: f64,
    work_x: f64,
    work_y: f64,
    work_width: f64,
    work_height: f64,
}

static PENDING: Mutex<Option<PendingImage>> = Mutex::new(None);
static WORK_AREA: Mutex<Option<StoredWorkArea>> = Mutex::new(None);

#[derive(Serialize)]
pub struct ImagePayload {
    data: String,
    media_type: String,
}

#[derive(Serialize)]
pub struct ImagePreviewWorkArea {
    cursor_x: f64,
    cursor_y: f64,
    work_x: f64,
    work_y: f64,
    work_width: f64,
    work_height: f64,
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

    let cursor_pos = win.cursor_position().map_err(|e| e.to_string())?;
    let monitor = find_monitor_at(&app, cursor_pos.x as i32, cursor_pos.y as i32)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingImage { data, media_type });
    *WORK_AREA.lock().unwrap_or_else(|e| e.into_inner()) = Some(StoredWorkArea {
        cursor_x: cursor_pos.x / scale,
        cursor_y: cursor_pos.y / scale,
        work_x: work.position.x as f64 / scale,
        work_y: work.position.y as f64 / scale,
        work_width: work.size.width as f64 / scale,
        work_height: work.size.height as f64 / scale,
    });

    let already_visible = win.is_visible().unwrap_or(false);
    if !already_visible {
        let dock = app.state::<DockManager>();
        dock.dialog_opened(&app);
    }

    #[cfg(target_os = "macos")]
    app.show().map_err(|e| e.to_string())?;

    app.emit_to("image-preview", "load-image", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_image_preview_work_area() -> Result<ImagePreviewWorkArea, String> {
    let wa = WORK_AREA
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or("no work area stored")?;

    Ok(ImagePreviewWorkArea {
        cursor_x: wa.cursor_x,
        cursor_y: wa.cursor_y,
        work_x: wa.work_x,
        work_y: wa.work_y,
        work_width: wa.work_width,
        work_height: wa.work_height,
    })
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
