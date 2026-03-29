use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::models::ai::StreamEvent;
use crate::models::context::ContextItem;
use crate::models::history::HistoryEntryType;
use crate::models::message::{
    ContentPart, ConversationMessage, ImageUrlData, MessageContent, ProcessedMessage,
};
use crate::services::notification::NotificationLevel;
use crate::services::prompt_execution::PromptExecutionService;

#[derive(Clone, Serialize)]
struct ExecutionStartedPayload {
    execution_id: String,
}

#[derive(Clone, Serialize)]
struct ExecutionCompletedPayload {
    execution_id: String,
    success: bool,
    error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct ExecutionState {
    pub is_executing: bool,
    pub execution_id: Option<String>,
}

#[tauri::command]
pub async fn execute_prompt(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    prompt_id: String,
    input_override: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, model_id, model_display_name, prompt_name, input_content, messages) = {
        let mut state = state.lock().await;

        let execution_id = state
            .prompt_execution
            .start_execution()
            .map_err(|e| e.to_string())?;

        let prompt = PromptExecutionService::resolve_prompt(&state.config, &prompt_id)
            .map_err(|e| e.to_string())?;
        let prompt_name = prompt.name.clone();

        let model_id = PromptExecutionService::resolve_model(&state.config, None)
            .map_err(|e| e.to_string())?;

        let model_display_name = state
            .config
            .settings()
            .models
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| model_id.clone());

        let input_content = match &input_override {
            Some(text) => text.clone(),
            None => state.clipboard.get_text().unwrap_or_default(),
        };

        let messages = PromptExecutionService::prepare_messages(
            &prompt,
            &state.placeholder,
            &state.clipboard,
            &state.context,
            input_override.as_deref(),
        )
        .map_err(|e| {
            state.prompt_execution.finish_execution();
            e.to_string()
        })?;

        (execution_id, model_id, model_display_name, prompt_name, input_content, messages)
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = {
            let state = state.lock().await;
            state
                .ai
                .complete_stream(&model_id, messages)
                .await
                .map_err(|e| e.to_string())
        };

        match stream {
            Ok(mut stream) => {
                let mut full_text = String::new();
                let mut stream_error: Option<String> = None;

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
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
                        Err(e) => {
                            let msg = e.to_string();
                            let _ = on_event.send(StreamEvent::Error {
                                message: msg.clone(),
                            });
                            stream_error = Some(msg);
                            break;
                        }
                    }
                }

                match stream_error {
                    Some(err) => Err(err),
                    None => {
                        let _ = on_event.send(StreamEvent::Done {
                            full_text: full_text.clone(),
                        });
                        Ok(full_text)
                    }
                }
            }
            Err(e) => Err(e),
        }
    };

    let mut state = state.lock().await;

    match stream_result {
        Ok(full_text) => {
            let _ = state.clipboard.set_text(&full_text);

            let elapsed = start_time.elapsed();

            state.history.add_entry(
                input_content,
                HistoryEntryType::Text,
                Some(full_text),
                Some(prompt_id),
                true,
                None,
                false,
                Some(prompt_name.clone()),
            );

            let _ = state.notifications.notify(
                "prompt_execution_success",
                NotificationLevel::Success,
                "Prompt executed",
                Some(format!(
                    "{model_display_name} · {:.1}s · copied to clipboard",
                    elapsed.as_secs_f64()
                )),
                &state.config.settings().notifications,
            );

            state.prompt_execution.finish_execution();

            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: true,
                    error: None,
                },
            );

            Ok(())
        }
        Err(error) => {
            let _ = state.notifications.notify(
                "prompt_execution_error",
                NotificationLevel::Error,
                "Execution failed",
                Some(error.as_str()),
                &state.config.settings().notifications,
            );

            state.prompt_execution.finish_execution();

            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: false,
                    error: Some(error.clone()),
                },
            );

            Err(error)
        }
    }
}

#[tauri::command]
pub async fn get_execution_state(
    state: State<'_, Mutex<AppState>>,
) -> Result<ExecutionState, String> {
    let state = state.lock().await;
    Ok(ExecutionState {
        is_executing: state.prompt_execution.is_busy(),
        execution_id: state
            .prompt_execution
            .current_execution_id()
            .map(|s| s.to_string()),
    })
}

