use tauri::State;
use tokio::sync::Mutex;

use super::settings::AppState;

#[tauri::command]
pub async fn get_ui_state(
    state: State<'_, Mutex<AppState>>,
    key: String,
) -> crate::Result<Option<serde_json::Value>> {
    let state = state.lock().await;
    Ok(state.ui_state.get(&key))
}

#[tauri::command]
pub async fn set_ui_state(
    state: State<'_, Mutex<AppState>>,
    key: String,
    value: serde_json::Value,
) -> crate::Result<()> {
    let mut state = state.lock().await;
    state.ui_state.set(&key, value)?;
    Ok(())
}
