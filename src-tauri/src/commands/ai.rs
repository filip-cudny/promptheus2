use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::models::ai::StreamEvent;
use crate::models::message::ProcessedMessage;
use crate::services::ai::capabilities::{capabilities_for, ModelCapabilities};
use crate::models::settings::Provider;

#[tauri::command]
pub async fn get_model_capabilities(
    provider: Provider,
    model: String,
) -> crate::Result<ModelCapabilities> {
    Ok(capabilities_for(&provider, &model))
}

#[tauri::command]
pub async fn complete(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
    messages: Vec<ProcessedMessage>,
) -> crate::Result<String> {
    let ai = state.lock().await.ai.clone();
    Ok(ai.complete(&model_id, messages).await?)
}

#[tauri::command]
pub async fn complete_stream(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
    messages: Vec<ProcessedMessage>,
    on_event: Channel<StreamEvent>,
) -> crate::Result<()> {
    let ai = state.lock().await.ai.clone();
    let mut stream = ai
        .complete_stream(&model_id, messages, None, None, vec![])
        .await?;

    let mut full_text = String::new();
    let mut full_thinking = String::new();
    let mut prompt_tokens: Option<usize> = None;
    let mut completion_tokens: Option<usize> = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(usage) = chunk.usage {
                    prompt_tokens = Some(usage.prompt_tokens);
                    completion_tokens = Some(usage.completion_tokens);
                }
                let has_content = !chunk.delta.is_empty() || chunk.thinking_delta.is_some();
                if has_content {
                    full_text.clone_from(&chunk.accumulated);
                    if let Some(ref acc) = chunk.accumulated_thinking {
                        full_thinking.clone_from(acc);
                    }
                    if on_event
                        .send(StreamEvent::Chunk {
                            delta: chunk.delta,
                            accumulated: chunk.accumulated,
                            thinking_delta: chunk.thinking_delta,
                            accumulated_thinking: chunk.accumulated_thinking,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
            Err(e) => {
                let _ = on_event.send(StreamEvent::Error {
                    message: e.to_string(),
                });
                return Err(e.into());
            }
        }
    }

    let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking) };
    let _ = on_event.send(StreamEvent::Done {
        full_text,
        full_thinking: thinking,
        prompt_tokens,
        completion_tokens,
    });
    Ok(())
}
