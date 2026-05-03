use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextConfig,
};
use crate::services::ai::AiService;
use crate::services::config::{ConfigService, SurfaceKind};

fn emit_changed(app: &AppHandle) -> crate::Result<()> {
    app.emit("settings-changed", ())?;
    Ok(())
}

fn save_and_emit(config: &ConfigService, app: &AppHandle) -> crate::Result<()> {
    config.save()?;
    emit_changed(app)
}

fn rebuild_ai(config: &ConfigService, ai: &mut AiService) {
    *ai = AiService::new(&config.settings().models);
}

#[tauri::command]
pub async fn get_settings(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Settings> {
    Ok(config.lock().await.settings().clone())
}

#[tauri::command]
pub async fn update_setting(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    key: String,
    value: serde_json::Value,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_setting(&key, value);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_surface_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    model_id: Option<String>,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_surface_model(surface, model_id);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_surface_parameter(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    key: String,
    value: serde_json::Value,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_surface_parameter(surface, &key, value);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_surface_enabled_tools(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    tools: Vec<String>,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_surface_enabled_tools(surface, tools);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_speech_to_text_config(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    config_value: SpeechToTextConfig,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_speech_to_text(config_value);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn add_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    config_value: ModelConfig,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.add_model(config_value);
    rebuild_ai(&config, &mut *ai.lock().await);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    model_id: String,
    config_value: ModelConfig,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_model(&model_id, config_value);
    rebuild_ai(&config, &mut *ai.lock().await);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn delete_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    model_id: String,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.delete_model(&model_id);
    rebuild_ai(&config, &mut *ai.lock().await);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_notifications(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    config_value: NotificationSettings,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_notifications(config_value);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn update_keymaps(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    keymaps: Vec<KeymapGroup>,
) -> crate::Result<()> {
    let settings = {
        let mut c = config.lock().await;
        c.update_keymaps(keymaps);
        save_and_emit(&c, &app)?;
        c.settings().clone()
    };
    crate::reload_shortcuts(&app, &settings);
    Ok(())
}

#[tauri::command]
pub async fn update_menu_section_order(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    order: Vec<String>,
) -> crate::Result<()> {
    let mut config = config.lock().await;
    config.update_menu_section_order(order);
    save_and_emit(&config, &app)
}

#[tauri::command]
pub async fn reload_settings(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
) -> crate::Result<()> {
    let settings = {
        let mut c = config.lock().await;
        c.reload()?;
        rebuild_ai(&c, &mut *ai.lock().await);
        c.settings().clone()
    };
    crate::reload_shortcuts(&app, &settings);
    crate::services::autostart::reconcile(&app, &settings);
    Ok(())
}
