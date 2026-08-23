use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Manager, WebviewWindow};

use crate::services::monitor::find_monitor_at;
use crate::services::notification::NotificationPayload;
use crate::Error;

static PENDING: Mutex<Vec<NotificationPayload>> = Mutex::new(Vec::new());
static SHOW_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct AnchorPosition {
    work_right: i32,
    work_bottom: i32,
    scale: f64,
}

static ANCHOR: Mutex<Option<AnchorPosition>> = Mutex::new(None);

/// Last time the notification webview proved it is alive by draining the queue
/// or resizing its window. Without this, a webview that dies while
/// `SHOW_IN_FLIGHT` is set would swallow every later notification: the show
/// path is skipped and the `drainPending()` eval lands nowhere.
static LAST_WEBVIEW_ACK: Mutex<Option<Instant>> = Mutex::new(None);

const WEBVIEW_ACK_TIMEOUT: Duration = Duration::from_secs(10);

fn mark_webview_alive() {
    *LAST_WEBVIEW_ACK.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
}

fn ack_expired(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|t| now.duration_since(t) > WEBVIEW_ACK_TIMEOUT)
}

fn webview_unresponsive() -> bool {
    let last = *LAST_WEBVIEW_ACK.lock().unwrap_or_else(|e| e.into_inner());
    ack_expired(last, Instant::now())
}

pub fn show_notification(handle: &tauri::AppHandle, payload: NotificationPayload) {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(payload);

    let claimed = SHOW_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_ok();

    if !claimed && webview_unresponsive() {
        log::warn!(
            "notification webview silent for over {}s, retaking the show path",
            WEBVIEW_ACK_TIMEOUT.as_secs(),
        );
    } else if !claimed {
        if let Some(win) = handle.get_webview_window("notification") {
            if let Err(e) = win.eval("drainPending()") {
                log::error!("notification drainPending eval failed: {e}");
            }
        }
        return;
    }

    mark_webview_alive();
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = show_notification_window(&handle) {
            log::error!("show_notification failed: {e}");
            SHOW_IN_FLIGHT.store(false, Ordering::Release);
        }
    });
}

fn compute_anchor(
    handle: &tauri::AppHandle,
    win: &WebviewWindow,
) -> crate::Result<(i32, i32, f64)> {
    let cursor_pos = win.cursor_position()?;
    let monitor = find_monitor_at(handle, cursor_pos.x as i32, cursor_pos.y as i32)
        .map_err(Error::Other)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    let work_right = work.position.x + work.size.width as i32;
    let work_bottom = work.position.y + work.size.height as i32;

    *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = Some(AnchorPosition {
        work_right,
        work_bottom,
        scale,
    });

    Ok((work_right, work_bottom, scale))
}

fn show_notification_window(handle: &tauri::AppHandle) -> crate::Result<()> {
    let win = handle
        .get_webview_window("notification")
        .ok_or_else(|| Error::Other("notification window not found".into()))?;

    let (work_right, work_bottom, scale) = compute_anchor(handle, &win)?;

    let win_width = (380.0 * scale) as i32;
    let win_height = (140.0 * scale) as i32;

    let x = work_right - win_width;
    let y = work_bottom - win_height;

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))?;

    win.show()?;

    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::WidgetExt;
        if let Ok(gtk_win) = win.gtk_window() {
            gtk_win.set_opacity(0.8);
        }
    }

    win.eval("drainPending()")?;

    Ok(())
}

#[tauri::command]
pub fn drain_pending_notifications() -> Vec<NotificationPayload> {
    mark_webview_alive();
    let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    pending.drain(..).collect()
}

#[tauri::command]
pub async fn update_notification_window(
    app: tauri::AppHandle,
    count: u32,
    height: u32,
) -> crate::Result<()> {
    let win = app
        .get_webview_window("notification")
        .ok_or_else(|| Error::Other("notification window not found".into()))?;

    mark_webview_alive();

    if count == 0 {
        win.hide()?;
        *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = None;
        SHOW_IN_FLIGHT.store(false, Ordering::Release);
        return Ok(());
    }

    let anchor = ANCHOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|a| (a.work_right, a.work_bottom, a.scale));

    let (work_right, work_bottom, scale) = match anchor {
        Some(cached) => cached,
        None => {
            log::warn!("notification anchor dropped while toasts are live, re-showing window");
            let recomputed = compute_anchor(&app, &win)?;
            SHOW_IN_FLIGHT.store(true, Ordering::Release);
            win.show()?;
            recomputed
        }
    };

    let new_height = height.max(60);
    let win_width = (380.0 * scale) as i32;
    let win_height = (new_height as f64 * scale) as i32;

    let x = work_right - win_width;
    let y = work_bottom - win_height;

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))?;
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 380.0,
        height: new_height as f64,
    }))?;

    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::WidgetExt;
        if let Ok(gtk_win) = win.gtk_window() {
            gtk_win.queue_draw();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_expires_only_after_the_timeout() {
        let now = Instant::now();
        let within = now.checked_sub(WEBVIEW_ACK_TIMEOUT / 2).expect("instant in range");
        let beyond = now
            .checked_sub(WEBVIEW_ACK_TIMEOUT + Duration::from_secs(1))
            .expect("instant in range");

        assert!(!ack_expired(Some(within), now));
        assert!(ack_expired(Some(beyond), now));
    }

    #[test]
    fn missing_ack_counts_as_expired() {
        assert!(ack_expired(None, Instant::now()));
    }
}
