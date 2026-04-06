use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::services::config::ConfigService;
use crate::models::ai::StreamEvent;
use crate::models::settings::ModelParameters;
use crate::models::history::SerializedConversationNode;
use crate::models::message::{
    ConversationNodeForExecution, ImageData, MessageContent,
    NodeUpdate, ProcessedMessage,
};
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

#[tauri::command]
pub async fn execute_skill(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    skill_name: String,
    input_override: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, model_id, model_display_name, skill_display_name, input_content, messages, ai, skill_param_overrides) = {
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

        let model_id = PromptExecutionService::resolve_model(&state.config, skill.model.as_deref())
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

        let system_prompt = build_system_prompt_base(&state.config, None, state.active_app(), &state.recent_apps_display());
        let messages = skill_execution::prepare_skill_messages(
            &system_prompt,
            &skill,
            &input_content,
            &state.context,
        );

        let ai = state.ai.clone();
        let skill_param_overrides = skill.parameters.as_ref().map(ModelParameters::from_map);

        (execution_id, model_id, model_display_name, skill_display_name, input_content, messages, ai, skill_param_overrides)
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = ai
            .complete_stream(&model_id, messages, skill_param_overrides)
            .await
            .map_err(|e| e.to_string());

        match stream {
            Ok(mut stream) => {
                let mut full_text = String::new();
                let mut full_thinking = String::new();
                let mut stream_error: Option<String> = None;
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
                        let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking) };
                        let _ = on_event.send(StreamEvent::Done {
                            full_text: full_text.clone(),
                            full_thinking: thinking,
                            prompt_tokens,
                            completion_tokens,
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

            let user_display_text = format!("/{skill_name} {input_content}");

            let user_node = SerializedConversationNode {
                node_id: user_node_id.clone(),
                parent_id: None,
                role: "user".to_string(),
                content: user_display_text,
                timestamp: now.clone(),
                children: vec![assistant_node_id.clone()],
                updates: vec![],
                prompt_tokens: None,
                completion_tokens: None,
                thinking: None,
                error: None,
                cancelled: false,
            };

            let assistant_node = SerializedConversationNode {
                node_id: assistant_node_id.clone(),
                parent_id: Some(user_node_id.clone()),
                role: "assistant".to_string(),
                content: full_text.clone(),
                timestamp: now,
                children: vec![],
                updates: vec![],
                prompt_tokens: None,
                completion_tokens: None,
                thinking: None,
                error: None,
                cancelled: false,
            };

            state.history.add_conversation_entry(
                String::new(),
                Some(skill_name),
                Some(skill_display_name),
                true,
                None,
                vec![user_node, assistant_node],
                Some(user_node_id.clone()),
                vec![user_node_id, assistant_node_id],
                true,
                None,
                vec![],
                None,
                None,
            );
            let _ = app.emit("history-changed", ());

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
pub async fn generate_conversation_title(
    state: State<'_, Mutex<AppState>>,
    user_message: String,
) -> Result<String, String> {
    let (model_id, prompt, ai) = {
        let state = state.lock().await;
        let settings = state.config.settings();

        let model_id = if !settings.conversation_title_model.is_empty() {
            settings.conversation_title_model.clone()
        } else if let Some(ref default) = settings.default_model {
            default.clone()
        } else {
            return Err("No model configured for title generation".to_string());
        };

        (model_id, settings.conversation_title_prompt.clone(), state.ai.clone())
    };

    let messages = vec![
        ProcessedMessage {
            role: "system".to_string(),
            content: MessageContent::Text(prompt),
        },
        ProcessedMessage {
            role: "user".to_string(),
            content: MessageContent::Text(user_message),
        },
    ];

    let title = ai
        .complete(&model_id, messages)
        .await
        .map_err(|e| e.to_string())?;

    Ok(title.trim().to_string())
}

pub fn resolve_environment_section_template(config: &ConfigService, active_app: &str, recent_apps: &str) -> String {
    let template = config.environment_section_template();
    if template.is_empty() {
        return String::new();
    }

    let now = chrono::Local::now();
    template
        .replace("{{date}}", &now.format("%Y-%m-%d").to_string())
        .replace("{{time}}", &now.format("%H:%M").to_string())
        .replace("{{timezone}}", &now.format("%Z").to_string())
        .replace("{{os}}", std::env::consts::OS)
        .replace("{{active_app}}", active_app)
        .replace("{{recent_apps}}", recent_apps)
}

pub fn build_system_prompt_base(config: &ConfigService, resolved_environment_section: Option<&str>, active_app: &str, recent_apps: &str) -> String {
    let system_prompt = &config.settings().system_prompt;
    let input_format_guide = config.input_format_guide();
    let about_me = config.about_me();

    let environment_section = resolved_environment_section
        .map(|s| s.to_string())
        .unwrap_or_else(|| resolve_environment_section_template(config, active_app, recent_apps));

    if environment_section.is_empty() {
        format!("{system_prompt}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}")
    } else {
        format!("{system_prompt}\n\n---\n\n{environment_section}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}")
    }
}

#[tauri::command]
pub async fn resolve_environment_section(
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let state = state.lock().await;
    Ok(resolve_environment_section_template(&state.config, state.active_app(), &state.recent_apps_display()))
}

#[tauri::command]
pub async fn release_conversation_context(
    state: State<'_, Mutex<AppState>>,
    tab_id: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.conversation_context.remove(&tab_id);
    Ok(())
}

#[tauri::command]
pub async fn seed_conversation_context(
    state: State<'_, Mutex<AppState>>,
    tab_id: String,
    resolved_environment_section: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    if !state.conversation_context.has(&tab_id) {
        state.conversation_context.insert(tab_id, resolved_environment_section);
    }
    Ok(())
}

#[derive(Clone, Serialize)]
pub struct ResolveSkillInputResult {
    pub resolved_text: String,
    pub had_skills: bool,
}

#[tauri::command]
pub async fn resolve_skill_input(
    state: State<'_, Mutex<AppState>>,
    text: String,
) -> Result<ResolveSkillInputResult, String> {
    let state = state.lock().await;
    let result = skill_execution::resolve_skill_input(&state.skill_service, &text);
    Ok(ResolveSkillInputResult {
        resolved_text: result.resolved_text,
        had_skills: result.had_skills,
    })
}

#[tauri::command]
pub async fn respond_to_tool_call(
    state: State<'_, Mutex<AppState>>,
    tool_call_id: String,
    approved: bool,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.tool_confirmation.respond(&tool_call_id, approved)
}

#[tauri::command]
pub async fn retry_tool_call(
    state: State<'_, Mutex<AppState>>,
    tool_call_id: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.tool_confirmation.respond(&tool_call_id, true)
}

#[tauri::command]
pub async fn execute_conversation_from_tree(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    mut nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    skill_id: Option<String>,
    skill_name: Option<String>,
    model_id: Option<String>,
    reasoning_effort: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, resolved_model_id, model_display_name, all_messages, ai, updates_for_event, param_overrides) = {
        let mut state = state.lock().await;

        let execution_id = state
            .prompt_execution
            .start_execution()
            .map_err(|e| e.to_string())?;

        let resolved_model_id =
            PromptExecutionService::resolve_model(&state.config, model_id.as_deref()).map_err(|e| {
                state.prompt_execution.finish_execution();
                e.to_string()
            })?;

        let param_overrides = reasoning_effort.map(|effort| {
            ModelParameters {
                reasoning_effort: Some(effort),
                ..Default::default()
            }
        });

        let model_display_name = state
            .config
            .settings()
            .models
            .iter()
            .find(|m| m.id == resolved_model_id)
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| resolved_model_id.clone());

        let active_app = state.active_app().to_string();
        let recent_apps = state.recent_apps_display();

        if !state.conversation_context.has(&tab_id) {
            let resolved =
                resolve_environment_section_template(&state.config, &active_app, &recent_apps);
            state.conversation_context.insert(tab_id.clone(), resolved);
        }
        let resolved_environment_section = state.conversation_context.get(&tab_id).unwrap().to_string();

        let system_content = build_system_prompt_base(
            &state.config,
            Some(&resolved_environment_section),
            &active_app,
            &recent_apps,
        );

        let mut node_updates: Vec<NodeUpdate> = Vec::new();

        if let Some(env_update) = state.conversation_context.environment_update_if_stale(
            &tab_id,
            &active_app,
            &recent_apps,
        ) {
            node_updates.push(env_update);
        }

        let context_str = context_text.as_deref().unwrap_or("");
        let image_data_len: usize = context_images.iter().map(|i| i.data.len()).sum();
        if let Some(ctx_update) = state.conversation_context.context_update_if_changed(
            &tab_id,
            context_str,
            context_images.len(),
            image_data_len,
        ) {
            node_updates.push(ctx_update);
        }

        let updates_for_event = if !node_updates.is_empty() {
            if let Some(last_user) = nodes.iter_mut().rev().find(|n| n.role == "user") {
                let node_id = last_user.node_id.clone();
                last_user.updates = node_updates.clone();
                Some((node_id, node_updates))
            } else {
                None
            }
        } else {
            None
        };

        let system_message = ProcessedMessage {
            role: "system".to_string(),
            content: MessageContent::Text(system_content),
        };

        let tree_messages =
            skill_execution::build_messages_from_tree(&nodes, &context_images);

        let mut all_messages = vec![system_message];
        all_messages.extend(tree_messages);

        let ai = state.ai.clone();

        (
            execution_id,
            resolved_model_id,
            model_display_name,
            all_messages,
            ai,
            updates_for_event,
            param_overrides,
        )
    };

    if let Some((node_id, updates)) = updates_for_event {
        let _ = on_event.send(StreamEvent::NodeUpdates { node_id, updates });
    }

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = ai
            .complete_stream(&resolved_model_id, all_messages, param_overrides)
            .await
            .map_err(|e| e.to_string());

        match stream {
            Ok(mut stream) => {
                let mut full_text = String::new();
                let mut full_thinking = String::new();
                let mut stream_error: Option<String> = None;
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
                        let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking) };
                        let _ = on_event.send(StreamEvent::Done {
                            full_text: full_text.clone(),
                            full_thinking: thinking,
                            prompt_tokens,
                            completion_tokens,
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
        Ok(_full_text) => {
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

