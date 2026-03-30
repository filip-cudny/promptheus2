use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::models::ai::StreamEvent;
use crate::models::context::ContextItem;
use crate::models::history::{
    HistoryEntryType, SerializedConversationNode, SerializedConversationTurn,
};
use crate::models::message::{ConversationMessage, ProcessedMessage};
use crate::services::notification::NotificationLevel;
use crate::services::prompt_execution::PromptExecutionService;
use crate::services::skill_execution;

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

            if !skip_clipboard_copy {
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
            }

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
pub async fn execute_skill(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    skill_name: String,
    input_override: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, model_id, model_display_name, skill_display_name, input_content, messages) = {
        let mut state = state.lock().await;

        let execution_id = state
            .prompt_execution
            .start_execution()
            .map_err(|e| e.to_string())?;

        let skill = skill_execution::resolve_skill_or_err(&state.skill_service, &skill_name)
            .map_err(|e| {
                state.prompt_execution.finish_execution();
                e.to_string()
            })?;
        let skill_display_name = skill.display_name.clone();

        let model_id = PromptExecutionService::resolve_model(&state.config, None)
            .map_err(|e| {
                state.prompt_execution.finish_execution();
                e.to_string()
            })?;

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
            None => state.clipboard.get_text().map_err(|e| {
                state.prompt_execution.finish_execution();
                e.to_string()
            })?,
        };

        let system_prompt = &state.config.settings().system_prompt;
        let messages = skill_execution::prepare_skill_messages(
            system_prompt,
            &skill,
            &input_content,
            &state.context,
        );

        (execution_id, model_id, model_display_name, skill_display_name, input_content, messages)
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

            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            let user_node_id = format!("skill-user-{}", uuid::Uuid::new_v4());
            let assistant_node_id = format!("skill-asst-{}", uuid::Uuid::new_v4());

            let turn = SerializedConversationTurn {
                turn_number: 1,
                message_text: input_content,
                message_image_paths: vec![],
                output_text: Some(full_text.clone()),
                is_complete: true,
                output_versions: vec![full_text],
                current_version_index: 0,
            };

            let user_node = SerializedConversationNode {
                node_id: user_node_id.clone(),
                parent_id: None,
                role: "user".to_string(),
                content: turn.message_text.clone(),
                image_paths: vec![],
                timestamp: now.clone(),
                children: vec![assistant_node_id.clone()],
            };

            let assistant_node = SerializedConversationNode {
                node_id: assistant_node_id.clone(),
                parent_id: Some(user_node_id.clone()),
                role: "assistant".to_string(),
                content: turn.output_text.clone().unwrap_or_default(),
                image_paths: vec![],
                timestamp: now,
                children: vec![],
            };

            state.history.add_conversation_entry(
                &[turn],
                String::new(),
                vec![],
                Some(skill_name),
                Some(skill_display_name),
                true,
                None,
                vec![user_node, assistant_node],
                Some(user_node_id.clone()),
                vec![user_node_id, assistant_node_id],
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
pub async fn get_system_prompt(
    state: State<'_, Mutex<AppState>>,
    context_text: Option<String>,
) -> Result<Vec<ProcessedMessage>, String> {
    let state = state.lock().await;

    let system_prompt = state.config.settings().system_prompt.clone();
    let system_content = match &context_text {
        Some(text) if !text.is_empty() => {
            format!("{system_prompt}\n\n<context>\n{text}\n</context>")
        }
        _ => system_prompt,
    };

    Ok(vec![ProcessedMessage {
        role: "system".to_string(),
        content: crate::models::message::MessageContent::Text(system_content),
    }])
}

#[tauri::command]
pub async fn process_skill_template(
    state: State<'_, Mutex<AppState>>,
    skill_name: String,
    context_text: Option<String>,
) -> Result<Vec<ProcessedMessage>, String> {
    let mut state = state.lock().await;

    let skill = skill_execution::resolve_skill_or_err(&state.skill_service, &skill_name)
        .map_err(|e| e.to_string())?;

    let clipboard_text = state.clipboard.get_text().unwrap_or_default();

    let saved_items = state.context.get_items();
    state.context.clear();
    if let Some(text) = &context_text {
        if !text.is_empty() {
            state.context.set_context(text.clone());
        }
    }

    let system_prompt = state.config.settings().system_prompt.clone();
    let messages = skill_execution::prepare_skill_messages(
        &system_prompt,
        &skill,
        &clipboard_text,
        &state.context,
    );

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