#[tauri::command]
pub async fn process_prompt_template(
    state: State<'_, Mutex<AppState>>,
    prompt_id: String,
    context_text: Option<String>,
) -> Result<Vec<ProcessedMessage>, String> {
    let mut state = state.lock().await;

    let prompt = PromptExecutionService::resolve_prompt(&state.config, &prompt_id)
        .map_err(|e| e.to_string())?;

    let clipboard_text = state.clipboard.get_text().unwrap_or_default();

    let saved_items = state.context.get_items();
    state.context.clear();
    if let Some(text) = &context_text {
        if !text.is_empty() {
            state.context.set_context(text.clone());
        }
    }

    let mut messages = Vec::new();
    for (i, msg) in prompt.messages.iter().enumerate() {
        let content_with_clipboard = msg.content.replace("{{clipboard}}", &clipboard_text);
        let processed_text = state
            .placeholder
            .process_content(&content_with_clipboard, None, &state.clipboard, &state.context)
            .unwrap_or(content_with_clipboard);

        let is_last = i == prompt.messages.len() - 1;
        let content = if is_last && state.context.has_images() {
            let mut parts = Vec::new();
            if !processed_text.trim().is_empty() {
                parts.push(ContentPart::Text {
                    text: processed_text,
                });
            }
            for item in &state.context.get_items() {
                if let ContextItem::Image { data, media_type } = item {
                    parts.push(ContentPart::ImageUrl {
                        image_url: ImageUrlData {
                            url: format!("data:{media_type};base64,{data}"),
                        },
                    });
                }
            }
            MessageContent::Parts(parts)
        } else {
            MessageContent::Text(processed_text)
        };

        messages.push(ProcessedMessage {
            role: msg.role.clone(),
            content,
        });
    }

    state.context.clear();
    for item in saved_items {
        match item {
            ContextItem::Text { content } => state.context.append_context(content),
            ContextItem::Image { data, media_type } => {
                state.context.append_context_image(data, media_type)
            }
        }
    }

    Ok(messages)
}

#[tauri::command]
pub async fn execute_conversation_turn(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    messages: Vec<ConversationMessage>,
    model_id: Option<String>,
    prompt_id: Option<String>,
    prompt_name: Option<String>,
    skip_clipboard_copy: bool,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, resolved_model_id, model_display_name, processed_messages) = {
        let mut state = state.lock().await;

        let execution_id = state
            .prompt_execution
            .start_execution()
            .map_err(|e| e.to_string())?;

        let resolved_model_id =
            PromptExecutionService::resolve_model(&state.config, model_id.as_deref())
                .map_err(|e| {
                    state.prompt_execution.finish_execution();
                    e.to_string()
                })?;

        let model_display_name = state
            .config
            .settings()
            .models
            .iter()
            .find(|m| m.id == resolved_model_id)
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| resolved_model_id.clone());

        let processed_messages = messages.into_iter().map(Into::into).collect();

        (execution_id, resolved_model_id, model_display_name, processed_messages)
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = {
            let state = state.lock().await;
            state
                .ai
                .complete_stream(&resolved_model_id, processed_messages)
                .await
                .map_err(|e| e.to_string())
        };

        match stream {
            Ok(mut stream) => {
                let mut full_text = String::new();
                let mut stream_error: Option<String> = None;

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
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
                        Err(e) => {
                            let msg = e.to_string();
                            let _ = on_event.send(StreamEvent::Error {
                                message: msg.clone(),
                            });
                            stream_error = Some(msg);
                            break;
                        }
                    }
                }

                match stream_error {
                    Some(err) => Err(err),
                    None => {
                        let _ = on_event.send(StreamEvent::Done {
                            full_text: full_text.clone(),
                        });
                        Ok(full_text)
                    }
                }
            }
            Err(e) => Err(e),
        }
    };

    let mut state = state.lock().await;

    match stream_result {
        Ok(full_text) => {
            if !skip_clipboard_copy {
                let _ = state.clipboard.set_text(&full_text);
            }

            let elapsed = start_time.elapsed();

            let input_summary = "conversation turn".to_string();
            state.history.add_entry(
                input_summary,
                HistoryEntryType::Text,
                Some(full_text),
                prompt_id,
                true,
                None,
                false,
                prompt_name.clone(),
            );

            let clipboard_note = if skip_clipboard_copy {
                ""
            } else {
                " · copied to clipboard"
            };
            let _ = state.notifications.notify(
                "prompt_execution_success",
                NotificationLevel::Success,
                "Prompt executed",
                Some(format!(
                    "{model_display_name} · {:.1}s{clipboard_note}",
                    elapsed.as_secs_f64()
                )),
                &state.config.settings().notifications,
            );

            state.prompt_execution.finish_execution();

            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: true,
                    error: None,
                },
            );

            Ok(())
        }
        Err(error) => {
            let _ = state.notifications.notify(
                "prompt_execution_error",
                NotificationLevel::Error,
                "Execution failed",
                Some(error.as_str()),
                &state.config.settings().notifications,
            );

            state.prompt_execution.finish_execution();

            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: false,
                    error: Some(error.clone()),
                },
            );

            Err(error)
        }
    }
}
