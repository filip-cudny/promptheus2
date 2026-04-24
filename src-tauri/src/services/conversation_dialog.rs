use tauri::{LogicalPosition, Manager, WebviewUrl, WindowEvent};
use tokio::sync::Mutex;

use super::ai_webview;
use super::dialog::{focus_window, save_geometry};
use super::dock::DockManager;
use crate::commands::settings::AppState;

const BASE_LABEL: &str = "conversation-dialog";
const BASE_TITLE: &str = "Promptheus — chat";
const DEFAULT_WIDTH: f64 = 700.0;
const DEFAULT_HEIGHT: f64 = 600.0;
const NEW_WINDOW_OFFSET: f64 = 32.0;

pub async fn open_new_instance(app: &tauri::AppHandle) -> Result<(), String> {
    let label = next_available_label(app);

    let geometry = seed_geometry_for_new_instance(app, &label).await;

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));

    let mut window_builder = tauri::window::WindowBuilder::new(app, &label)
        .title(BASE_TITLE)
        .inner_size(width, height)
        .resizable(true)
        .decorations(true);

    if let Some(g) = &geometry {
        window_builder = window_builder.position(g.x, g.y);
    }

    let window = window_builder.build().map_err(|e| e.to_string())?;

    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let webview_builder = tauri::webview::WebviewBuilder::new(
        &label,
        WebviewUrl::App("conversation-dialog.html".into()),
    );

    window
        .add_child(webview_builder, LogicalPosition::new(0.0, 0.0), inner)
        .map_err(|e| e.to_string())?;

    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("failed to attach webview for {label}"))?;

    let dock = app.state::<DockManager>();
    dock.dialog_opened(app);

    let app_handle = app.clone();
    let save_label = label.clone();
    let resize_app = app.clone();
    let resize_label = label.clone();
    let destroy_label = label.clone();

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } => {
            save_geometry(&app_handle, &save_label, &save_label);
        }
        WindowEvent::Destroyed => {
            ai_webview::cleanup_host_state(&app_handle, &destroy_label);
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        WindowEvent::Resized(size) => {
            let Some(host) = resize_app.get_window(&resize_label) else {
                return;
            };
            let scale = host.scale_factor().unwrap_or(1.0);
            let logical = tauri::LogicalSize::new(
                size.width as f64 / scale,
                size.height as f64 / scale,
            );
            for webview in host.webviews() {
                let _ = webview.set_size(logical);
                let _ = webview.set_position(LogicalPosition::new(0.0, 0.0));
            }
        }
        _ => {}
    });

    focus_window(&win)?;

    log::info!(
        target: "app_lib::services::conversation_dialog",
        "opened new conversation-dialog instance: {label}",
    );
    Ok(())
}

fn next_available_label(app: &tauri::AppHandle) -> String {
    if app.get_window(BASE_LABEL).is_none() {
        return BASE_LABEL.to_string();
    }
    let mut i = 2u32;
    loop {
        let candidate = format!("{BASE_LABEL}-{i}");
        if app.get_window(&candidate).is_none() {
            return candidate;
        }
        i += 1;
    }
}

async fn seed_geometry_for_new_instance(
    app: &tauri::AppHandle,
    label: &str,
) -> Option<super::ui_state::WindowGeometry> {
    let state = app.state::<Mutex<AppState>>();
    let mut guard = state.lock().await;
    if let Some(g) = guard.ui_state.get_geometry(label) {
        return Some(g);
    }
    let base = guard.ui_state.get_geometry(BASE_LABEL)?;
    let offset = super::ui_state::WindowGeometry {
        x: base.x + NEW_WINDOW_OFFSET,
        y: base.y + NEW_WINDOW_OFFSET,
        width: base.width,
        height: base.height,
    };
    let _ = guard.ui_state.set_geometry(label, offset.clone());
    Some(offset)
}
