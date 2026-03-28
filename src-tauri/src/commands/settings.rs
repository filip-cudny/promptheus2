use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, PromptData, Settings, SpeechToTextModel,
};
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::menu_coordinator::MenuCoordinator;
use crate::services::notification::NotificationService;
use crate::services::placeholder::PlaceholderService;

pub struct AppState {
    pub config: ConfigService,
    pub clipboard: ClipboardService,
    pub notifications: NotificationService,
    pub menu_coordinator: MenuCoordinator,
    pub context: ContextManagerService,
    pub placeholder: PlaceholderService,
}

fn emit_changed(app: &AppHandle) -> Result<(), String> {
    app.emit("settings-changed", ()).map_err(|e| e.to_string())
}

fn save_and_emit(config: &ConfigService, app: &AppHandle) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    emit_changed(app)
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
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn add_prompt(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    prompt: PromptData,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.add_prompt(prompt);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn update_prompt(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    prompt_id: String,
    prompt: PromptData,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.update_prompt(&prompt_id, prompt);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn delete_prompt(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    prompt_id: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.delete_prompt(&prompt_id);
    save_and_emit(&state.config, &app)
}

#[tauri::command]
pub async fn reorder_prompts(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    prompt_ids: Vec<String>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.reorder_prompts(&prompt_ids);
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
    let mut state = state.lock().await;
    state.config.update_keymaps(keymaps);
    save_and_emit(&state.config, &app)
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
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.config.reload().map_err(|e| e.to_string())
}
