use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::models::ai::StreamEvent;
use crate::models::message::{AppliedSkill, ConversationNodeForExecution, ImageData};
use crate::services::ai::AiService;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::conversation_context::ConversationContextCache;
use crate::services::execution::conversation_executor::ConversationExecutor;
use crate::services::execution::skill_executor::run_skill_stream;
use crate::services::execution::system_prompt::resolve_environment_section_template;
use crate::services::execution::PromptExecutionService;
use crate::services::mcp::McpRegistry;
use crate::services::notification::NotificationService;
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::recent_apps::RecentAppsState;
use crate::services::skill::SkillService;
use crate::services::skill_message;
use crate::services::sqlite_history::SqliteHistoryService;

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn execute_skill(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    clipboard: State<'_, ClipboardService>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    notifications: State<'_, NotificationService>,
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
    recent_apps: State<'_, Arc<RecentAppsState>>,
    skill_name: String,
    input_override: Option<String>,
    on_event: Channel<StreamEvent>,
) -> crate::Result<()> {
    run_skill_stream(
        app,
        Arc::clone(&config),
        (*clipboard).clone(),
        Arc::clone(&ai),
        Arc::clone(&skill_service),
        Arc::clone(&context),
        Arc::clone(&history),
        (*notifications).clone(),
        Arc::clone(&prompt_execution),
        Arc::clone(&recent_apps),
        skill_name,
        input_override,
        on_event,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn execute_conversation_from_tree(
    _app: AppHandle,
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
    _skill_id: Option<String>,
    _skill_name: Option<String>,
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
pub async fn resolve_environment_section(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    recent_apps: State<'_, Arc<RecentAppsState>>,
) -> crate::Result<String> {
    let active_app = recent_apps.active().await;
    let recent_apps_display = recent_apps.display().await;
    let ctx = PlaceholderContext::with_apps(active_app, recent_apps_display);
    Ok(resolve_environment_section_template(
        &*config.lock().await,
        &ctx,
    ))
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
    text: String,
) -> crate::Result<ResolveSkillInputResult> {
    let skill_service = skill_service.lock().await;
    let result = skill_message::resolve_skill_input(&skill_service, &text);
    Ok(ResolveSkillInputResult {
        had_skills: result.had_skills,
        applied_skills: result.applied_skills,
    })
}
