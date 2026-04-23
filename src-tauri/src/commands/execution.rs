use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};

use crate::commands::settings::AppState;
use crate::services::config::ConfigService;
use crate::models::ai::{StreamEvent, ToolCall, ToolCallStatus, ToolCallType};
use crate::models::settings::ModelParameters;
use crate::services::ai::AiError;
use crate::services::ai::provider::{StreamChunk, ToolCallEvent};
use crate::models::history::SerializedConversationNode;
use crate::models::message::{
    ConversationNodeForExecution, ImageData, MessageContent,
    NodeUpdate, ProcessedMessage, ToolCallFunction, ToolCallPayload,
};
use crate::services::notification::NotificationLevel;
use crate::services::execution::{ExecutionSnapshot, LiveExecution, PromptExecutionService};
use crate::services::skill_message;

fn tool_display_name(tool_name: &str) -> &str {
    match tool_name {
        "web_search" => "Web Search",
        other => other,
    }
}

fn merge_optional_parameters(
    base: Option<ModelParameters>,
    overrides: Option<ModelParameters>,
) -> Option<ModelParameters> {
    match (base, overrides) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(b), Some(o)) => {
            let mut extra = b.extra.clone();
            extra.extend(o.extra.clone());
            Some(ModelParameters {
                temperature: o.temperature.or(b.temperature),
                max_tokens: o.max_tokens.or(b.max_tokens),
                top_p: o.top_p.or(b.top_p),
                frequency_penalty: o.frequency_penalty.or(b.frequency_penalty),
                presence_penalty: o.presence_penalty.or(b.presence_penalty),
                reasoning_effort: o.reasoning_effort.or(b.reasoning_effort),
                extra,
            })
        }
    }
}

fn surface_effort_override(base: Option<String>, override_effort: Option<String>) -> Option<ModelParameters> {
    let effort = override_effort.or(base);
    effort.map(|e| ModelParameters {
        reasoning_effort: Some(e),
        ..Default::default()
    })
}

fn tool_type_from_name(tool_name: &str) -> ToolCallType {
    match tool_name {
        "web_search" => ToolCallType::WebSearch,
        _ => ToolCallType::Custom,
    }
}

const MAX_TOOL_LOOP_ITERATIONS: usize = 10;

struct PendingToolCall {
    tool_call_id: String,
    tool_name: String,
    arguments: serde_json::Value,
}

struct StreamResult {
    full_text: String,
    full_thinking: Option<String>,
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
    pending_tool_calls: Vec<PendingToolCall>,
}

