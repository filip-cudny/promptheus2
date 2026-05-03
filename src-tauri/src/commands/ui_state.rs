use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::services::ui_state::UiStateService;

#[tauri::command]
pub async fn get_ui_state(
    ui_state: State<'_, Arc<Mutex<UiStateService>>>,
    key: String,
) -> crate::Result<Option<serde_json::Value>> {
    Ok(ui_state.lock().await.get(&key))
}

#[tauri::command]
pub async fn set_ui_state(
    ui_state: State<'_, Arc<Mutex<UiStateService>>>,
    key: String,
    value: serde_json::Value,
) -> crate::Result<()> {
    ui_state.lock().await.set(&key, value)?;
    Ok(())
}
