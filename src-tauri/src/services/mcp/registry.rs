use std::collections::{HashMap, HashSet};
use std::time::Duration;

use rmcp::model::{CallToolResult, Tool};
use serde::Serialize;
use serde_json::Value;

use crate::models::settings::McpServerConfig;

use super::client::{McpClient, McpError};

#[derive(Debug, Serialize)]
pub struct McpServerState {
    pub name: String,
    pub state: &'static str,
    pub error: Option<String>,
    pub tools: Vec<String>,
}

pub fn without_disabled_tools(
    qualified_names: &[String],
    servers: &HashMap<String, McpServerConfig>,
) -> Vec<String> {
    let disabled: HashSet<String> = servers
        .iter()
        .flat_map(|(server, config)| {
            config
                .disabled_tools
                .iter()
                .map(move |tool| McpRegistry::qualified_name(server, tool))
        })
        .collect();
    qualified_names
        .iter()
        .filter(|name| !disabled.contains(*name))
        .cloned()
        .collect()
}

pub struct McpRegistry {
    servers: HashMap<String, McpClient>,
    start_errors: HashMap<String, String>,
    skipped: HashSet<String>,
    qualified_index: HashMap<String, (String, String)>,
    tools: Vec<(String, Tool)>,
}

impl McpRegistry {
    fn qualified_name(server: &str, tool: &str) -> String {
        format!("{}.{}", server, tool)
    }

    pub async fn start_all(config: &HashMap<String, McpServerConfig>) -> Self {
        let mut servers = HashMap::new();
        let mut start_errors = HashMap::new();
        let mut skipped = HashSet::new();
        let mut qualified_index = HashMap::new();
        let mut tools = Vec::new();

        for (name, server_config) in config {
            if server_config.disabled {
                log::info!("MCP server '{}' is disabled, not starting", name);
                skipped.insert(name.clone());
                continue;
            }
            let resolved_env = server_config.resolved_env();
            let resolved_command = server_config.resolved_command();
            let resolved_args = server_config.resolved_args();
            let client = match McpClient::start(name, &resolved_command, &resolved_args, &resolved_env, server_config.timeout_secs, &server_config.tool_timeouts).await {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to start MCP server '{}': {}", name, e);
                    start_errors.insert(name.clone(), e.to_string());
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
            start_errors,
            skipped,
            qualified_index,
            tools,
        }
    }

    pub fn server_states(&self) -> Vec<McpServerState> {
        let mut states: Vec<McpServerState> = self
            .servers
            .keys()
            .map(|name| McpServerState {
                name: name.clone(),
                state: "running",
                error: None,
                tools: self
                    .tools
                    .iter()
                    .filter(|(server, _)| server == name)
                    .map(|(_, tool)| tool.name.to_string())
                    .collect(),
            })
            .chain(self.start_errors.iter().map(|(name, error)| McpServerState {
                name: name.clone(),
                state: "error",
                error: Some(error.clone()),
                tools: Vec::new(),
            }))
            .chain(self.skipped.iter().map(|name| McpServerState {
                name: name.clone(),
                state: "off",
                error: None,
                tools: Vec::new(),
            }))
            .collect();
        states.sort_by(|a, b| a.name.cmp(&b.name));
        states
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
                    if !self.is_of_skipped_server(qname) {
                        log::warn!("MCP tool '{}' not found in registry, ignoring", qname);
                    }
                    None
                }
            })
            .collect()
    }

    fn is_of_skipped_server(&self, qname: &str) -> bool {
        self.skipped
            .iter()
            .any(|server| qname.starts_with(&format!("{server}.")))
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

    pub async fn close_all(&self, timeout: Duration) {
        futures::future::join_all(self.servers.values().map(|client| client.close(timeout))).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn close_all_on_empty_registry_returns() {
        let registry = McpRegistry::start_all(&HashMap::new()).await;
        registry.close_all(Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn failed_start_is_listed_with_its_error() {
        let config = HashMap::from([(
            "broken".to_string(),
            McpServerConfig {
                command: "/nonexistent/promptheus-mcp-test-server".to_string(),
                args: vec![],
                env: HashMap::new(),
                timeout_secs: 180,
                tool_timeouts: HashMap::new(),
                ..Default::default()
            },
        )]);
        let registry = McpRegistry::start_all(&config).await;

        let states = registry.server_states();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].name, "broken");
        assert_eq!(states[0].state, "error");
        assert!(!states[0].error.as_deref().unwrap_or("").is_empty());
        assert!(registry.tools_with_server().is_empty());
    }

    async fn disabled_server_registry() -> McpRegistry {
        let config = HashMap::from([(
            "off-server".to_string(),
            McpServerConfig {
                command: "/nonexistent/promptheus-mcp-test-server".to_string(),
                disabled: true,
                ..Default::default()
            },
        )]);
        McpRegistry::start_all(&config).await
    }

    #[tokio::test]
    async fn disabled_server_is_not_started_and_reported_off() {
        let registry = disabled_server_registry().await;

        let states = registry.server_states();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].name, "off-server");
        assert_eq!(states[0].state, "off");
        assert!(states[0].error.is_none());
        assert!(states[0].tools.is_empty());
    }

    #[tokio::test]
    async fn qualified_names_of_a_skipped_server_resolve_to_nothing() {
        let registry = disabled_server_registry().await;

        assert!(registry.is_of_skipped_server("off-server.search"));
        assert!(!registry.is_of_skipped_server("off-server-two.search"));
        assert!(registry
            .get_tools_by_qualified_names(&["off-server.search".to_string(), "other.search".to_string()])
            .is_empty());
    }

    #[test]
    fn disabled_tools_are_dropped_from_qualified_names() {
        let servers = HashMap::from([
            (
                "srv".to_string(),
                McpServerConfig { disabled_tools: vec!["web_search".to_string()], ..Default::default() },
            ),
            ("srv-two".to_string(), McpServerConfig::default()),
        ]);
        let names: Vec<String> = ["srv.web_search", "srv.echo", "srv-two.web_search", "web_search", "other.web_search"]
            .iter()
            .map(|name| name.to_string())
            .collect();

        assert_eq!(
            without_disabled_tools(&names, &servers),
            vec!["srv.echo", "srv-two.web_search", "web_search", "other.web_search"]
        );
    }
}
