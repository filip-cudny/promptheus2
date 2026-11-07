use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;

use crate::services::execution::stream::call_mcp_tool;
use crate::services::mcp::McpRegistry;

use super::tools;

pub struct McpCallResult {
    pub text: String,
    pub is_error: bool,
}

#[async_trait]
pub trait McpSource: Send + Sync {
    fn tools(&self, qualified_names: &[String]) -> Vec<(String, Tool)>;

    async fn call(&self, qualified_name: &str, arguments: Value) -> McpCallResult;
}

pub struct RegistrySource(pub Arc<McpRegistry>);

#[async_trait]
impl McpSource for RegistrySource {
    fn tools(&self, qualified_names: &[String]) -> Vec<(String, Tool)> {
        qualified_names
            .iter()
            .filter_map(|name| {
                let found = self.0.get_tools_by_qualified_names(std::slice::from_ref(name));
                found.into_iter().next().map(|tool| (name.clone(), tool))
            })
            .collect()
    }

    async fn call(&self, qualified_name: &str, arguments: Value) -> McpCallResult {
        let (text, is_error) = call_mcp_tool(&self.0, qualified_name, arguments).await;
        McpCallResult { text, is_error }
    }
}

#[derive(Default)]
pub struct EnabledTools {
    pub definitions: Vec<Tool>,
    pub qualified_by_raw: BTreeMap<String, String>,
}

pub fn select_enabled(source: &dyn McpSource, enabled: &[String]) -> EnabledTools {
    let candidates = source.tools(enabled);
    let agent_tools: Vec<String> = tools::definitions().iter().map(|tool| tool.name.to_string()).collect();
    let mut selected = EnabledTools::default();
    for (qualified, tool) in &candidates {
        let raw = tool.name.to_string();
        if agent_tools.contains(&raw) {
            log::warn!("settings agent: MCP tool '{qualified}' not offered, its name is a settings tool");
        } else if candidates.iter().filter(|(_, other)| other.name == tool.name).count() > 1 {
            log::warn!("settings agent: MCP tool '{qualified}' not offered, another enabled tool shares its name '{raw}'");
        } else {
            selected.qualified_by_raw.insert(raw, qualified.clone());
            selected.definitions.push(tool.clone());
        }
    }
    selected
}

#[cfg(test)]
pub struct ScriptedMcp {
    offered: Vec<(String, Tool)>,
    reply: McpCallResult,
    pub calls: std::sync::Mutex<Vec<(String, Value)>>,
}

#[cfg(test)]
impl ScriptedMcp {
    pub fn offering(qualified_names: &[&str], reply_text: &str) -> Self {
        let offered = qualified_names
            .iter()
            .map(|qualified| {
                let raw = qualified.rsplit_once('.').map_or(*qualified, |(_, raw)| raw);
                let schema = Arc::new(serde_json::Map::new());
                (qualified.to_string(), Tool::new(raw.to_string(), "scripted tool", schema))
            })
            .collect();
        Self {
            offered,
            reply: McpCallResult { text: reply_text.to_string(), is_error: false },
            calls: std::sync::Mutex::default(),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl McpSource for ScriptedMcp {
    fn tools(&self, qualified_names: &[String]) -> Vec<(String, Tool)> {
        qualified_names
            .iter()
            .filter_map(|name| self.offered.iter().find(|(qualified, _)| qualified == name).cloned())
            .collect()
    }

    async fn call(&self, qualified_name: &str, arguments: Value) -> McpCallResult {
        self.calls.lock().unwrap().push((qualified_name.to_string(), arguments));
        McpCallResult { text: self.reply.text.clone(), is_error: self.reply.is_error }
    }
}
