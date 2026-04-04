use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::models::ai::StreamEvent;
use crate::models::message::ProcessedMessage;

#[tauri::command]
pub async fn complete(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
    messages: Vec<ProcessedMessage>,
) -> Result<String, String> {
    let ai = state.lock().await.ai.clone();
    ai.complete(&model_id, messages)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn complete_stream(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
    messages: Vec<ProcessedMessage>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let ai = state.lock().await.ai.clone();
    let mut stream = ai
        .complete_stream(&model_id, messages)
        .await
        .map_err(|e| e.to_string())?;

    let mut full_text = String::new();
    let mut prompt_tokens: Option<usize> = None;
    let mut completion_tokens: Option<usize> = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(usage) = chunk.usage {
                    prompt_tokens = Some(usage.prompt_tokens);
                    completion_tokens = Some(usage.completion_tokens);
                }
                if !chunk.delta.is_empty() {
                    full_text.clone_from(&chunk.accumulated);
                    if on_event
                        .send(StreamEvent::Chunk {
                            delta: chunk.delta,
                            accumulated: chunk.accumulated,
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
                return Err(e.to_string());
            }
        }
    }

    let _ = on_event.send(StreamEvent::Done {
        full_text,
        prompt_tokens,
        completion_tokens,
    });
    Ok(())
}
