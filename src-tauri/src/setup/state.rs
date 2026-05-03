use std::path::Path;
use std::sync::Arc;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::providers::{LastInteractionMenuProvider, SkillMenuProvider, SpeechMenuProvider};
use crate::services::ai::AiService;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::{ContextManagerService, ContextMenuProvider};
use crate::services::database::Database;
use crate::services::dock::DockManager;
use crate::services::execution::PromptExecutionService;
use crate::services::history_search::HistorySearch;
use crate::services::image_storage::ImageStorage;
use crate::services::mcp::McpRegistry;
use crate::services::menu_coordinator::MenuCoordinator;
use crate::services::notification::NotificationService;
use crate::services::placeholder::PlaceholderService;
use crate::services::recent_apps::RecentAppsState;
use crate::services::skill::SkillService;
use crate::services::speech::SpeechService;
use crate::services::sqlite_history::SqliteHistoryService;
use crate::services::{self, conversation_context, tool_confirmation, ui_state};

pub fn manage(
    app: &tauri::App,
    config_dir: &Path,
    resource_dir: &Path,
    config_service: ConfigService,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    app.manage(DockManager::new());
    app.manage(services::ai_webview::AiWebviewState::default());

    let skills_dir = config_dir.join("skills");
    let mut skill_service = SkillService::load(
        &skills_dir,
        Some(resource_dir),
        &config_service.settings().skills_order,
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    log::info!(
        "loaded {} skills from {}",
        skill_service.list_skills().len(),
        skills_dir.display()
    );

    let clipboard_service = ClipboardService::new(app.handle().clone())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let notification_service = NotificationService::new(app.handle().clone());
    let mut menu_coordinator = MenuCoordinator::new();
    menu_coordinator.add_provider(Box::new(ContextMenuProvider::new()));
    menu_coordinator.add_provider(Box::new(LastInteractionMenuProvider::new()));
    menu_coordinator.add_provider(Box::new(SpeechMenuProvider::new()));

    let skill_summaries: Vec<_> = skill_service
        .list_skills()
        .iter()
        .map(|s| crate::models::skill::SkillSummary {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            description: s.description.clone(),
        })
        .collect();
    menu_coordinator.add_provider(Box::new(SkillMenuProvider::new(skill_summaries)));

    let ui_state_service = ui_state::UiStateService::load(config_dir)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let ai_service = AiService::new(&config_service.settings().models);
    let context_service = ContextManagerService::new();
    let placeholder_service = PlaceholderService::new();
    let app_data_dir = app.path().app_data_dir()?;
    let database =
        Database::open(&app_data_dir).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    skill_service
        .sync_versions(database.conn())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let history_service = SqliteHistoryService::new(database, 1000);
    let image_storage = ImageStorage::new(&app_data_dir);
    image_storage
        .initialize()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    log::info!("image storage initialized at {}", app_data_dir.display());

    let mcp_servers_config = config_service.settings().mcp_servers.clone();
    let mcp_registry = tauri::async_runtime::block_on(McpRegistry::start_all(&mcp_servers_config));

    app.manage(Arc::new(Mutex::new(config_service)));
    app.manage(clipboard_service);
    app.manage(notification_service);
    app.manage(Arc::new(Mutex::new(menu_coordinator)));
    app.manage(Arc::new(Mutex::new(context_service)));
    app.manage(Arc::new(Mutex::new(placeholder_service)));
    app.manage(Arc::new(Mutex::new(ai_service)));
    app.manage(Arc::new(Mutex::new(history_service)));
    app.manage(Arc::new(Mutex::new(HistorySearch::new())));
    app.manage(image_storage);
    app.manage(Arc::new(mcp_registry));
    app.manage(Arc::new(Mutex::new(PromptExecutionService::new())));
    app.manage(Arc::new(Mutex::new(skill_service)));
    app.manage(Arc::new(Mutex::new(SpeechService::new())));
    app.manage(Arc::new(Mutex::new(ui_state_service)));
    app.manage(Arc::new(Mutex::new(
        conversation_context::ConversationContextCache::new(),
    )));
    app.manage(Arc::new(Mutex::new(
        tool_confirmation::ToolConfirmationService::new(),
    )));
    app.manage(Arc::new(RecentAppsState::new()));

    if !mcp_servers_config.is_empty() {
        let _ = app.emit("mcp-ready", ());
    }

    Ok(())
}
