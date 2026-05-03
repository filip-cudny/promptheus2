use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::parameters::{merge_optional_parameters, surface_effort_override};
use super::stream::{build_tool_loop_messages, execute_tool_calls, run_stream_loop};
use super::system_prompt::{build_system_prompt_base, resolve_environment_section_template};
use super::types::MAX_TOOL_LOOP_ITERATIONS;
use crate::models::ai::StreamEvent;
use crate::models::message::{
    ConversationNodeForExecution, ImageData, MessageContent, NodeUpdate, ProcessedMessage,
};
use crate::models::settings::ModelParameters;
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::conversation_context::ConversationContextCache;
use crate::services::execution::lifecycle::{LiveExecution, PromptExecutionService};
use crate::services::mcp::McpRegistry;
use crate::services::recent_apps::RecentAppsState;
use crate::services::skill_message;
use crate::services::sqlite_history::SqliteHistoryService;

pub struct ConversationPrepared {
    pub execution_id: String,
    pub resolved_model_id: String,
    pub all_messages: Vec<ProcessedMessage>,
    pub ai: AiService,
    pub param_overrides: Option<ModelParameters>,
    pub updates_for_event: Option<(String, Vec<NodeUpdate>)>,
    pub user_message: String,
}

pub struct ConversationExecutor;

