use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::Manager;

use crate::services::monitor::find_monitor_at;
use crate::services::notification::NotificationPayload;

static PENDING: Mutex<Vec<NotificationPayload>> = Mutex::new(Vec::new());
static SHOW_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct AnchorPosition {
    work_right: i32,
    work_bottom: i32,
    scale: f64,
}

static ANCHOR: Mutex<Option<AnchorPosition>> = Mutex::new(None);

pub fn show_notification(handle: &tauri::AppHandle, payload: NotificationPayload) {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(payload);

    if SHOW_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = show_notification_window(&handle) {
                log::error!("show_notification failed: {e}");
                SHOW_IN_FLIGHT.store(false, Ordering::Release);
            }
        });
    } else if let Some(win) = handle.get_webview_window("notification") {
        if let Err(e) = win.eval("drainPending()") {
            log::error!("notification drainPending eval failed: {e}");
        }
    }
}

fn show_notification_window(handle: &tauri::AppHandle) -> Result<(), String> {
    let win = handle
        .get_webview_window("notification")
        .ok_or("notification window not found")?;

    let cursor_pos = win.cursor_position().map_err(|e| e.to_string())?;
    let monitor = find_monitor_at(handle, cursor_pos.x as i32, cursor_pos.y as i32)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    let work_right = work.position.x + work.size.width as i32;
    let work_bottom = work.position.y + work.size.height as i32;

    *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = Some(AnchorPosition {
        work_right,
        work_bottom,
        scale,
    });

    let win_width = (380.0 * scale) as i32;
    let win_height = (140.0 * scale) as i32;

    let x = work_right - win_width;
    let y = work_bottom - win_height;

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
        *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = None;
        SHOW_IN_FLIGHT.store(false, Ordering::Release);
        return Ok(());
    }

    let anchor = ANCHOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|a| (a.work_right, a.work_bottom, a.scale));

    let (work_right, work_bottom, scale) = anchor.ok_or("no anchor position cached")?;

    let new_height = height.max(60);
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 380.0,
        height: new_height as f64,
    }))
    .map_err(|e| e.to_string())?;

    let win_width = (380.0 * scale) as i32;
    let win_height = (new_height as f64 * scale) as i32;

    let x = work_right - win_width;
    let y = work_bottom - win_height;

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

