use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::message::{
    ContentPart, ConversationNodeForExecution, ImageData, MessageContent, ProcessedMessage,
};
use crate::models::settings::Provider;
use crate::services::ai::tools::build_tool_payloads;
use crate::services::config::ConfigService;
use crate::services::conversation_context::ConversationContextCache;
use crate::services::execution::conversation_executor::{
    build_conversation_messages, ConversationExecutor,
};
use crate::services::execution::{resolve_environment_section_template, PromptExecutionService};
use crate::services::mcp::McpRegistry;
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::recent_apps::RecentAppsState;
use crate::services::skill::SkillService;
use crate::services::skill_message;
use crate::services::sqlite_history::SqliteHistoryService;
use crate::services::tokenizer;
use crate::Error;

fn extract_text_from_messages(messages: &[ProcessedMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        match &msg.content {
            MessageContent::Text(text) => parts.push(text.as_str()),
            MessageContent::Parts(content_parts) => {
                for part in content_parts {
                    if let ContentPart::Text { text } = part {
                        parts.push(text.as_str());
                    }
                }
            }
        }
    }
    parts.join("\n")
}

fn count_images_in_messages(messages: &[ProcessedMessage]) -> usize {
    let mut count = 0;
    for msg in messages {
        if let MessageContent::Parts(parts) = &msg.content {
            for part in parts {
                if matches!(part, ContentPart::ImageUrl { .. }) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn image_tokens_for_provider(provider: &Provider) -> usize {
    match provider {
        Provider::Openai | Provider::ElevenLabs => 765,
        Provider::Anthropic => 1334,
        Provider::Gemini => 258,
    }
}

fn count_request_tokens(
    messages: &[ProcessedMessage],
    tool_payloads: &[serde_json::Value],
    provider: &Provider,
) -> usize {
    let mut text = extract_text_from_messages(messages);
    if !tool_payloads.is_empty() {
        text.push('\n');
        text.push_str(&serde_json::Value::from(tool_payloads.to_vec()).to_string());
    }
    tokenizer::count_tokens(&text)
        + count_images_in_messages(messages) * image_tokens_for_provider(provider)
}

#[tauri::command]
pub async fn count_tokens(text: String, image_count: Option<usize>) -> crate::Result<usize> {
    let image_tokens = image_count.unwrap_or(0) * image_tokens_for_provider(&Provider::Openai);
    let text_tokens = tokio::task::spawn_blocking(move || tokenizer::count_tokens(&text))
        .await
        .map_err(|e| Error::Other(e.to_string()))?;
    Ok(text_tokens + image_tokens)
}

#[tauri::command]
pub async fn count_conversation_tokens(
    mut nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    tool_names: Vec<String>,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    conversation_context: State<'_, Arc<Mutex<ConversationContextCache>>>,
    recent_apps: State<'_, Arc<RecentAppsState>>,
    mcp: State<'_, Arc<McpRegistry>>,
) -> crate::Result<usize> {
    {
        let skill_service = skill_service.lock().await;
        for node in nodes.iter_mut() {
            if node.role == "user"
                && node.applied_skills.is_empty()
                && skill_message::has_skill_references(&node.content)
            {
                let result = skill_message::resolve_skill_input(&skill_service, &node.content);
                if result.had_skills {
                    node.applied_skills = result.applied_skills;
                }
            }
        }
    }

    let active_app = recent_apps.active().await;
    let recent_apps_display = recent_apps.display().await;
    let (builtin_names, mcp_tools) = ConversationExecutor::resolve_tools(&mcp, Some(&tool_names));

    let (messages, tool_payloads, provider) = {
        let config_guard = config.lock().await;
        let history_guard = history.lock().await;
        let cache = conversation_context.lock().await;

        let model_config = PromptExecutionService::resolve_model(&config_guard, None)
            .ok()
            .and_then(|id| config_guard.settings().models.iter().find(|m| m.id == id).cloned());
        let provider = model_config
            .as_ref()
            .and_then(|m| m.provider.clone())
            .unwrap_or_default();
        let api_mode = model_config
            .as_ref()
            .and_then(|m| m.api_mode.clone())
            .unwrap_or_default();

        let resolved_env = cache.get(&tab_id).map(str::to_string).unwrap_or_else(|| {
            let placeholder_ctx =
                PlaceholderContext::with_apps(active_app.clone(), recent_apps_display.clone());
            resolve_environment_section_template(&config_guard, &placeholder_ctx)
        });

        let built = build_conversation_messages(
            &config_guard,
            &history_guard,
            &cache,
            &active_app,
            &recent_apps_display,
            &resolved_env,
            &tab_id,
            &mut nodes,
            context_text.as_deref(),
            &context_images,
        )?;
        let tool_payloads =
            build_tool_payloads(builtin_names.as_deref(), &mcp_tools, &provider, &api_mode);
        (built.messages, tool_payloads, provider)
    };

    tokio::task::spawn_blocking(move || count_request_tokens(&messages, &tool_payloads, &provider))
        .await
        .map_err(|e| Error::Other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::NodeUpdate;
    use crate::models::settings::ApiMode;
    use crate::services::database::Database;
    use serde_json::json;
    use tempfile::TempDir;

    const TAB: &str = "tab-1";

    fn user_node(content: &str) -> ConversationNodeForExecution {
        ConversationNodeForExecution {
            node_id: "node-1".to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            images: vec![],
            text_attachments: vec![],
            updates: vec![],
            applied_skills: vec![],
        }
    }

    fn mcp_tool() -> rmcp::model::Tool {
        let mut schema = serde_json::Map::new();
        schema.insert("type".into(), json!("object"));
        let mut tool = rmcp::model::Tool::default();
        tool.name = "get_weather".into();
        tool.description = Some("Get weather for a location".into());
        tool.input_schema = std::sync::Arc::new(schema);
        tool
    }

    fn message_texts(messages: &[ProcessedMessage]) -> Vec<&str> {
        messages
            .iter()
            .map(|m| match &m.content {
                MessageContent::Text(text) => text.as_str(),
                MessageContent::Parts(_) => unreachable!("test messages carry no images"),
            })
            .collect()
    }

    #[test]
    fn count_includes_updates_and_mcp_tools_of_the_send_path_without_marking_them_sent() {
        let dir = TempDir::new().unwrap();
        let config = ConfigService::load(dir.path()).unwrap();
        let history = SqliteHistoryService::new(Database::open_in_memory().unwrap(), 0);
        let provider = Provider::Openai;
        let api_mode = ApiMode::Responses;

        let mut stale = ConversationContextCache::new();
        stale.insert_aged(TAB.into(), "env".into(), chrono::Duration::hours(1));
        stale.mark_context_sent(TAB, "old context", 0, 0);
        let mut fresh = ConversationContextCache::new();
        fresh.insert(TAB.into(), "env".into());
        fresh.mark_context_sent(TAB, "new context", 0, 0);

        let build = |cache: &ConversationContextCache| {
            let mut nodes = vec![user_node("hello there")];
            build_conversation_messages(
                &config,
                &history,
                cache,
                "Safari",
                "Safari, Code",
                "env",
                TAB,
                &mut nodes,
                Some("new context"),
                &[],
            )
            .unwrap()
        };

        let with_updates = build(&stale);
        let without_updates = build(&fresh);
        let tool_payloads = build_tool_payloads(Some(&[]), &[mcp_tool()], &provider, &api_mode);

        let (node_id, updates) = with_updates.updates_for_event.clone().unwrap();
        assert_eq!(node_id, "node-1");
        assert!(matches!(updates[0], NodeUpdate::Environment { .. }));
        assert!(matches!(updates[1], NodeUpdate::Context { .. }));
        assert!(without_updates.updates_for_event.is_none());

        let mut sent_text = message_texts(&with_updates.messages).join("\n");
        sent_text.push('\n');
        sent_text.push_str(&serde_json::to_string(&tool_payloads).unwrap());
        let counted = count_request_tokens(&with_updates.messages, &tool_payloads, &provider);
        assert_eq!(counted, tokenizer::count_tokens(&sent_text));

        let baseline = count_request_tokens(&without_updates.messages, &[], &provider);
        assert!(counted > baseline);

        assert_eq!(
            stale.pending_updates(TAB, "Safari", "Safari, Code", "new context", 0, 0),
            updates
        );
    }
}
