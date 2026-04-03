use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::commands::settings::AppState;
use crate::services::config::ConfigService;
use crate::models::ai::StreamEvent;
use crate::models::history::{
    SerializedConversationNode, SerializedConversationTurn,
};
use crate::models::message::{
    ContentPart, ConversationNodeForExecution, ImageData, MessageContent,
    ProcessedMessage,
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

    let (execution_id, model_id, model_display_name, skill_display_name, input_content, messages, ai) = {
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

        let system_prompt = build_system_prompt_base(&state.config, None, state.active_app(), &state.recent_apps_display());
        let messages = skill_execution::prepare_skill_messages(
            &system_prompt,
            &skill,
            &input_content,
            &state.context,
        );

        let ai = state.ai.clone();

        (execution_id, model_id, model_display_name, skill_display_name, input_content, messages, ai)
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = ai
            .complete_stream(&model_id, messages)
            .await
            .map_err(|e| e.to_string());

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

            let user_display_text = format!("/{skill_name} {input_content}");

            let turn = SerializedConversationTurn {
                turn_number: 1,
                message_text: user_display_text.clone(),
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
                content: user_display_text,
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
                true,
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

fn resolve_context_section_template(config: &ConfigService, active_app: &str, recent_apps: &str) -> String {
    let template = config.context_section_template();
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

fn build_system_prompt_base(config: &ConfigService, resolved_context_section: Option<&str>, active_app: &str, recent_apps: &str) -> String {
    let system_prompt = &config.settings().system_prompt;
    let input_format_guide = config.input_format_guide();
    let about_me = config.about_me();

    let context_section = resolved_context_section
        .map(|s| s.to_string())
        .unwrap_or_else(|| resolve_context_section_template(config, active_app, recent_apps));

    if context_section.is_empty() {
        format!("{system_prompt}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}")
    } else {
        format!("{system_prompt}\n\n---\n\n{context_section}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}")
    }
}

#[tauri::command]
pub async fn resolve_context_section(
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let state = state.lock().await;
    Ok(resolve_context_section_template(&state.config, state.active_app(), &state.recent_apps_display()))
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
    resolved_context_section: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    if !state.conversation_context.has(&tab_id) {
        state.conversation_context.insert(tab_id, resolved_context_section);
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
pub async fn execute_conversation_from_tree(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    prompt_id: Option<String>,
    prompt_name: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let start_time = Instant::now();

    let (execution_id, resolved_model_id, model_display_name, all_messages, ai) = {
        let mut state = state.lock().await;

        let execution_id = state
            .prompt_execution
            .start_execution()
            .map_err(|e| e.to_string())?;

        let resolved_model_id =
            PromptExecutionService::resolve_model(&state.config, None).map_err(|e| {
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

        let active_app = state.active_app().to_string();
        let recent_apps = state.recent_apps_display();

        if !state.conversation_context.has(&tab_id) {
            let resolved =
                resolve_context_section_template(&state.config, &active_app, &recent_apps);
            state.conversation_context.insert(tab_id.clone(), resolved);
        }
        let resolved_context_section = state.conversation_context.get(&tab_id).unwrap().to_string();

        let base_prompt = build_system_prompt_base(
            &state.config,
            Some(&resolved_context_section),
            &active_app,
            &recent_apps,
        );
        let system_content = match &context_text {
            Some(text) if !text.is_empty() => {
                format!("{base_prompt}\n\n<context>\n{text}\n</context>")
            }
            _ => base_prompt,
        };

        let time_update = state.conversation_context.time_update_if_stale(&tab_id);

        let system_message = ProcessedMessage {
            role: "system".to_string(),
            content: MessageContent::Text(system_content),
        };

        let mut tree_messages =
            skill_execution::build_messages_from_tree(&nodes, &context_images);

        if let Some(update) = time_update {
            prepend_to_last_user_message(&mut tree_messages, &update);
        }

        let mut all_messages = vec![system_message];
        all_messages.append(&mut tree_messages);

        let ai = state.ai.clone();

        (
            execution_id,
            resolved_model_id,
            model_display_name,
            all_messages,
            ai,
        )
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
        },
    );

    let stream_result = {
        let stream = ai
            .complete_stream(&resolved_model_id, all_messages)
            .await
            .map_err(|e| e.to_string());

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

fn prepend_to_last_user_message(messages: &mut [ProcessedMessage], prefix: &str) {
    for msg in messages.iter_mut().rev() {
        if msg.role != "user" {
            continue;
        }
        match &mut msg.content {
            MessageContent::Text(text) => {
                *text = format!("{prefix}\n\n{text}");
            }
            MessageContent::Parts(parts) => {
                if let Some(last_text_idx) = parts.iter().rposition(|p| matches!(p, ContentPart::Text { .. })) {
                    if let ContentPart::Text { text } = &mut parts[last_text_idx] {
                        *text = format!("{prefix}\n\n{text}");
                    }
                }
            }
        }
        break;
    }
}
