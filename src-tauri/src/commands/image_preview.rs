use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::services::dock::DockManager;
use crate::services::monitor::find_monitor_at;

const MAX_SIZE: f64 = 400.0;

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
        let cx = pos.x as i32;
        let cy = pos.y as i32;

        let (x, y) = if let Ok(monitor) = find_monitor_at(&app, cx, cy) {
            let work = monitor.work_area();
            let scale = monitor.scale_factor();
            let win_size = (MAX_SIZE * scale) as i32;

            let right_edge = work.position.x + work.size.width as i32;
            let bottom_edge = work.position.y + work.size.height as i32;

            let mut x = cx;
            let mut y = cy;
            if x + win_size > right_edge { x = right_edge - win_size; }
            if y + win_size > bottom_edge { y = bottom_edge - win_size; }
            if x < work.position.x { x = work.position.x; }
            if y < work.position.y { y = work.position.y; }
            (x, y)
        } else {
            (cx, cy)
        };

        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }

    let already_visible = win.is_visible().unwrap_or(false);
    if !already_visible {
        let dock = app.state::<DockManager>();
        dock.dialog_opened(&app);
    }

    #[cfg(target_os = "macos")]
    app.show().map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    app.emit_to("image-preview", "load-image", ())
        .map_err(|e| e.to_string())?;

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
