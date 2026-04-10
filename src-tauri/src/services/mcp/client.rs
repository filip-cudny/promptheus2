use std::collections::HashMap;
use std::time::Duration;

use rmcp::model::{CallToolRequestParams, CallToolResult, Tool};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::ServiceExt;
use serde_json::Value;

const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("failed to start MCP server '{server}': {reason}")]
    StartFailed { server: String, reason: String },
    #[error("MCP protocol error: {0}")]
    Protocol(String),
    #[error("MCP server '{server}' timed out")]
    Timeout { server: String },
    #[error("MCP service error: {0}")]
    Service(String),
}

pub struct McpClient {
    service: RunningService<rmcp::RoleClient, ()>,
    server_name: String,
}

impl McpClient {
    pub async fn start(
        server_name: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);
        cmd.envs(env);

        let transport =
            TokioChildProcess::new(cmd).map_err(|e| McpError::StartFailed {
                server: server_name.to_string(),
                reason: e.to_string(),
            })?;

        let service = ().serve(transport).await.map_err(|e| McpError::StartFailed {
            server: server_name.to_string(),
            reason: e.to_string(),
        })?;

        log::info!("MCP server '{}' started", server_name);

        Ok(Self {
            service,
            server_name: server_name.to_string(),
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>, McpError> {
        let tools = self
            .service
            .peer()
            .list_all_tools()
            .await
            .map_err(|e| McpError::Service(e.to_string()))?;

        log::debug!(
            "MCP server '{}': listed {} tools",
            self.server_name,
            tools.len()
        );

        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, McpError> {
        let args_map = match arguments {
            Value::Object(map) => map,
            other => {
                let mut map = serde_json::Map::new();
                map.insert("value".to_string(), other);
                map
            }
        };

        let params = CallToolRequestParams::new(name.to_string()).with_arguments(args_map);

        let result = tokio::time::timeout(TOOL_CALL_TIMEOUT, self.service.peer().call_tool(params))
            .await
            .map_err(|_| McpError::Timeout {
                server: self.server_name.clone(),
            })?
            .map_err(|e| McpError::Service(e.to_string()))?;

        log::debug!(
            "MCP server '{}': tool '{}' returned {} content items",
            self.server_name,
            name,
            result.content.len()
        );

        Ok(result)
    }

    pub async fn shutdown(self) {
        log::info!("MCP server '{}' shutting down", self.server_name);
        drop(self.service);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_error_display_start_failed() {
        let err = McpError::StartFailed {
            server: "test-server".to_string(),
            reason: "command not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "failed to start MCP server 'test-server': command not found"
        );
    }

    #[test]
    fn mcp_error_display_protocol() {
        let err = McpError::Protocol("invalid json".to_string());
        assert_eq!(err.to_string(), "MCP protocol error: invalid json");
    }

    #[test]
    fn mcp_error_display_timeout() {
        let err = McpError::Timeout {
            server: "slow-server".to_string(),
        };
        assert_eq!(err.to_string(), "MCP server 'slow-server' timed out");
    }

    #[test]
    fn mcp_error_display_service() {
        let err = McpError::Service("connection lost".to_string());
        assert_eq!(err.to_string(), "MCP service error: connection lost");
    }

    #[test]
    fn deserialize_tool_from_mcp_response() {
        let json = serde_json::json!({
            "name": "web_search",
            "description": "Search the web for information",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }
        });

        let tool: Tool = serde_json::from_value(json).expect("should deserialize Tool");
        assert_eq!(&*tool.name, "web_search");
        assert_eq!(
            tool.description.as_deref(),
            Some("Search the web for information")
        );
    }

    #[test]
    fn deserialize_call_tool_result_from_mcp_response() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "Result text here..." }
            ],
            "isError": false
        });

        let result: CallToolResult =
            serde_json::from_value(json).expect("should deserialize CallToolResult");
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 1);
    }
}
