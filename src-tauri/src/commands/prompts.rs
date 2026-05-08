use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::services::config::{ConfigService, PromptKind};
use crate::services::placeholder_registry::{list, PlaceholderContext, PlaceholderInfo};
use crate::services::recent_apps::RecentAppsState;

#[derive(Serialize)]
pub struct PromptDoc {
    pub kind: PromptKind,
    pub label: &'static str,
    pub path: String,
    pub content: String,
    pub supports_env_placeholders: bool,
}

#[tauri::command]
pub async fn list_prompts(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<PromptDoc>> {
    let config = config.lock().await;
    Ok(PromptKind::ALL
        .iter()
        .map(|kind| build_prompt_doc(&config, *kind))
        .collect())
}

#[tauri::command]
pub async fn get_prompt(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    kind: PromptKind,
) -> crate::Result<PromptDoc> {
    let config = config.lock().await;
    Ok(build_prompt_doc(&config, kind))
}

#[tauri::command]
pub async fn save_prompt(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    kind: PromptKind,
    content: String,
) -> crate::Result<()> {
    let config = config.lock().await;
    config.write_prompt(kind, &content)?;
    app.emit("prompt-changed", &kind)?;
    Ok(())
}

#[tauri::command]
pub async fn get_environment_placeholders(
    recent_apps: State<'_, Arc<RecentAppsState>>,
) -> crate::Result<Vec<PlaceholderInfo>> {
    let active_app = recent_apps.active().await;
    let recent = recent_apps.display().await;
    Ok(list(&PlaceholderContext::with_apps(active_app, recent)))
}

fn build_prompt_doc(config: &ConfigService, kind: PromptKind) -> PromptDoc {
    let path = config
        .prompt_path(kind)
        .map(|p| p.to_string())
        .unwrap_or_else(|| kind.default_path().to_string());
    let content = config.read_prompt(kind);
    PromptDoc {
        kind,
        label: kind.label(),
        path,
        content,
        supports_env_placeholders: kind.supports_env_placeholders(),
    }
}
