use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Manager, WebviewWindow};

use crate::services::monitor::find_monitor_at;
use crate::services::notification::NotificationPayload;
use crate::Error;

static PENDING: Mutex<Vec<NotificationPayload>> = Mutex::new(Vec::new());
static SHOW_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

pub const NOTIFICATION_TITLE: &str = "Promptheus Notifications";

const WINDOW_WIDTH: f64 = 380.0;
const FIRST_SHOW_HEIGHT: f64 = 140.0;
const MIN_HEIGHT: u32 = 60;

#[derive(Clone, Copy, Debug, PartialEq)]
struct AnchorPosition {
    work_right: i32,
    work_bottom: i32,
    scale: f64,
    through_shell: bool,
}

impl AnchorPosition {
    fn from_work_area(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scale: f64,
        through_shell: bool,
    ) -> Self {
        Self {
            work_right: x + width,
            work_bottom: y + height,
            scale,
            through_shell,
        }
    }

    fn origin(&self, logical_height: f64) -> (i32, i32) {
        let width = (WINDOW_WIDTH * self.scale) as i32;
        let height = (logical_height * self.scale) as i32;
        (self.work_right - width, self.work_bottom - height)
    }
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
        if let Err(e) = show_notification_window(&handle).await {
            log::error!("show_notification failed: {e}");
            SHOW_IN_FLIGHT.store(false, Ordering::Release);
        }
    });
}

fn resize(win: &WebviewWindow, logical_height: f64) -> crate::Result<()> {
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: WINDOW_WIDTH,
        height: logical_height,
    }))?;
    Ok(())
}

fn remember_anchor(anchor: AnchorPosition) {
    *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = Some(anchor);
}

fn compute_anchor(
    handle: &tauri::AppHandle,
    win: &WebviewWindow,
) -> crate::Result<AnchorPosition> {
    let cursor_pos = win.cursor_position()?;
    let monitor = find_monitor_at(handle, cursor_pos.x as i32, cursor_pos.y as i32)
        .map_err(Error::Other)?;
    let work = monitor.work_area();
    Ok(AnchorPosition::from_work_area(
        work.position.x,
        work.position.y,
        work.size.width as i32,
        work.size.height as i32,
        monitor.scale_factor(),
        false,
    ))
}

#[cfg(target_os = "linux")]
async fn show_through_shell(win: &WebviewWindow) -> crate::Result<()> {
    let pointer = crate::services::gnome_shell::proxy()
        .await
        .map_err(|e| Error::Other(e.to_string()))?
        .get_pointer()
        .await
        .map_err(|e| Error::Other(e.to_string()))?;
    let (_, _, work_x, work_y, work_width, work_height) = pointer;
    let anchor =
        AnchorPosition::from_work_area(work_x, work_y, work_width, work_height, 1.0, true);
    remember_anchor(anchor);
    crate::services::gnome_shell::place_window_anchored(
        NOTIFICATION_TITLE,
        anchor.work_right,
        anchor.work_bottom,
        false,
        Some(win),
    )
    .await?;
    Ok(())
}

async fn anchor_and_show(
    handle: &tauri::AppHandle,
    win: &WebviewWindow,
    logical_height: f64,
) -> crate::Result<()> {
    resize(win, logical_height)?;

    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        match show_through_shell(win).await {
            Ok(()) => return Ok(()),
            Err(e) => log::warn!(
                "placing the notification through the GNOME Shell extension failed, \
                 leaving placement to the compositor: {e}"
            ),
        }
    }

    let anchor = compute_anchor(handle, win)?;
    remember_anchor(anchor);
    let (x, y) = anchor.origin(logical_height);
    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))?;
    win.show()?;
    Ok(())
}

fn resize_and_place(
    win: &WebviewWindow,
    anchor: AnchorPosition,
    logical_height: f64,
) -> crate::Result<()> {
    if !anchor.through_shell {
        let (x, y) = anchor.origin(logical_height);
        win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))?;
    }
    resize(win, logical_height)
}

async fn show_notification_window(handle: &tauri::AppHandle) -> crate::Result<()> {
    let win = handle
        .get_webview_window("notification")
        .ok_or_else(|| Error::Other("notification window not found".into()))?;

    anchor_and_show(handle, &win, FIRST_SHOW_HEIGHT).await?;

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

    let new_height = height.max(MIN_HEIGHT) as f64;
    let cached = *ANCHOR.lock().unwrap_or_else(|e| e.into_inner());

    let anchor = match cached {
        Some(anchor) => anchor,
        None => {
            log::warn!("notification anchor dropped while toasts are live, re-showing window");
            SHOW_IN_FLIGHT.store(true, Ordering::Release);
            anchor_and_show(&app, &win, new_height).await?;
            let shown = *ANCHOR.lock().unwrap_or_else(|e| e.into_inner());
            shown.ok_or_else(|| Error::Other("notification anchor missing after show".into()))?
        }
    };

    if anchor.through_shell {
        log::debug!(
            "notification window update: count={count}, logical height={new_height}, \
             anchored by the extension at bottom-right ({}, {})",
            anchor.work_right,
            anchor.work_bottom,
        );
    } else {
        log::debug!(
            "notification window update: count={count}, logical height={new_height}, \
             origin={:?}",
            anchor.origin(new_height),
        );
    }
    resize_and_place(&win, anchor, new_height)?;

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
    fn work_area_gives_the_bottom_right_anchor() {
        let anchor = AnchorPosition::from_work_area(1920, 32, 2560, 1408, 1.0, true);
        assert_eq!((anchor.work_right, anchor.work_bottom), (1920 + 2560, 32 + 1408));
    }

    #[test]
    fn origin_anchors_the_bottom_right_corner_in_logical_pixels() {
        let anchor = AnchorPosition::from_work_area(0, 0, 1920, 1080, 1.0, false);
        assert_eq!(anchor.origin(200.0), (1920 - 380, 1080 - 200));
    }

    #[test]
    fn origin_scales_window_size_to_physical_pixels() {
        let anchor = AnchorPosition::from_work_area(0, 0, 3840, 2100, 2.0, false);
        assert_eq!(anchor.origin(140.0), (3840 - 760, 2100 - 280));
    }

    #[test]
    fn missing_ack_counts_as_expired() {
        assert!(ack_expired(None, Instant::now()));
    }
}
