use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use crate::commands::settings::AppState;

#[derive(Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn list_mcp_tools(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<Vec<McpToolInfo>> {
    let state = state.lock().await;
    let tools = state
        .mcp
        .tools_with_server()
        .iter()
        .map(|(server, tool)| McpToolInfo {
            name: tool.name.to_string(),
            server: server.clone(),
            description: tool.description.as_ref().map(|d| d.to_string()),
        })
        .collect();
    Ok(tools)
}
