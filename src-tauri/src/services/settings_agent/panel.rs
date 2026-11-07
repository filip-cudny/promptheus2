use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{LogicalSize, Manager};

use crate::services::dialog::{window, SETTINGS_DIALOG_LABEL};
use crate::services::ui_state::{UiStateService, WindowGeometry};

const DEFAULT_PANEL_WIDTH: f64 = 380.0;
const MIN_PANEL_WIDTH: f64 = 300.0;
const PANEL_WIDTH_KEY: &str = "settings-dialog.agent_panel_width";
const RESIZE_WAIT: Duration = Duration::from_millis(300);
const RESIZE_POLL: Duration = Duration::from_millis(30);

#[derive(Default)]
pub struct PanelState(Mutex<f64>);

impl PanelState {
    fn get(&self) -> f64 {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set(&self, width: f64) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = width;
    }
}

pub fn added_width(app: &tauri::AppHandle) -> f64 {
    app.try_state::<PanelState>().map_or(0.0, |s| s.get())
}

pub fn reset(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<PanelState>() {
        state.set(0.0);
    }
}

fn saved_width(value: Option<serde_json::Value>) -> f64 {
    value
        .and_then(|v| v.as_f64())
        .map_or(DEFAULT_PANEL_WIDTH, |w| w.max(MIN_PANEL_WIDTH))
}

async fn read_saved_width(app: &tauri::AppHandle) -> f64 {
    let Some(ui_state) = app.try_state::<Arc<tokio::sync::Mutex<UiStateService>>>() else {
        return DEFAULT_PANEL_WIDTH;
    };
    let value = ui_state.lock().await.get(PANEL_WIDTH_KEY);
    saved_width(value)
}

pub fn geometry_without_panel(geometry: WindowGeometry, added_width: f64) -> WindowGeometry {
    WindowGeometry {
        width: geometry.width - added_width,
        ..geometry
    }
}

pub async fn set_open(app: &tauri::AppHandle, open: bool) -> crate::Result<&'static str> {
    app.manage(PanelState::default());
    let state = app.state::<PanelState>();
    let win = window(app, SETTINGS_DIALOG_LABEL)?;
    let scale = win.scale_factor()?;
    let size = win.inner_size()?;
    let logical_w = size.width as f64 / scale;
    let logical_h = size.height as f64 / scale;

    if !open {
        let added = state.get();
        if added > 0.0 {
            win.set_size(LogicalSize::new(logical_w - added, logical_h))?;
            state.set(0.0);
            log::debug!(target: "app_lib::services::settings_agent::panel", "panel closed, shrunk by {added}");
        }
        return Ok("overlay");
    }

    if state.get() > 0.0 {
        return Ok("widened");
    }

    let panel_width = read_saved_width(app).await;
    log::debug!(target: "app_lib::services::settings_agent::panel", "panel width {panel_width}");

    let fits = match win.current_monitor()? {
        Some(monitor) => {
            let work = monitor.work_area();
            let pos = win.outer_position()?;
            let right = pos.x as f64 + (logical_w + panel_width) * scale;
            right <= (work.position.x as f64 + work.size.width as f64)
        }
        None => false,
    };
    if win.is_maximized()? || !fits {
        log::debug!(target: "app_lib::services::settings_agent::panel", "panel overlays: maximized or no room");
        return Ok("overlay");
    }

    win.set_size(LogicalSize::new(logical_w + panel_width, logical_h))?;
    let mut waited = Duration::ZERO;
    while waited < RESIZE_WAIT {
        tokio::time::sleep(RESIZE_POLL).await;
        waited += RESIZE_POLL;
        if win.inner_size()?.width != size.width {
            state.set(panel_width);
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
        let saved = geometry_without_panel(
            geometry(960.0 + DEFAULT_PANEL_WIDTH),
            DEFAULT_PANEL_WIDTH,
        );
        assert_eq!(saved.width, 960.0);
        assert_eq!((saved.x, saved.y, saved.height), (10.0, 20.0, 640.0));
    }

    #[test]
    fn saved_geometry_is_unchanged_without_added_width() {
        assert_eq!(geometry_without_panel(geometry(960.0), 0.0).width, 960.0);
    }

    #[test]
    fn absent_saved_width_is_the_default() {
        assert_eq!(saved_width(None), DEFAULT_PANEL_WIDTH);
    }

    #[test]
    fn non_number_saved_width_is_the_default() {
        assert_eq!(saved_width(Some(serde_json::json!("wide"))), DEFAULT_PANEL_WIDTH);
    }

    #[test]
    fn saved_width_below_the_minimum_is_raised_to_it() {
        assert_eq!(saved_width(Some(serde_json::json!(120))), MIN_PANEL_WIDTH);
    }

    #[test]
    fn saved_width_in_range_is_kept() {
        assert_eq!(saved_width(Some(serde_json::json!(450))), 450.0);
    }
}
