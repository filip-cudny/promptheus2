use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use crate::models::settings::McpServerConfig;

use super::client::{McpClient, McpError};

pub struct McpRegistry {
    servers: HashMap<String, McpClient>,
    qualified_index: HashMap<String, (String, String)>,
    tools: Vec<(String, Tool)>,
}

impl McpRegistry {
    pub fn empty() -> Self {
        Self {
            servers: HashMap::new(),
            qualified_index: HashMap::new(),
            tools: Vec::new(),
        }
    }

    fn qualified_name(server: &str, tool: &str) -> String {
        format!("{}.{}", server, tool)
    }

    pub async fn start_all(config: &HashMap<String, McpServerConfig>) -> Self {
        let mut servers = HashMap::new();
        let mut qualified_index = HashMap::new();
        let mut tools = Vec::new();

        for (name, server_config) in config {
            let resolved_env = server_config.resolved_env();
            let resolved_command = server_config.resolved_command();
            let resolved_args = server_config.resolved_args();
            let client = match McpClient::start(name, &resolved_command, &resolved_args, &resolved_env).await {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to start MCP server '{}': {}", name, e);
                    continue;
                }
            };

            match client.list_tools().await {
                Ok(server_tools) => {
                    for tool in server_tools {
                        let raw_name = tool.name.to_string();
                        let qname = Self::qualified_name(name, &raw_name);
                        if qualified_index.contains_key(&qname) {
                            log::warn!(
                                "MCP tool '{}' from server '{}' conflicts with existing tool, skipping",
                                raw_name, name
                            );
                        } else {
                            qualified_index.insert(qname, (name.clone(), raw_name));
                            tools.push((name.clone(), tool));
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
            qualified_index,
            tools,
        }
    }

    pub fn tools_with_server(&self) -> &[(String, Tool)] {
        &self.tools
    }

    pub fn get_tools_by_qualified_names(&self, names: &[String]) -> Vec<Tool> {
        names
            .iter()
            .filter_map(|qname| {
                if self.qualified_index.contains_key(qname) {
                    self.tools
                        .iter()
                        .find(|(server, tool)| Self::qualified_name(server, &tool.name) == *qname)
                        .map(|(_, tool)| tool.clone())
                } else {
                    log::warn!("MCP tool '{}' not found in registry, ignoring", qname);
                    None
                }
            })
            .collect()
    }

    pub fn resolve_raw_name(&self, raw_name: &str) -> Option<String> {
        self.qualified_index
            .keys()
            .find(|qname| {
                qname.rsplit_once('.').map(|(_, name)| name) == Some(raw_name)
            })
            .cloned()
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, McpError> {
        let qualified_name = if name.contains('.') {
            name.to_string()
        } else {
            self.resolve_raw_name(name)
                .ok_or_else(|| McpError::Protocol(format!("unknown tool '{}'", name)))?
        };
        let (server_name, raw_name) = self
            .qualified_index
            .get(&qualified_name)
            .ok_or_else(|| McpError::Protocol(format!("unknown tool '{}'", qualified_name)))?;

        let client = self
            .servers
            .get(server_name)
            .ok_or_else(|| McpError::Protocol(format!("server '{}' not found", server_name)))?;

        client.call_tool(raw_name, arguments).await
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
