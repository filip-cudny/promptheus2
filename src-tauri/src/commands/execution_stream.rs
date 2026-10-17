use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::models::ai::StreamEvent;
use crate::models::history::{InvocationSource, OutputRoute};
use crate::models::message::{AppliedSkill, ConversationNodeForExecution, ImageData};
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::conversation_context::ConversationContextCache;
use crate::services::execution::conversation_executor::ConversationExecutor;
use crate::services::execution::skill_executor::run_skill_stream;
use crate::services::execution::PromptExecutionService;
use crate::services::mcp::McpRegistry;
use crate::services::recent_apps::RecentAppsState;
use crate::services::skill::SkillService;
use crate::services::skill_message;
use crate::services::sqlite_history::SqliteHistoryService;

#[tauri::command]
pub async fn execute_skill(
    app: AppHandle,
    skill_name: String,
    input_override: Option<String>,
    invocation_source: InvocationSource,
    on_event: Channel<StreamEvent>,
) -> crate::Result<()> {
    run_skill_stream(
        &app,
        skill_name,
        input_override,
        invocation_source,
        OutputRoute::Clipboard,
        true,
        on_event,
    )
    .await
    .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn execute_conversation_from_tree(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    conversation_context: State<'_, Arc<Mutex<ConversationContextCache>>>,
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
    mcp: State<'_, Arc<McpRegistry>>,
    recent_apps: State<'_, Arc<RecentAppsState>>,
    nodes: Vec<ConversationNodeForExecution>,
    context_text: Option<String>,
    context_images: Vec<ImageData>,
    tab_id: String,
    model_id: Option<String>,
    reasoning_effort: Option<String>,
    tools_override: Option<Vec<String>>,
    on_event: Channel<StreamEvent>,
) -> crate::Result<()> {
    ConversationExecutor::run(
        Arc::clone(&config),
        Arc::clone(&ai),
        Arc::clone(&history),
        Arc::clone(&conversation_context),
        Arc::clone(&prompt_execution),
        Arc::clone(&mcp),
        Arc::clone(&recent_apps),
        nodes,
        context_text,
        context_images,
        tab_id,
        model_id,
        reasoning_effort,
        tools_override,
        on_event,
    )
    .await
}

#[tauri::command]
pub async fn release_conversation_context(
    conversation_context: State<'_, Arc<Mutex<ConversationContextCache>>>,
    tab_id: String,
) -> crate::Result<()> {
    conversation_context.lock().await.remove(&tab_id);
    Ok(())
}

#[tauri::command]
pub async fn seed_conversation_context(
    conversation_context: State<'_, Arc<Mutex<ConversationContextCache>>>,
    tab_id: String,
    resolved_environment_section: String,
) -> crate::Result<()> {
    let mut cache = conversation_context.lock().await;
    if !cache.has(&tab_id) {
        cache.insert(tab_id, resolved_environment_section);
    }
    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct ResolveSkillInputResult {
    pub had_skills: bool,
    pub applied_skills: Vec<AppliedSkill>,
}

#[tauri::command]
pub async fn resolve_skill_input(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    text: String,
) -> crate::Result<ResolveSkillInputResult> {
    let mut skill_service = skill_service.lock().await;
    let history = history.lock().await;
    for name in skill_message::referenced_skill_names(&text) {
        let needs_version = skill_service
            .get_skill(&name)
            .is_some_and(|s| s.skill_version_id.is_none());
        if needs_version {
            let _ = skill_service.ensure_version(&name, history.conn());
        }
    }
    let result = skill_message::resolve_skill_input(&skill_service, &text);
    Ok(ResolveSkillInputResult {
        had_skills: result.had_skills,
        applied_skills: result.applied_skills,
    })
}
