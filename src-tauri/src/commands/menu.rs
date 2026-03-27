use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::menu::MenuItem;

#[tauri::command]
pub async fn get_context_menu_items(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<MenuItem>, String> {
    let state = state.lock().await;
    Ok(state.menu_coordinator.get_menu_items(&state.config))
}

#[tauri::command]
pub async fn execute_menu_item(
    state: State<'_, Mutex<AppState>>,
    item_id: String,
    shift_pressed: bool,
) -> Result<(), String> {
    let _state = state.lock().await;
    log::debug!("execute_menu_item: id={item_id}, shift={shift_pressed}");
    Ok(())
}

#[tauri::command]
pub async fn show_context_menu_window(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    if let Ok(pos) = win.cursor_position() {
        log::debug!("positioning context menu at ({}, {})", pos.x, pos.y);
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: pos.x as i32,
            y: pos.y as i32,
        }));
    }

    app.emit_to("context-menu", "show-context-menu", ())
        .map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn refresh_menu_providers(
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.menu_coordinator.refresh_all();
    Ok(())
}
