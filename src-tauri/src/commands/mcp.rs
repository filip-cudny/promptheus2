use tauri::State;
use tokio::sync::Mutex;

use crate::commands::settings::AppState;

#[tauri::command]
pub async fn list_mcp_tools(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<rmcp::model::Tool>, String> {
    let state = state.lock().await;
    Ok(state.mcp.all_tools().to_vec())
}
