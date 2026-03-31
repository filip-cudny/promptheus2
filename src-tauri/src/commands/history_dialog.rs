use tauri::Manager;
use tokio::sync::Mutex;

use super::settings::AppState;
use crate::services::dock::DockManager;
use crate::services::ui_state::WindowGeometry;

const GEOMETRY_KEY: &str = "history-dialog";
const DEFAULT_WIDTH: f64 = 600.0;
const DEFAULT_HEIGHT: f64 = 500.0;

#[tauri::command]
pub async fn open_history_dialog(app: tauri::AppHandle) -> Result<(), String> {
    let label = "history-dialog";

    if let Some(existing) = app.get_webview_window(label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let state = app.state::<Mutex<AppState>>();
    let geometry = state.lock().await.ui_state.get_geometry(GEOMETRY_KEY);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));

    let mut builder = tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App("history-dialog.html".into()),
    )
    .title("Execution History")
    .inner_size(width, height)
    .resizable(true)
    .decorations(true);

    if let Some(g) = &geometry {
        builder = builder.position(g.x, g.y);
    }

    let win = builder.build().map_err(|e| e.to_string())?;

    let dock = app.state::<DockManager>();
    dock.dialog_opened(&app);

    let app_handle = app.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            if let Some(win) = app_handle.get_webview_window(label) {
                let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
                    return;
                };
                let geom = WindowGeometry {
                    x: pos.x as f64,
                    y: pos.y as f64,
                    width: size.width as f64,
                    height: size.height as f64,
                };
                let app = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<Mutex<AppState>>();
                    let mut s = state.lock().await;
                    if let Err(e) = s.ui_state.set_geometry(GEOMETRY_KEY, geom) {
                        log::warn!("failed to save history dialog geometry: {e}");
                    }
                });
            }
        }
        tauri::WindowEvent::Destroyed => {
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        _ => {}
    });

    Ok(())
}