async fn run_stream_loop(
    mut stream: Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>,
    live: Arc<TokioMutex<LiveExecution>>,
    cancel_rx: Option<watch::Receiver<bool>>,
    text_prefix: &str,
) -> Result<StreamResult, String> {
    let mut full_text = String::new();
    let mut full_thinking = String::new();
    let mut prompt_tokens: Option<usize> = None;
    let mut completion_tokens: Option<usize> = None;
    let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
    let mut cancel_rx = cancel_rx;

    loop {
        let chunk_result: Option<Result<StreamChunk, AiError>> = if let Some(ref mut rx) = cancel_rx {
            tokio::select! {
                biased;
                result = rx.changed() => {
                    if result.is_ok() && *rx.borrow() {
                        let mut live = live.lock().await;
                        live.snapshot.finished = true;
                        live.snapshot.error = Some("Cancelled".to_string());
                        if let Some(ref ch) = live.channel {
                            let _ = ch.send(StreamEvent::Error { message: "Cancelled".to_string() });
                        }
                        return Err("Cancelled".to_string());
                    }
                    continue;
                }
                next = stream.next() => next,
            }
        } else {
            stream.next().await
        };

        let Some(result) = chunk_result else { break };

        match result {
            Ok(chunk) => {
                if let Some(usage) = chunk.usage {
                    prompt_tokens = Some(usage.prompt_tokens);
                    completion_tokens = Some(usage.completion_tokens);
                }

                if let Some(tool_event) = chunk.tool_call_event {
                    let prefixed_accumulated = format!("{text_prefix}{}", chunk.accumulated);
                    full_text.clone_from(&prefixed_accumulated);
                    let mut live = live.lock().await;
                    live.snapshot.accumulated_text.clone_from(&prefixed_accumulated);

                    match tool_event {
                        ToolCallEvent::Started { tool_call_id, tool_name } => {
                            let display_name = tool_display_name(&tool_name).to_string();
                            let tool_type = tool_type_from_name(&tool_name);
                            let tc = ToolCall {
                                tool_call_id,
                                tool_name,
                                tool_display_name: display_name,
                                tool_type,
                                arguments: serde_json::json!({}),
                                result: None,
                                error: None,
                                status: ToolCallStatus::InProgress,
                                requires_confirmation: false,
                                started_at: Some(chrono::Utc::now().to_rfc3339()),
                                completed_at: None,
                            };
                            live.snapshot.tool_calls.push(tc.clone());
                            if let Some(ref ch) = live.channel {
                                if ch.send(StreamEvent::ToolCallStart { tool_call: tc }).is_err() {
                                    live.channel = None;
                                }
                            }
                        }
                        ToolCallEvent::ArgumentsComplete { tool_call_id, tool_name, arguments } => {
                            if let Some(tc) = live.snapshot.tool_calls.iter_mut().find(|t| t.tool_call_id == tool_call_id) {
                                tc.arguments = arguments.clone();
                            }
                            pending_tool_calls.push(PendingToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                            });
                        }
                        ToolCallEvent::Done { tool_call_id, result, error } => {
                            if let Some(tc) = live.snapshot.tool_calls.iter_mut().find(|t| t.tool_call_id == tool_call_id) {
                                tc.status = if error.is_some() { ToolCallStatus::Failed } else { ToolCallStatus::Completed };
                                tc.result.clone_from(&result);
                                tc.error.clone_from(&error);
                                tc.completed_at = Some(chrono::Utc::now().to_rfc3339());
                            }
                            if let Some(ref ch) = live.channel {
                                if ch.send(StreamEvent::ToolCallDone { tool_call_id, result, error }).is_err() {
                                    live.channel = None;
                                }
                            }
                        }
                    }
                    continue;
                }

                let has_content = !chunk.delta.is_empty() || chunk.thinking_delta.is_some();
                if has_content {
                    let prefixed_accumulated = format!("{text_prefix}{}", chunk.accumulated);
                    full_text.clone_from(&prefixed_accumulated);
                    if let Some(ref acc) = chunk.accumulated_thinking {
                        full_thinking.clone_from(acc);
                    }

                    let mut live = live.lock().await;
                    live.snapshot.accumulated_text.clone_from(&prefixed_accumulated);
                    live.snapshot.accumulated_thinking = chunk.accumulated_thinking.clone();
                    live.snapshot.is_thinking = chunk.accumulated_thinking.is_some() && chunk.accumulated.is_empty();

                    if let Some(ref ch) = live.channel {
                        if ch.send(StreamEvent::Chunk {
                            delta: chunk.delta,
                            accumulated: prefixed_accumulated,
                            thinking_delta: chunk.thinking_delta,
                            accumulated_thinking: chunk.accumulated_thinking,
                        }).is_err() {
                            live.channel = None;
                        }
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let mut live = live.lock().await;
                live.snapshot.finished = true;
                live.snapshot.error = Some(msg.clone());
                if let Some(ref ch) = live.channel {
                    let _ = ch.send(StreamEvent::Error { message: msg.clone() });
                }
                return Err(msg);
            }
        }
    }

    let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking) };

    if pending_tool_calls.is_empty() {
        let mut live = live.lock().await;
        live.snapshot.finished = true;
        live.snapshot.prompt_tokens = prompt_tokens;
        live.snapshot.completion_tokens = completion_tokens;
        if let Some(ref ch) = live.channel {
            let _ = ch.send(StreamEvent::Done {
                full_text: full_text.clone(),
                full_thinking: thinking.clone(),
                prompt_tokens,
                completion_tokens,
            });
        }
    }

    Ok(StreamResult { full_text, full_thinking: thinking, prompt_tokens, completion_tokens, pending_tool_calls })
}

fn extract_mcp_result_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct ToolExecutionResult {
    tool_call_id: String,
    tool_name: String,
    result_text: String,
    is_error: bool,
}

