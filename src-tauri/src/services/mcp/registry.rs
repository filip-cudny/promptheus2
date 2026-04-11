use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use crate::models::settings::McpServerConfig;

use super::client::{McpClient, McpError};

pub struct McpRegistry {
    servers: HashMap<String, McpClient>,
    tool_index: HashMap<String, String>,
    tools: Vec<Tool>,
}

impl McpRegistry {
    pub fn empty() -> Self {
        Self {
            servers: HashMap::new(),
            tool_index: HashMap::new(),
            tools: Vec::new(),
        }
    }

    pub async fn start_all(config: &HashMap<String, McpServerConfig>) -> Self {
        let mut servers = HashMap::new();
        let mut tool_index = HashMap::new();
        let mut tools = Vec::new();

        for (name, server_config) in config {
            let resolved_env = server_config.resolved_env();
            let client = match McpClient::start(name, &server_config.command, &server_config.args, &resolved_env).await {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to start MCP server '{}': {}", name, e);
                    continue;
                }
            };

            match client.list_tools().await {
                Ok(server_tools) => {
                    for tool in server_tools {
                        let tool_name = tool.name.to_string();
                        if tool_index.contains_key(&tool_name) {
                            log::warn!(
                                "MCP tool '{}' from server '{}' conflicts with existing tool, skipping",
                                tool_name, name
                            );
                        } else {
                            tool_index.insert(tool_name, name.clone());
                            tools.push(tool);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to list tools from MCP server '{}': {}", name, e);
                }
            }

            servers.insert(name.clone(), client);
        }

        log::info!(
            "MCP registry started: {} servers, {} tools",
            servers.len(),
            tools.len()
        );

        Self {
            servers,
            tool_index,
            tools,
        }
    }

    pub fn all_tools(&self) -> &[Tool] {
        &self.tools
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<CallToolResult, McpError> {
        let server_name = self
            .tool_index
            .get(tool_name)
            .ok_or_else(|| McpError::Protocol(format!("unknown tool '{}'", tool_name)))?;

        let client = self
            .servers
            .get(server_name)
            .ok_or_else(|| McpError::Protocol(format!("server '{}' not found", server_name)))?;

        client.call_tool(tool_name, arguments).await
    }

    pub async fn shutdown_all(self) {
        for (name, client) in self.servers {
            log::info!("Shutting down MCP server '{}'", name);
            client.shutdown().await;
        }
    }

    pub fn has_tools(&self) -> bool {
        !self.tools.is_empty()
    }
}
