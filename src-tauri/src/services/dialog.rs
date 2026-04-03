use tauri::Manager;
use tokio::sync::Mutex;

use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

pub struct DialogConfig {
    pub label: &'static str,
    pub url: String,
    pub title: &'static str,
    pub default_width: f64,
    pub default_height: f64,
    pub geometry_key: &'static str,
}

pub async fn open_or_focus(
    app: &tauri::AppHandle,
    config: &DialogConfig,
) -> Result<(tauri::WebviewWindow, bool), String> {
    if let Some(existing) = app.get_webview_window(config.label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok((existing, false));
    }

    let state = app.state::<Mutex<AppState>>();
    let geometry = state.lock().await.ui_state.get_geometry(config.geometry_key);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((config.default_width, config.default_height));

    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        config.label,
        tauri::WebviewUrl::App(config.url.clone().into()),
    )
    .title(config.title)
    .inner_size(width, height)
    .resizable(true)
    .decorations(true);

    if let Some(g) = &geometry {
        builder = builder.position(g.x, g.y);
    }

    let win = builder.build().map_err(|e| e.to_string())?;

    let dock = app.state::<DockManager>();
    dock.dialog_opened(app);

    let app_handle = app.clone();
    let label = config.label;
    let geometry_key = config.geometry_key;
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            save_geometry(&app_handle, label, geometry_key);
        }
        tauri::WindowEvent::Destroyed => {
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        _ => {}
    });

    Ok((win, true))
}

pub fn save_geometry(app: &tauri::AppHandle, window_label: &str, geometry_key: &str) {
    let Some(win) = app.get_webview_window(window_label) else {
        return;
    };

    let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
        return;
    };

    let scale = win.scale_factor().unwrap_or(1.0);
    let geom = WindowGeometry {
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    };

    let app = app.clone();
    let key = geometry_key.to_string();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Mutex<AppState>>();
        let mut s = state.lock().await;
        if let Err(e) = s.ui_state.set_geometry(&key, geom) {
            log::warn!("failed to save window geometry: {e}");
        }
    });
}

pub async fn restore_size(app: &tauri::AppHandle, window_label: &str, geometry_key: &str) {
    let Some(win) = app.get_webview_window(window_label) else {
        return;
    };

    let state = app.state::<Mutex<AppState>>();
    let Some(geom) = state.lock().await.ui_state.get_geometry(geometry_key) else {
        return;
    };

    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: geom.width,
        height: geom.height,
    }));
}
