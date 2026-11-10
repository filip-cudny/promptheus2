use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{LogicalSize, Manager};

use crate::services::dialog::{window, SETTINGS_DIALOG_LABEL};
use crate::services::ui_state::{UiStateService, WindowGeometry};

const DEFAULT_PANEL_WIDTH: f64 = 380.0;
const MIN_PANEL_WIDTH: f64 = 300.0;
const PANEL_WIDTH_KEY: &str = "settings-dialog.agent_panel_width";
const PANEL_OPEN_KEY: &str = "settings-dialog.agent_panel_open";
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

#[derive(Debug, PartialEq)]
pub enum InitialPanel {
    Hidden,
    Widened(f64),
    Overlay,
}

impl InitialPanel {
    pub fn mode(&self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Widened(_) => "widened",
            Self::Overlay => "overlay",
        }
    }
}

pub fn initial_panel(
    open: Option<&serde_json::Value>,
    panel_width: f64,
    window_x: Option<f64>,
    window_width: f64,
    work_area: Option<(f64, f64)>,
) -> InitialPanel {
    if open.and_then(|v| v.as_bool()) == Some(false) {
        return InitialPanel::Hidden;
    }
    let Some((work_x, work_width)) = work_area else {
        return InitialPanel::Overlay;
    };
    let right = window_x.unwrap_or(work_x) + window_width + panel_width;
    if right <= work_x + work_width {
        InitialPanel::Widened(panel_width)
    } else {
        InitialPanel::Overlay
    }
}

pub fn adopt_widened(app: &tauri::AppHandle, width: f64) {
    app.manage(PanelState::default());
    app.state::<PanelState>().set(width);
}

pub async fn initial_for_new_window(
    app: &tauri::AppHandle,
    geometry_key: &str,
    default_width: f64,
) -> InitialPanel {
    let Some(ui_state) = app.try_state::<Arc<tokio::sync::Mutex<UiStateService>>>() else {
        return InitialPanel::Overlay;
    };
    let (open, width, geometry) = {
        let guard = ui_state.lock().await;
        (
            guard.get(PANEL_OPEN_KEY),
            saved_width(guard.get(PANEL_WIDTH_KEY)),
            guard.get_geometry(geometry_key),
        )
    };
    let work_area = work_area_for(app, geometry.as_ref().map(|g| (g.x, g.y)));
    let window_width = geometry.as_ref().map_or(default_width, |g| g.width);
    let decision = initial_panel(
        open.as_ref(),
        width,
        geometry.as_ref().map(|g| g.x),
        window_width,
        work_area,
    );
    log::debug!(
        target: "app_lib::services::settings_agent::panel",
        "initial panel {decision:?} (open state {open:?}, panel width {width}, window width {window_width}, work area {work_area:?})",
    );
    decision
}

fn work_area_for(app: &tauri::AppHandle, saved_origin: Option<(f64, f64)>) -> Option<(f64, f64)> {
    let monitors = app.available_monitors().ok()?;
    let containing = saved_origin.and_then(|(x, y)| {
        monitors.iter().find(|m| {
            let scale = m.scale_factor();
            let pos = m.position();
            let size = m.size();
            let (left, top) = (pos.x as f64 / scale, pos.y as f64 / scale);
            let (width, height) = (size.width as f64 / scale, size.height as f64 / scale);
            x >= left && x < left + width && y >= top && y < top + height
        })
    });
    let primary = app.primary_monitor().ok().flatten();
    let monitor = containing.or(primary.as_ref())?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    Some((work.position.x as f64 / scale, work.size.width as f64 / scale))
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

    fn open_with(
        open: Option<serde_json::Value>,
        window_x: Option<f64>,
        work_area: Option<(f64, f64)>,
    ) -> InitialPanel {
        initial_panel(open.as_ref(), 380.0, window_x, 960.0, work_area)
    }

    #[test]
    fn remembered_open_with_room_is_widened() {
        let panel = open_with(Some(serde_json::json!(true)), Some(100.0), Some((0.0, 1920.0)));
        assert_eq!(panel, InitialPanel::Widened(380.0));
    }

    #[test]
    fn absent_state_with_room_is_widened() {
        assert_eq!(
            open_with(None, Some(100.0), Some((0.0, 1920.0))),
            InitialPanel::Widened(380.0)
        );
    }

    #[test]
    fn open_without_room_overlays() {
        for open in [Some(serde_json::json!(true)), None] {
            assert_eq!(
                open_with(open, Some(100.0), Some((0.0, 1400.0))),
                InitialPanel::Overlay
            );
        }
    }

    #[test]
    fn remembered_closed_is_hidden_even_with_room() {
        assert_eq!(
            open_with(Some(serde_json::json!(false)), Some(100.0), Some((0.0, 1920.0))),
            InitialPanel::Hidden
        );
    }

    #[test]
    fn missing_saved_position_uses_the_work_area_x() {
        assert_eq!(open_with(None, None, Some((1920.0, 1300.0))), InitialPanel::Overlay);
        assert_eq!(
            open_with(None, None, Some((1920.0, 1340.0))),
            InitialPanel::Widened(380.0)
        );
    }

    #[test]
    fn unknown_work_area_overlays() {
        assert_eq!(open_with(None, Some(100.0), None), InitialPanel::Overlay);
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
