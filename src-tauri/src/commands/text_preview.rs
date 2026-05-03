use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::settings::AppState;
use crate::services::dialog;
use crate::services::dock::DockManager;
use crate::services::monitor::find_monitor_at;

const GEOMETRY_KEY: &str = "text-preview";
const LABEL_PREFIX: &str = "text-preview-";

struct PendingText {
    text: String,
    index: usize,
    source_window: String,
}

static PENDING: Mutex<Option<HashMap<String, PendingText>>> = Mutex::new(None);

fn pending_insert(label: String, payload: PendingText) {
    let mut guard = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(label, payload);
}

fn pending_take(label: &str) -> Option<PendingText> {
    let mut guard = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().and_then(|m| m.remove(label))
}

fn pending_remove(label: &str) {
    let mut guard = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(label);
    }
}

fn label_for(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{}{:016x}", LABEL_PREFIX, hasher.finish())
}

#[derive(Serialize)]
pub struct TextPayload {
    text: String,
    index: usize,
    source_window: String,
}

#[tauri::command]
pub async fn open_text_preview(
    app: tauri::AppHandle,
    text: String,
    index: usize,
    source_window: String,
) -> crate::Result<()> {
    let label = label_for(&text);
    log::debug!("open_text_preview: label={} index={} source={}", label, index, source_window);

    if let Some(win) = app.get_webview_window(&label) {
        log::debug!("open_text_preview: reusing existing window {}", label);
        win.show()?;
        dialog::focus_window(&win).map_err(crate::Error::Other)?;
        return Ok(());
    }

    pending_insert(
        label.clone(),
        PendingText { text, index, source_window },
    );

    let state = app.state::<tokio::sync::Mutex<AppState>>();
    let geometry = state.lock().await.ui_state.get_geometry(GEOMETRY_KEY);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((500.0, 400.0));

    let mut builder = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App("text-preview.html".into()),
    )
    .title("Text Preview")
    .inner_size(width, height)
    .resizable(true)
    .decorations(true)
    .transparent(false)
    .visible(false);

    if let Some(g) = &geometry {
        builder = builder.position(g.x, g.y);
    }

    let win = builder.build().map_err(|e| {
        pending_remove(&label);
        crate::Error::from(e)
    })?;

    if geometry.is_none() {
        position_near_cursor(&win);
    }

    let dock = app.state::<DockManager>();
    dock.dialog_opened(&app);

    let app_handle = app.clone();
    let label_owned = label.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            dialog::save_geometry(&app_handle, &label_owned, GEOMETRY_KEY);
        }
        tauri::WindowEvent::Destroyed => {
            pending_remove(&label_owned);
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        _ => {}
    });

    #[cfg(target_os = "macos")]
    app.show()?;

    win.show()?;
    dialog::focus_window(&win).map_err(crate::Error::Other)?;

    Ok(())
}

fn position_near_cursor(win: &tauri::WebviewWindow) {
    let Ok(pos) = win.cursor_position() else { return };
    let cx = pos.x as i32;
    let cy = pos.y as i32;

    let (x, y) = if let Ok(monitor) = find_monitor_at(win.app_handle(), cx, cy) {
        let work = monitor.work_area();
        let win_size = win.outer_size().unwrap_or(tauri::PhysicalSize {
            width: 500,
            height: 400,
        });

        let right_edge = work.position.x + work.size.width as i32;
        let bottom_edge = work.position.y + work.size.height as i32;

        let mut x = cx;
        let mut y = cy;
        if x + win_size.width as i32 > right_edge {
            x = right_edge - win_size.width as i32;
        }
        if y + win_size.height as i32 > bottom_edge {
            y = bottom_edge - win_size.height as i32;
        }
        if x < work.position.x {
            x = work.position.x;
        }
        if y < work.position.y {
            y = work.position.y;
        }
        (x, y)
    } else {
        (cx, cy)
    };

    let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
}

#[tauri::command]
pub fn save_text_preview_geometry(window: tauri::Window) {
    dialog::save_geometry(window.app_handle(), window.label(), GEOMETRY_KEY);
}

#[tauri::command]
pub fn get_pending_text(window: tauri::Window) -> Option<TextPayload> {
    pending_take(window.label()).map(|p| TextPayload {
        text: p.text,
        index: p.index,
        source_window: p.source_window,
    })
}
