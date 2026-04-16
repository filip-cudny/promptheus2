use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::services::dialog;
use crate::services::dock::DockManager;
use crate::services::monitor::find_monitor_at;

const GEOMETRY_KEY: &str = "text-preview";

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
    let label = "text-preview";

    let win = app
        .get_webview_window(label)
        .ok_or("text-preview window not found")?;

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingText {
        text,
        index,
        source_window,
    });

    dialog::restore_size(&app, label, GEOMETRY_KEY).await;

    if let Ok(pos) = win.cursor_position() {
        let cx = pos.x as i32;
        let cy = pos.y as i32;

        let (x, y) = if let Ok(monitor) = find_monitor_at(&app, cx, cy) {
            let work = monitor.work_area();
            let win_size = win.outer_size().unwrap_or(tauri::PhysicalSize {
                width: 500,
                height: 400,
            });

            let right_edge = work.position.x + work.size.width as i32;
            let bottom_edge = work.position.y + work.size.height as i32;

            let mut x = cx;
            let mut y = cy;
            if x + win_size.width as i32 > right_edge { x = right_edge - win_size.width as i32; }
            if y + win_size.height as i32 > bottom_edge { y = bottom_edge - win_size.height as i32; }
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
    dialog::focus_window(&win)?;

    app.emit_to(label, "load-text", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn save_text_preview_geometry(app: tauri::AppHandle) {
    dialog::save_geometry(&app, "text-preview", GEOMETRY_KEY);
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
