use serde::Deserialize;
use serde_json::json;

use crate::models::settings::{ApiMode, Provider};

#[derive(Debug, Clone, PartialEq)]
pub enum BuiltInTool {
    WebSearch,
}

pub struct ToolRegistry;

impl ToolRegistry {
    pub fn available_tools(provider: &Provider, api_mode: &ApiMode) -> Vec<BuiltInTool> {
        match (provider, api_mode) {
            (Provider::Openai, ApiMode::Responses) => vec![BuiltInTool::WebSearch],
            _ => vec![],
        }
    }

    pub fn resolve_tools(
        requested: &[String],
        provider: &Provider,
        api_mode: &ApiMode,
    ) -> Vec<BuiltInTool> {
        let available = Self::available_tools(provider, api_mode);
        requested
            .iter()
            .filter_map(|name| {
                let tool = match name.as_str() {
                    "web_search" => Some(BuiltInTool::WebSearch),
                    _ => {
                        log::warn!("unknown tool requested: '{name}', ignoring");
                        None
                    }
                };
                tool.filter(|t| {
                    if available.contains(t) {
                        true
                    } else {
                        log::warn!(
                            "tool '{name}' not supported by {provider:?}/{api_mode:?}, ignoring"
                        );
                        false
                    }
                })
            })
            .collect()
    }

    pub fn format_web_search_result(action: &serde_json::Value) -> Option<String> {
        let parsed: WebSearchAction = serde_json::from_value(action.clone()).ok()?;
        if parsed.queries.is_empty() {
            return None;
        }
        Some(parsed.queries.join("\n"))
    }

    pub fn to_request_payload(
        tool: &BuiltInTool,
        provider: &Provider,
        api_mode: &ApiMode,
    ) -> serde_json::Value {
        match (tool, provider, api_mode) {
            (BuiltInTool::WebSearch, Provider::Openai, ApiMode::Responses) => {
                json!({"type": "web_search_preview"})
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Deserialize)]
struct WebSearchAction {
    #[serde(default)]
    queries: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_search_available_on_openai_responses() {
        let tools = ToolRegistry::available_tools(&Provider::Openai, &ApiMode::Responses);
        assert_eq!(tools, vec![BuiltInTool::WebSearch]);
    }

    #[test]
    fn no_tools_on_openai_completions() {
        let tools = ToolRegistry::available_tools(&Provider::Openai, &ApiMode::Completions);
        assert!(tools.is_empty());
    }

    #[test]
    fn no_tools_on_anthropic() {
        let tools = ToolRegistry::available_tools(&Provider::Anthropic, &ApiMode::Responses);
        assert!(tools.is_empty());
    }

    #[test]
    fn resolve_filters_unsupported() {
        let resolved = ToolRegistry::resolve_tools(
            &["web_search".to_string()],
            &Provider::Openai,
            &ApiMode::Completions,
        );
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_filters_unknown() {
        let resolved = ToolRegistry::resolve_tools(
            &["unknown_tool".to_string()],
            &Provider::Openai,
            &ApiMode::Responses,
        );
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_passes_valid() {
        let resolved = ToolRegistry::resolve_tools(
            &["web_search".to_string()],
            &Provider::Openai,
            &ApiMode::Responses,
        );
        assert_eq!(resolved, vec![BuiltInTool::WebSearch]);
    }

    #[test]
    fn web_search_payload() {
        let payload = ToolRegistry::to_request_payload(
            &BuiltInTool::WebSearch,
            &Provider::Openai,
            &ApiMode::Responses,
        );
        assert_eq!(payload, serde_json::json!({"type": "web_search_preview"}));
    }
}
