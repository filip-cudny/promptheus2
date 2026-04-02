use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use super::settings::AppState;
use crate::services::dock::DockManager;
use crate::services::ui_state::WindowGeometry;

const GEOMETRY_KEY: &str = "prompt-dialog";
const DEFAULT_WIDTH: f64 = 700.0;
const DEFAULT_HEIGHT: f64 = 600.0;

#[tauri::command]
pub async fn open_prompt_dialog(
    app: tauri::AppHandle,
    prompt_id: String,
    prompt_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
) -> Result<(), String> {
    let label = "prompt-dialog";

    if let Some(existing) = app.get_webview_window(label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        if let Some(entry_id) = history_entry_id {
            app.emit_to(
                label,
                "restore-history",
                serde_json::json!({
                    "entry_id": entry_id,
                    "last_interaction_only": last_interaction_only.unwrap_or(false),
                }),
            )
            .map_err(|e| e.to_string())?;
        } else if let Some(input) = &initial_input {
            app.emit_to(
                label,
                "voice-input",
                serde_json::json!({
                    "text": input,
                    "auto_send": auto_send_input.unwrap_or(false),
                }),
            )
            .map_err(|e| e.to_string())?;
        } else {
            app.emit_to(
                label,
                "open-for-skill",
                serde_json::json!({
                    "prompt_id": prompt_id,
                    "prompt_name": prompt_name,
                }),
            )
            .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    let mut url = format!(
        "prompt-dialog.html?promptId={}&promptName={}",
        prompt_id,
        urlencoding::encode(&prompt_name),
    );
    if let Some(entry_id) = history_entry_id {
        url.push_str(&format!("&historyEntryId={}", entry_id));
    }
    if last_interaction_only.unwrap_or(false) {
        url.push_str("&lastInteractionOnly=true");
    }
    if let Some(input) = &initial_input {
        url.push_str(&format!("&initialInput={}", urlencoding::encode(input)));
        if auto_send_input.unwrap_or(false) {
            url.push_str("&autoSendInput=true");
        }
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
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Promptheus — chat")
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
            save_geometry(&app_handle, GEOMETRY_KEY);
        }
        tauri::WindowEvent::Destroyed => {
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        _ => {}
    });

    Ok(())
}

fn save_geometry(app: &tauri::AppHandle, geometry_key: &str) {
    let Some(win) = app.get_webview_window("prompt-dialog") else {
        return;
    };

    let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
        return;
    };

    let geom = WindowGeometry {
        x: pos.x as f64,
        y: pos.y as f64,
        width: size.width as f64,
        height: size.height as f64,
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
