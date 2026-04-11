use std::collections::HashMap;

use tauri::State;
use tokio::sync::Mutex;

use super::settings::AppState;
use crate::models::message::{
    ConversationNodeForExecution, ImageData, MessageContent, ProcessedMessage,
};
use crate::models::settings::Provider;
use crate::services::ai::tools::ToolRegistry;
use crate::services::skill_message;
use crate::services::tokenizer;

use super::execution::{build_system_prompt_base, resolve_environment_section_template};

fn parse_provider(s: &str) -> Provider {
    match s {
        "anthropic" => Provider::Anthropic,
        "gemini" => Provider::Gemini,
        _ => Provider::Openai,
    }
}

fn extract_text_from_messages(messages: &[ProcessedMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        match &msg.content {
            MessageContent::Text(text) => parts.push(text.as_str()),
            MessageContent::Parts(content_parts) => {
                for part in content_parts {
                    if let crate::models::message::ContentPart::Text { text } = part {
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
                if matches!(part, crate::models::message::ContentPart::ImageUrl { .. }) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn image_tokens_for_provider(provider: &Provider) -> usize {
    match provider {
        Provider::Openai => 765,
        Provider::Anthropic => 1334,
        Provider::Gemini => 258,
    }
}

#[tauri::command]
pub async fn count_tokens(
    text: String,
    provider: String,
    _state: State<'_, Mutex<AppState>>,
) -> Result<usize, String> {
    let provider = parse_provider(&provider);
    Ok(tokio::task::spawn_blocking(move || {
        tokenizer::count_tokens(&text, &provider)
    })
    .await
    .map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn get_skill_token_counts(
    provider: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<HashMap<String, usize>, String> {
    let skills: Vec<(String, String)> = {
        let s = state.lock().await;
        s.skill_service
            .list_skills()
            .iter()
            .map(|skill| (skill.name.clone(), skill.body.clone()))
            .collect()
    };

    let provider = parse_provider(&provider);
    tokio::task::spawn_blocking(move || {
        skills
            .into_iter()
            .map(|(name, body)| {
                let count = tokenizer::count_tokens(&body, &provider);
                (name, count)
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn count_conversation_tokens(
    nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    tool_names: Vec<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<usize, String> {
    let (all_text, image_count, provider) = {
        let state = state.lock().await;

        let nodes: Vec<ConversationNodeForExecution> = nodes
            .into_iter()
            .map(|mut node| {
                if node.role == "user" {
                    let result =
                        skill_message::resolve_skill_input(&state.skill_service, &node.content);
                    node.content = result.resolved_text;
                }
                node
            })
            .collect();

        let model_id = crate::services::execution::PromptExecutionService::resolve_model(
            &state.config,
            None,
        )
        .ok();

        let model_config = model_id.as_ref().and_then(|id| {
            state
                .config
                .settings()
                .models
                .iter()
                .find(|m| &m.id == id)
                .cloned()
        });

        let provider = model_config
            .as_ref()
            .map(|m| m.provider.clone())
            .unwrap_or_default();

        let api_mode = model_config
            .as_ref()
            .and_then(|m| m.api_mode.clone())
            .unwrap_or_default();

        let tools_text = if !tool_names.is_empty() {
            let resolved = ToolRegistry::resolve_tools(&tool_names, &provider, &api_mode);
            let payloads: Vec<serde_json::Value> = resolved
                .iter()
                .map(|t| ToolRegistry::to_request_payload(t, &provider, &api_mode))
                .collect();
            serde_json::to_string(&payloads).unwrap_or_default()
        } else {
            String::new()
        };

        let active_app = state.active_app().to_string();
        let recent_apps = state.recent_apps_display();

        let resolved_env = state
            .conversation_context
            .get(&tab_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                resolve_environment_section_template(&state.config, &active_app, &recent_apps)
            });

        let system_content =
            build_system_prompt_base(&state.config, Some(&resolved_env), &active_app, &recent_apps);

        let system_message = ProcessedMessage {
            role: "system".to_string(),
            content: MessageContent::Text(system_content),
            tool_calls: None,
            tool_call_id: None,
        };

        let tree_messages = skill_message::build_messages_from_tree(&nodes, &context_images);

        let mut all_messages = vec![system_message];
        all_messages.extend(tree_messages);

        let mut all_text = extract_text_from_messages(&all_messages);
        if !tools_text.is_empty() {
            all_text.push('\n');
            all_text.push_str(&tools_text);
        }
        let image_count = count_images_in_messages(&all_messages);

        (all_text, image_count, provider)
    };

    let image_tokens = image_count * image_tokens_for_provider(&provider);

    let text_tokens = tokio::task::spawn_blocking(move || {
        tokenizer::count_tokens(&all_text, &provider)
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(text_tokens + image_tokens)
}