impl ConversationExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        config: &ConfigService,
        ai: &AiService,
        history: &SqliteHistoryService,
        conversation_context: &mut ConversationContextCache,
        active_app: &str,
        recent_apps: &str,
        nodes: &mut [ConversationNodeForExecution],
        context_text: Option<&str>,
        context_images: &[ImageData],
        tab_id: &str,
        model_id: Option<&str>,
        reasoning_effort: Option<String>,
    ) -> crate::Result<ConversationPrepared> {
        let execution_id = Uuid::new_v4().to_string();

        let resolved_model_id = PromptExecutionService::resolve_model(config, model_id)?;

        let chat_surface_params = config
            .settings()
            .surfaces
            .chat
            .generation
            .parameters
            .clone();
        let tab_override = surface_effort_override(None, reasoning_effort);
        let param_overrides = merge_optional_parameters(Some(chat_surface_params), tab_override);

        if !conversation_context.has(tab_id) {
            let resolved = resolve_environment_section_template(config, active_app, recent_apps);
            conversation_context.insert(tab_id.to_string(), resolved);
        }
        let resolved_environment_section =
            conversation_context.get(tab_id).unwrap().to_string();

        let system_content = build_system_prompt_base(
            config,
            Some(&resolved_environment_section),
            active_app,
            recent_apps,
        );

        let mut node_updates: Vec<NodeUpdate> = Vec::new();

        if let Some(env_update) =
            conversation_context.environment_update_if_stale(tab_id, active_app, recent_apps)
        {
            node_updates.push(env_update);
        }

        let context_str = context_text.unwrap_or("");
        let image_data_len: usize = context_images.iter().map(|i| i.data.len()).sum();
        if let Some(ctx_update) = conversation_context.context_update_if_changed(
            tab_id,
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

        let version_ids = skill_message::collect_skill_version_ids(nodes);
        let skill_bodies =
            skill_message::load_skill_version_bodies(history.conn(), &version_ids)?;
        let tree_messages =
            skill_message::build_messages_from_tree(nodes, context_images, &skill_bodies);

        let mut all_messages = vec![system_message];
        all_messages.extend(tree_messages);

        let user_message = nodes
            .iter()
            .rev()
            .find(|n| n.role == "user")
            .map(|n| n.content.clone())
            .unwrap_or_default();

        Ok(ConversationPrepared {
            execution_id,
            resolved_model_id,
            all_messages,
            ai: ai.clone(),
            param_overrides,
            updates_for_event,
            user_message,
        })
    }

    pub fn resolve_tools(
        mcp: &Arc<McpRegistry>,
        tools_override: Option<&[String]>,
    ) -> (Option<Vec<String>>, Vec<rmcp::model::Tool>) {
        match tools_override {
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
                let mcp_tools = mcp.get_tools_by_qualified_names(&mcp_names);
                (Some(builtin), mcp_tools)
            }
            None => (None, vec![]),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        config: Arc<Mutex<ConfigService>>,
        ai: Arc<Mutex<AiService>>,
        history: Arc<Mutex<SqliteHistoryService>>,
        conversation_context: Arc<Mutex<ConversationContextCache>>,
        prompt_execution: Arc<Mutex<PromptExecutionService>>,
        mcp: Arc<McpRegistry>,
        recent_apps: Arc<RecentAppsState>,
        mut nodes: Vec<ConversationNodeForExecution>,
        context_text: Option<String>,
        context_images: Vec<ImageData>,
        tab_id: String,
        model_id: Option<String>,
        reasoning_effort: Option<String>,
        tools_override: Option<Vec<String>>,
        on_event: Channel<StreamEvent>,
    ) -> crate::Result<()> {
        let active_app = recent_apps.active().await;
        let recent_apps_display = recent_apps.display().await;

        let ConversationPrepared {
            execution_id,
            resolved_model_id,
            all_messages,
            ai: ai_service,
            param_overrides,
            updates_for_event,
            user_message,
        } = {
            let config_guard = config.lock().await;
            let ai_guard = ai.lock().await;
            let history_guard = history.lock().await;
            let mut conv_ctx = conversation_context.lock().await;
            ConversationExecutor::prepare(
                &config_guard,
                &ai_guard,
                &history_guard,
                &mut conv_ctx,
                &active_app,
                &recent_apps_display,
                &mut nodes,
                context_text.as_deref(),
                &context_images,
                &tab_id,
                model_id.as_deref(),
                reasoning_effort,
            )?
        };

        let (live_execution, cancel_rx) = prompt_execution.lock().await.start_live(
            &execution_id,
            user_message,
            on_event.clone(),
        );

        if let Some((node_id, updates)) = updates_for_event {
            let _ = on_event.send(StreamEvent::NodeUpdates { node_id, updates });
        }

        let (builtin_tools, mcp_tools) =
            ConversationExecutor::resolve_tools(&mcp, tools_override.as_deref());

        let mut iteration = 0;
        let mut accumulated_prefix = String::new();
        let mut all_messages = all_messages;
        loop {
            let stream_result = match ai_service
                .complete_stream(
                    &resolved_model_id,
                    all_messages.clone(),
                    param_overrides.clone(),
                    builtin_tools.clone(),
                    mcp_tools.clone(),
                )
                .await
            {
                Ok(stream) => {
                    run_stream_loop(
                        stream,
                        Arc::clone(&live_execution),
                        Some(cancel_rx.clone()),
                        &accumulated_prefix,
                    )
                    .await
                }
                Err(e) => Err(e.into()),
            };

            match stream_result {
                Ok(result) => {
                    if result.pending_tool_calls.is_empty()
                        || iteration >= MAX_TOOL_LOOP_ITERATIONS
                    {
                        if iteration >= MAX_TOOL_LOOP_ITERATIONS {
                            log::warn!(
                                "Tool loop reached max iterations ({})",
                                MAX_TOOL_LOOP_ITERATIONS
                            );
                            ConversationExecutor::finalize_done(
                                &live_execution,
                                result.full_text,
                                result.full_thinking,
                                result.prompt_tokens,
                                result.completion_tokens,
                            )
                            .await;
                        }
                        prompt_execution.lock().await.clear_live();
                        return Ok(());
                    }

                    accumulated_prefix = result.full_text.clone();
                    iteration += 1;
                    log::info!("Tool loop iteration {}", iteration);

                    {
                        let mut live = live_execution.lock().await;
                        live.snapshot.finished = false;
                    }

                    let tool_results =
                        execute_tool_calls(&mcp, &result.pending_tool_calls, &live_execution)
                            .await;

                    let new_messages = build_tool_loop_messages(
                        &result.pending_tool_calls,
                        &result.full_text,
                        &tool_results,
                    );

                    for tc in &result.pending_tool_calls {
                        log::debug!(
                            "Executed tool '{}' (id: {})",
                            tc.tool_name,
                            tc.tool_call_id
                        );
                    }

                    all_messages.extend(new_messages);
                }
                Err(error) => {
                    let _ = on_event.send(StreamEvent::Error {
                        message: error.to_string(),
                    });
                    prompt_execution.lock().await.clear_live();
                    return Err(error);
                }
            }
        }
    }

    pub async fn finalize_done(
        live: &Arc<tokio::sync::Mutex<LiveExecution>>,
        full_text: String,
        full_thinking: Option<String>,
        prompt_tokens: Option<usize>,
        completion_tokens: Option<usize>,
    ) {
        let mut live = live.lock().await;
        live.snapshot.finished = true;
        if let Some(ref ch) = live.channel {
            let _ = ch.send(crate::models::ai::StreamEvent::Done {
                full_text,
                full_thinking,
                prompt_tokens,
                completion_tokens,
            });
        }
    }
}

