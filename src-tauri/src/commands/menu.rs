use tauri::State;
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
    eprintln!("execute_menu_item: id={item_id}, shift={shift_pressed}");
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
