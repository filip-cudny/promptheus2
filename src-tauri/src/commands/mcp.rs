use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::Tool;
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use crate::models::settings::McpServerConfig;
use crate::services::config::ConfigService;
use crate::services::mcp::registry::McpServerState;
use crate::services::mcp::McpRegistry;

#[derive(Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
    pub disabled: bool,
}

fn tool_infos(
    tools: &[(String, Tool)],
    servers: &HashMap<String, McpServerConfig>,
) -> Vec<McpToolInfo> {
    tools
        .iter()
        .map(|(server, tool)| McpToolInfo {
            name: tool.name.to_string(),
            server: server.clone(),
            description: tool.description.as_ref().map(|d| d.to_string()),
            disabled: servers
                .get(server)
                .is_some_and(|config| config.disabled_tools.iter().any(|t| t == tool.name.as_ref())),
        })
        .collect()
}

#[tauri::command]
pub async fn list_mcp_tools(
    mcp: State<'_, Arc<McpRegistry>>,
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<McpToolInfo>> {
    let servers = config.lock().await.settings().mcp_servers.clone();
    Ok(tool_infos(mcp.tools_with_server(), &servers))
}

#[tauri::command]
pub async fn list_mcp_servers(
    mcp: State<'_, Arc<McpRegistry>>,
) -> crate::Result<Vec<McpServerState>> {
    Ok(mcp.server_states())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str) -> Tool {
        Tool::new(name.to_string(), "a tool", Arc::new(serde_json::Map::new()))
    }

    #[test]
    fn listed_tool_in_disabled_tools_is_marked_disabled() {
        let servers = HashMap::from([(
            "srv".to_string(),
            McpServerConfig { disabled_tools: vec!["web_search".to_string()], ..Default::default() },
        )]);
        let tools = vec![
            ("srv".to_string(), tool("web_search")),
            ("srv".to_string(), tool("echo")),
            ("other".to_string(), tool("web_search")),
        ];

        let listed = tool_infos(&tools, &servers);

        let flags: Vec<(&str, &str, bool)> = listed
            .iter()
            .map(|info| (info.server.as_str(), info.name.as_str(), info.disabled))
            .collect();
        assert_eq!(
            flags,
            vec![("srv", "web_search", true), ("srv", "echo", false), ("other", "web_search", false)]
        );
    }
}
