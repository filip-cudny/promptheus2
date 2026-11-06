use std::sync::Mutex;
use std::time::Duration;

use tauri::{LogicalSize, Manager};

use crate::services::dialog::{window, SETTINGS_DIALOG_LABEL};
use crate::services::ui_state::WindowGeometry;

pub const PANEL_WIDTH: f64 = 380.0;
const RESIZE_WAIT: Duration = Duration::from_millis(300);
const RESIZE_POLL: Duration = Duration::from_millis(30);

static ADDED_WIDTH: Mutex<f64> = Mutex::new(0.0);

pub fn added_width() -> f64 {
    *ADDED_WIDTH.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn reset() {
    set_added_width(0.0);
}

fn set_added_width(width: f64) {
    *ADDED_WIDTH.lock().unwrap_or_else(|e| e.into_inner()) = width;
}

pub fn geometry_without_panel(geometry: WindowGeometry, added_width: f64) -> WindowGeometry {
    WindowGeometry {
        width: geometry.width - added_width,
        ..geometry
    }
}

#[tauri::command]
pub async fn settings_agent_panel(app: tauri::AppHandle, open: bool) -> crate::Result<&'static str> {
    let win = window(&app, SETTINGS_DIALOG_LABEL)?;
    let scale = win.scale_factor()?;
    let size = win.inner_size()?;
    let logical_w = size.width as f64 / scale;
    let logical_h = size.height as f64 / scale;

    if !open {
        let added = added_width();
        if added > 0.0 {
            win.set_size(LogicalSize::new(logical_w - added, logical_h))?;
            reset();
            log::debug!(target: "app_lib::services::settings_agent::panel", "panel closed, shrunk by {added}");
        }
        return Ok("overlay");
    }

    if added_width() > 0.0 {
        return Ok("widened");
    }

    let fits = match win.current_monitor()? {
        Some(monitor) => {
            let work = monitor.work_area();
            let pos = win.outer_position()?;
            let right = pos.x as f64 + (logical_w + PANEL_WIDTH) * scale;
            right <= (work.position.x as f64 + work.size.width as f64)
        }
        None => false,
    };
    if win.is_maximized()? || !fits {
        log::debug!(target: "app_lib::services::settings_agent::panel", "panel overlays: maximized or no room");
        return Ok("overlay");
    }

    win.set_size(LogicalSize::new(logical_w + PANEL_WIDTH, logical_h))?;
    let mut waited = Duration::ZERO;
    while waited < RESIZE_WAIT {
        tokio::time::sleep(RESIZE_POLL).await;
        waited += RESIZE_POLL;
        if win.inner_size()?.width != size.width {
            set_added_width(PANEL_WIDTH);
            return Ok("widened");
        }
    }
    log::debug!(target: "app_lib::services::settings_agent::panel", "window size unchanged after resize, panel overlays");
    Ok("overlay")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(width: f64) -> WindowGeometry {
        WindowGeometry { x: 10.0, y: 20.0, width, height: 640.0 }
    }

    #[test]
    fn saved_geometry_drops_the_added_width() {
        let saved = geometry_without_panel(geometry(960.0 + PANEL_WIDTH), PANEL_WIDTH);
        assert_eq!(saved.width, 960.0);
        assert_eq!((saved.x, saved.y, saved.height), (10.0, 20.0, 640.0));
    }

    #[test]
    fn saved_geometry_is_unchanged_without_added_width() {
        assert_eq!(geometry_without_panel(geometry(960.0), 0.0).width, 960.0);
    }
}
