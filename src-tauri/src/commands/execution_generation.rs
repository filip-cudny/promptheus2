use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::message::{MessageContent, ProcessedMessage};
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::execution::PromptExecutionService;

#[tauri::command]
pub async fn generate_conversation_title(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    user_message: String,
) -> crate::Result<String> {
    let (model_id, prompt, ai) = {
        let config = config.lock().await;
        let model_id = PromptExecutionService::resolve_title_generation_model(&config)?;
        let prompt = config.title_generation_prompt();
        (model_id, prompt, ai.lock().await.clone())
    };

    let messages = vec![
        ProcessedMessage {
            role: "system".to_string(),
            content: MessageContent::Text(prompt),
            tool_calls: None,
            tool_call_id: None,
        },
        ProcessedMessage {
            role: "user".to_string(),
            content: MessageContent::Text(user_message),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let title = ai.complete(&model_id, messages).await?;
    Ok(title.trim().to_string())
}
