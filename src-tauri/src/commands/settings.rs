use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextModel,
};
use crate::services::ai::AiService;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::history::HistoryService;
use crate::services::image_storage::ImageStorage;
use crate::services::menu_coordinator::MenuCoordinator;
use crate::services::notification::NotificationService;
use crate::services::placeholder::PlaceholderService;
use crate::services::prompt_execution::PromptExecutionService;
use crate::services::skill::SkillService;
use crate::services::speech::SpeechService;

pub struct AppState {
    pub config: ConfigService,
    pub clipboard: ClipboardService,
    pub notifications: NotificationService,
    pub menu_coordinator: MenuCoordinator,
    pub context: ContextManagerService,
    pub placeholder: PlaceholderService,
    pub ai: AiService,
    pub history: HistoryService,
    pub image_storage: ImageStorage,
    pub prompt_execution: PromptExecutionService,
    pub skill_service: SkillService,
    pub speech: SpeechService,
}

fn emit_changed(app: &AppHandle) -> Result<(), String> {
    app.emit("settings-changed", ()).map_err(|e| e.to_string())
}

fn save_and_emit(config: &ConfigService, app: &AppHandle) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    emit_changed(app)
}

fn rebuild_ai(state: &mut AppState) {
    state.ai = AiService::new(&state.config.settings().models);
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, Mutex<AppState>>,
) -> Result<Settings, String> {
    let state = state.lock().await;
    Ok(state.config.settings().clone())
}

#[tauri::command]
pub async fn update_setting(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_setting(&key, value);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn add_model(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    config: ModelConfig,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.add_model(config);
    rebuild_ai(&mut state);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn update_model(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    model_id: String,
    config: ModelConfig,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_model(&model_id, config);
    rebuild_ai(&mut state);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn delete_model(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    model_id: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.delete_model(&model_id);
    rebuild_ai(&mut state);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn update_notifications(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    config: NotificationSettings,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_notifications(config);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn update_speech_model(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    config: SpeechToTextModel,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_speech_model(config);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn update_keymaps(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    keymaps: Vec<KeymapGroup>,
) -> Result<(), String> {
    let settings = {
        let mut s = state.lock().await;
        s.config.update_keymaps(keymaps);
        save_and_emit(&s.config, &app)?;
        s.config.settings().clone()
    };
    crate::reload_shortcuts(&app, &settings);
    Ok(())
}

#[tauri::command]
pub async fn update_menu_section_order(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    order: Vec<String>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_menu_section_order(order);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn reload_settings(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let settings = {
        let mut s = state.lock().await;
        s.config.reload().map_err(|e| e.to_string())?;
        rebuild_ai(&mut s);
        s.config.settings().clone()
    };
    crate::reload_shortcuts(&app, &settings);
    Ok(())
}
