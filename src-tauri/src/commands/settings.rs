use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::settings::{
    ModelConfig, NotificationSettings, Settings, SpeechToTextConfig,
};
use crate::commands::skills::{emit_changed as emit_skills_changed, rescan_and_prune};
use crate::services::ai::AiService;
use crate::services::config::{ConfigService, SurfaceKind};
use crate::services::history_events::emit_history_changed;
use crate::services::skill::SkillService;
use crate::services::sqlite_history::SqliteHistoryService;

static SETTINGS_VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Clone)]
struct SettingsChangedEvent {
    version: u64,
}

pub(crate) fn emit_changed(app: &AppHandle) -> crate::Result<()> {
    let version = SETTINGS_VERSION.fetch_add(1, Ordering::Relaxed) + 1;
    app.emit("settings-changed", SettingsChangedEvent { version })?;
    Ok(())
}

pub(crate) fn rebuild_ai(config: &ConfigService, ai: &mut AiService) {
    *ai = AiService::new(&config.settings().models);
}

pub(crate) fn apply_reloaded_settings(app: &AppHandle, settings: &Settings) {
    crate::reload_shortcuts(app, settings);
    crate::services::autostart::reconcile(app, settings, false);
}

pub(crate) async fn apply_settings_reload(app: &AppHandle, reason: &str) -> crate::Result<()> {
    let config = app.state::<Arc<Mutex<ConfigService>>>();
    let ai = app.state::<Arc<Mutex<AiService>>>();
    let skills = app.state::<Arc<Mutex<SkillService>>>();
    let history = app.state::<Arc<Mutex<SqliteHistoryService>>>();

    let settings = {
        let guard = config.lock().await;
        rebuild_ai(&guard, &mut *ai.lock().await);
        guard.settings().clone()
    };
    rescan_and_prune(&skills, &config, &history).await?;
    apply_reloaded_settings(app, &settings);
    emit_changed(app)?;
    emit_skills_changed(app, reason)
}

async fn finish_mutation(app: &AppHandle, reloaded: bool) -> crate::Result<()> {
    if reloaded {
        apply_settings_reload(app, "reload").await
    } else {
        emit_changed(app)
    }
}

async fn mutate_settings(
    app: &AppHandle,
    config: &Mutex<ConfigService>,
    f: impl FnOnce(&mut ConfigService),
) -> crate::Result<()> {
    let reloaded = config.lock().await.mutate(f)?.reloaded;
    finish_mutation(app, reloaded).await
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
    mutate_settings(&app, &config, |c| c.update_setting(&key, value)).await
}

#[tauri::command]
pub async fn update_surface_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    model_id: Option<String>,
) -> crate::Result<()> {
    mutate_settings(&app, &config, |c| c.update_surface_model(surface, model_id)).await
}

#[tauri::command]
pub async fn update_surface_parameter(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    key: String,
    value: serde_json::Value,
) -> crate::Result<()> {
    mutate_settings(&app, &config, |c| {
        c.update_surface_parameter(surface, &key, value)
    })
    .await
}

#[tauri::command]
pub async fn update_surface_enabled_tools(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    surface: SurfaceKind,
    tools: Vec<String>,
) -> crate::Result<()> {
    mutate_settings(&app, &config, |c| {
        c.update_surface_enabled_tools(surface, tools)
    })
    .await
}

#[tauri::command]
pub async fn update_speech_to_text_config(
    app: AppHandle,
    config_svc: State<'_, Arc<Mutex<ConfigService>>>,
    config: SpeechToTextConfig,
) -> crate::Result<()> {
    mutate_settings(&app, &config_svc, |c| c.update_speech_to_text(config)).await
}

async fn mutate_models(
    app: &AppHandle,
    config: &Mutex<ConfigService>,
    ai: &Mutex<AiService>,
    f: impl FnOnce(&mut ConfigService),
) -> crate::Result<()> {
    let reloaded = {
        let mut svc = config.lock().await;
        let reloaded = svc.mutate(f)?.reloaded;
        rebuild_ai(&svc, &mut *ai.lock().await);
        reloaded
    };
    finish_mutation(app, reloaded).await
}

#[tauri::command]
pub async fn add_model(
    app: AppHandle,
    config_svc: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    config: ModelConfig,
) -> crate::Result<()> {
    mutate_models(&app, &config_svc, &ai, |c| c.add_model(config)).await
}

#[tauri::command]
pub async fn update_model(
    app: AppHandle,
    config_svc: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    model_id: String,
    config: ModelConfig,
) -> crate::Result<()> {
    mutate_models(&app, &config_svc, &ai, |c| c.update_model(&model_id, config)).await
}

#[tauri::command]
pub async fn delete_model(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    ai: State<'_, Arc<Mutex<AiService>>>,
    model_id: String,
) -> crate::Result<()> {
    mutate_models(&app, &config, &ai, |c| c.delete_model(&model_id)).await
}

#[tauri::command]
pub async fn update_notifications(
    app: AppHandle,
    config_svc: State<'_, Arc<Mutex<ConfigService>>>,
    config: NotificationSettings,
) -> crate::Result<()> {
    mutate_settings(&app, &config_svc, |c| c.update_notifications(config)).await
}

#[tauri::command]
pub async fn update_history_retention(
    app: AppHandle,
    config_svc: State<'_, Arc<Mutex<ConfigService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    days: u32,
) -> crate::Result<()> {
    mutate_settings(&app, &config_svc, |c| c.update_history_retention(days)).await?;

    let pruned = {
        let mut history = history.lock().await;
        tokio::task::block_in_place(|| {
            history.set_retention_days(days);
            let pruned = history.enforce_retention();
            if pruned > 0 {
                history.compact_after_bulk_delete();
            }
            pruned
        })
    };

    log::info!(
        "history retention set to {} days, pruned {} entries",
        days,
        pruned
    );

    if pruned > 0 {
        emit_history_changed(&app, None, None)?;
    }
    Ok(())
}