async fn execute_tool_calls(
    state: &State<'_, Mutex<AppState>>,
    pending: &[PendingToolCall],
    live: &Arc<TokioMutex<LiveExecution>>,
) -> Vec<ToolExecutionResult> {
    let mcp = {
        let s = state.lock().await;
        Arc::clone(&s.mcp)
    };

    let futures: Vec<_> = pending
        .iter()
        .map(|tc| {
            let tool_call_id = tc.tool_call_id.clone();
            let tool_name = tc.tool_name.clone();
            let arguments = tc.arguments.clone();
            let mcp = Arc::clone(&mcp);
            let live = Arc::clone(live);
            async move {
                let result = mcp.call_tool(&tool_name, arguments).await;

                let execution_result = match result {
                    Ok(result) => {
                        let is_error = result.is_error.unwrap_or(false);
                        let text = extract_mcp_result_text(&result);
                        ToolExecutionResult {
                            tool_call_id,
                            tool_name,
                            result_text: text,
                            is_error,
                        }
                    }
                    Err(e) => ToolExecutionResult {
                        tool_call_id,
                        result_text: format!("Error executing tool '{}': {}", tool_name, e),
                        tool_name,
                        is_error: true,
                    },
                };

                let mut live = live.lock().await;
                if let Some(tc) = live.snapshot.tool_calls.iter_mut().find(|t| t.tool_call_id == execution_result.tool_call_id) {
                    tc.status = if execution_result.is_error { ToolCallStatus::Failed } else { ToolCallStatus::Completed };
                    if execution_result.is_error {
                        tc.error = Some(execution_result.result_text.clone());
                    } else {
                        tc.result = Some(execution_result.result_text.clone());
                    }
                    tc.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
                if let Some(ref ch) = live.channel {
                    let _ = ch.send(StreamEvent::ToolCallDone {
                        tool_call_id: execution_result.tool_call_id.clone(),
                        result: if execution_result.is_error { None } else { Some(execution_result.result_text.clone()) },
                        error: if execution_result.is_error { Some(execution_result.result_text.clone()) } else { None },
                    });
                }

                execution_result
            }
        })
        .collect();

    futures::future::join_all(futures).await
}

fn build_tool_loop_messages(
    pending: &[PendingToolCall],
    accumulated_text: &str,
    results: &[ToolExecutionResult],
) -> Vec<ProcessedMessage> {
    let mut messages = Vec::with_capacity(1 + results.len());

    let tool_calls: Vec<ToolCallPayload> = pending
        .iter()
        .map(|tc| ToolCallPayload {
            id: tc.tool_call_id.clone(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: tc.tool_name.clone(),
                arguments: serde_json::to_string(&tc.arguments).unwrap_or_default(),
            },
        })
        .collect();

    messages.push(ProcessedMessage {
        role: "assistant".to_string(),
        content: MessageContent::Text(accumulated_text.to_string()),
        tool_calls: Some(tool_calls),
        tool_call_id: None,
    });

    for r in results {
        messages.push(ProcessedMessage {
            role: "tool".to_string(),
            content: MessageContent::Text(r.result_text.clone()),
            tool_calls: None,
            tool_call_id: Some(r.tool_call_id.clone()),
        });
    }

    messages
}

#[derive(Clone, Serialize)]
struct ExecutionStartedPayload {
    execution_id: String,
    skill_id: Option<String>,
}

#[derive(Clone, Serialize)]
struct ExecutionCompletedPayload {
    execution_id: String,
    success: bool,
    error: Option<String>,
    cancelled: bool,
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

    let (execution_id, cancel_rx, model_id, model_display_name, skill_display_name, input_content, messages, ai, param_overrides) = {
        let mut state = state.lock().await;

        let (execution_id, cancel_rx) = state
            .prompt_execution
            .start_skill_execution(skill_name.clone())
            .map_err(|e| e.to_string())?;

        let skill = skill_message::resolve_skill_or_err(&state.skill_service, &skill_name)
            .map_err(|e| {
                state.prompt_execution.finish_skill_execution();
                e.to_string()
            })?;
        let skill_display_name = skill.display_name.clone();

        let model_id = PromptExecutionService::resolve_quick_action_model(&state.config, skill.model.as_deref())
            .map_err(|e| {
                state.prompt_execution.finish_skill_execution();
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
                state.prompt_execution.finish_skill_execution();
                e.to_string()
            })?,
        };

        let system_prompt = build_system_prompt_base(&state.config, None, state.active_app(), &state.recent_apps_display());
        let messages = skill_message::prepare_skill_messages(
            &system_prompt,
            &skill,
            &input_content,
            &state.context,
        );

        let ai = state.ai.clone();
        let surface_params = state
            .config
            .settings()
            .surfaces
            .quick_actions
            .generation
            .parameters
            .clone();
        let skill_params = skill.parameters.as_ref().map(ModelParameters::from_map);
        let param_overrides = merge_optional_parameters(Some(surface_params), skill_params);

        (execution_id, cancel_rx, model_id, model_display_name, skill_display_name, input_content, messages, ai, param_overrides)
    };

    let user_display_text = format!("/{skill_name} {input_content}");
    let live_execution = {
        let mut state = state.lock().await;
        let (live, _) = state.prompt_execution.start_live(&execution_id, user_display_text.clone(), on_event);
        live
    };

    let _ = app.emit(
        "execution-started",
        ExecutionStartedPayload {
            execution_id: execution_id.clone(),
            skill_id: Some(skill_name.clone()),
        },
    );

    let stream_result = {
        let stream = ai
            .complete_stream(&model_id, messages, param_overrides, None, vec![])
            .await
            .map_err(|e| e.to_string());

        match stream {
            Ok(stream) => run_stream_loop(stream, live_execution, Some(cancel_rx), "").await,
            Err(e) => Err(e),
        }
    };

    let mut state = state.lock().await;

    match stream_result {
        Ok(result) => {
            let full_text = result.full_text;
            let _ = state.clipboard.set_text(&full_text);
            let elapsed = start_time.elapsed();

            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            let user_node_id = format!("skill-user-{}", uuid::Uuid::new_v4());
            let assistant_node_id = format!("skill-asst-{}", uuid::Uuid::new_v4());

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
                thinking_duration: None,
                query_duration: None,
                error: None,
                cancelled: false,
                tool_calls: vec![],
                text_attachments: vec![],
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
                thinking_duration: None,
                query_duration: Some(elapsed.as_secs_f64()),
                error: None,
                cancelled: false,
                tool_calls: vec![],
                text_attachments: vec![],
            };

            state.history.add_conversation_entry(
                String::new(),
                Some(skill_name),
                Some(skill_display_name.clone()),
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
                &format!("{skill_display_name} completed"),
                Some(format!(
                    "{model_display_name} · {:.1}s · copied to clipboard",
                    elapsed.as_secs_f64()
                )),
                &state.config.settings().notifications,
            );

            state.prompt_execution.finish_skill_execution();
            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: true,
                    error: None,
                    cancelled: false,
                },
            );
            Ok(())
        }
        Err(error) => {
            let is_cancelled = error == "Cancelled";

            if is_cancelled {
                let _ = state.notifications.notify(
                    "prompt_execution_cancel",
                    NotificationLevel::Info,
                    "Prompt cancelled",
                    None::<&str>,
                    &state.config.settings().notifications,
                );
            } else {
                let _ = state.notifications.notify(
                    "prompt_execution_error",
                    NotificationLevel::Error,
                    "Execution failed",
                    Some(error.as_str()),
                    &state.config.settings().notifications,
                );
            }

            state.prompt_execution.finish_skill_execution();
            let _ = app.emit(
                "execution-completed",
                ExecutionCompletedPayload {
                    execution_id,
                    success: false,
                    error: Some(error.clone()),
                    cancelled: is_cancelled,
                },
            );
            if is_cancelled { Ok(()) } else { Err(error) }
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

        let model_id = PromptExecutionService::resolve_title_generation_model(&state.config)
            .map_err(|e| e.to_string())?;

        (
            model_id,
            settings.surfaces.title_generation.prompt.clone(),
            state.ai.clone(),
        )
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
    let system_prompt = &config.settings().prompt_base.system_prompt;
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
    let result = skill_message::resolve_skill_input(&state.skill_service, &text);
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
    _app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    mut nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    _skill_id: Option<String>,
    _skill_name: Option<String>,
    model_id: Option<String>,
    reasoning_effort: Option<String>,
    tools_override: Option<Vec<String>>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let (execution_id, resolved_model_id, _model_display_name, mut all_messages, ai, updates_for_event, param_overrides) = {
        let mut state = state.lock().await;

        let execution_id = uuid::Uuid::new_v4().to_string();

        let resolved_model_id =
            PromptExecutionService::resolve_model(&state.config, model_id.as_deref()).map_err(|e| {
                e.to_string()
            })?;

        let chat_surface_params = state
            .config
            .settings()
            .surfaces
            .chat
            .generation
            .parameters
            .clone();
        let tab_override = surface_effort_override(None, reasoning_effort);
        let param_overrides = merge_optional_parameters(Some(chat_surface_params), tab_override);

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
            tool_calls: None,
            tool_call_id: None,
        };

        let tree_messages =
            skill_message::build_messages_from_tree(&nodes, &context_images);

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

    let user_message = nodes.iter().rev()
        .find(|n| n.role == "user")
        .map(|n| n.content.clone())
        .unwrap_or_default();

    let (live_execution, cancel_rx) = {
        let mut state = state.lock().await;
        state.prompt_execution.start_live(&execution_id, user_message, on_event.clone())
    };

    if let Some((node_id, updates)) = updates_for_event {
        let _ = on_event.send(StreamEvent::NodeUpdates { node_id, updates });
    }

    let (builtin_tools, mcp_tools) = {
        let s = state.lock().await;
        match &tools_override {
            Some(requested) => {
                let builtin: Vec<String> = requested
                    .iter()
                    .filter(|t| !t.contains('.'))
                    .cloned()
                    .collect();
                let mcp_names: Vec<String> = requested
                    .iter()
                    .filter(|t| t.contains('.'))
                    .cloned()
                    .collect();
                let mcp = s.mcp.get_tools_by_qualified_names(&mcp_names);
                (Some(builtin), mcp)
            }
            None => (None, vec![]),
        }
    };

    let mut iteration = 0;
    let mut accumulated_prefix = String::new();
    loop {
        let stream = ai
            .complete_stream(
                &resolved_model_id,
                all_messages.clone(),
                param_overrides.clone(),
                builtin_tools.clone(),
                mcp_tools.clone(),
            )
            .await
            .map_err(|e| e.to_string());

        let stream_result = match stream {
            Ok(stream) => run_stream_loop(stream, Arc::clone(&live_execution), Some(cancel_rx.clone()), &accumulated_prefix).await,
            Err(e) => Err(e),
        };

        match stream_result {
            Ok(result) => {
                if result.pending_tool_calls.is_empty() || iteration >= MAX_TOOL_LOOP_ITERATIONS {
                    if iteration >= MAX_TOOL_LOOP_ITERATIONS {
                        log::warn!("Tool loop reached max iterations ({})", MAX_TOOL_LOOP_ITERATIONS);
                        let mut live = live_execution.lock().await;
                        live.snapshot.finished = true;
                        if let Some(ref ch) = live.channel {
                            let _ = ch.send(StreamEvent::Done {
                                full_text: result.full_text,
                                full_thinking: result.full_thinking,
                                prompt_tokens: result.prompt_tokens,
                                completion_tokens: result.completion_tokens,
                            });
                        }
                    }
                    let mut s = state.lock().await;
                    s.prompt_execution.clear_live();
                    return Ok(());
                }

                accumulated_prefix = result.full_text.clone();
                iteration += 1;
                log::info!("Tool loop iteration {}", iteration);

                {
                    let mut live = live_execution.lock().await;
                    live.snapshot.finished = false;
                }

                let tool_results = execute_tool_calls(&state, &result.pending_tool_calls, &live_execution).await;

                let new_messages = build_tool_loop_messages(
                    &result.pending_tool_calls,
                    &result.full_text,
                    &tool_results,
                );

                for tc in &result.pending_tool_calls {
                    log::debug!("Executed tool '{}' (id: {})", tc.tool_name, tc.tool_call_id);
                }

                all_messages.extend(new_messages);
            }
            Err(error) => {
                let _ = on_event.send(StreamEvent::Error { message: error.clone() });
                let mut s = state.lock().await;
                s.prompt_execution.clear_live();
                return Err(error);
            }
        }
    }
}


#[tauri::command]
pub async fn reconnect_to_execution(
    state: State<'_, Mutex<AppState>>,
    on_event: Channel<StreamEvent>,
) -> Result<Option<ExecutionSnapshot>, String> {
    let live_arc = {
        let state = state.lock().await;
        state.prompt_execution.live.clone()
    };

    let Some(live_arc) = live_arc else {
        return Ok(None);
    };

    let mut live = live_arc.lock().await;
    let snapshot = live.snapshot.clone();

    if !snapshot.finished {
        live.channel = Some(on_event);
    }

    Ok(Some(snapshot))
}

#[tauri::command]
pub async fn cancel_skill_execution(
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let mut state = state.lock().await;
    Ok(state.prompt_execution.cancel_execution())
}

#[tauri::command]
pub async fn cancel_live_execution(
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let mut state = state.lock().await;
    Ok(state.prompt_execution.cancel_live())
}

#[tauri::command]
pub async fn get_executing_skill_id(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    let state = state.lock().await;
    Ok(state.prompt_execution.executing_skill_id().map(|s| s.to_string()))
}
