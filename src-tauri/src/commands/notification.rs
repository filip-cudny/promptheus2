use std::sync::Mutex;

use tauri::Manager;

use crate::services::monitor::find_monitor_at;
use crate::services::notification::NotificationPayload;

static PENDING: Mutex<Vec<NotificationPayload>> = Mutex::new(Vec::new());

pub fn show_notification(handle: &tauri::AppHandle, payload: NotificationPayload) {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(payload);

    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = show_notification_window(&handle) {
            log::error!("show_notification failed: {e}");
        }
    });
}

fn show_notification_window(handle: &tauri::AppHandle) -> Result<(), String> {
    let win = handle
        .get_webview_window("notification")
        .ok_or("notification window not found")?;

    let cursor_pos = win.cursor_position().map_err(|e| e.to_string())?;
    let monitor = find_monitor_at(handle, cursor_pos.x as i32, cursor_pos.y as i32)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    let margin = (20.0 * scale) as i32;
    let win_width = (380.0 * scale) as i32;
    let win_height = (100.0 * scale) as i32;

    let x = work.position.x + work.size.width as i32 - win_width - margin;
    let y = work.position.y + work.size.height as i32 - win_height - margin;

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;

    win.eval("drainPending()").map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::WidgetExt;
        if let Ok(gtk_win) = win.gtk_window() {
            gtk_win.set_opacity(0.8);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn drain_pending_notifications() -> Vec<NotificationPayload> {
    let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    pending.drain(..).collect()
}

#[tauri::command]
pub async fn update_notification_window(
    app: tauri::AppHandle,
    count: u32,
    height: u32,
) -> Result<(), String> {
    let win = app
        .get_webview_window("notification")
        .ok_or("notification window not found")?;

    if count == 0 {
        win.hide().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let new_height = height.max(60);
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 380.0,
        height: new_height as f64,
    }))
    .map_err(|e| e.to_string())?;

    let cursor_pos = win.cursor_position().map_err(|e| e.to_string())?;
    let monitor = find_monitor_at(&app, cursor_pos.x as i32, cursor_pos.y as i32)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    let margin = (20.0 * scale) as i32;
    let win_width = (380.0 * scale) as i32;
    let win_height = (new_height as f64 * scale) as i32;

    let x = work.position.x + work.size.width as i32 - win_width - margin;
    let y = work.position.y + work.size.height as i32 - win_height - margin;

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

