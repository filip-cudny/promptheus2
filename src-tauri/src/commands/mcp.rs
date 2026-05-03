use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::services::mcp::McpRegistry;

#[derive(Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn list_mcp_tools(
    mcp: State<'_, Arc<McpRegistry>>,
) -> crate::Result<Vec<McpToolInfo>> {
    let tools = mcp
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
