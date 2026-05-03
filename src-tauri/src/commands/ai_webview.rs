use std::sync::Arc;

use tauri::{Manager, State};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::ai_webview;
use crate::services::config::ConfigService;
use crate::Error;

async fn require_provider(
    config: &State<'_, Arc<Mutex<ConfigService>>>,
    provider_id: &str,
) -> crate::Result<WebviewProvider> {
    config
        .lock()
        .await
        .settings()
        .find_webview_provider(provider_id)
        .cloned()
        .ok_or_else(|| Error::Other(format!("unknown provider: {provider_id}")))
}

#[tauri::command]
pub async fn open_ai_webview(
    app: tauri::AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    provider_id: String,
    url: Option<String>,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_ai_webview: {provider_id} url={url:?}",
    );
    let provider = require_provider(&config, &provider_id).await?;
    ai_webview::open_or_focus(&app, provider, url)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn open_ai_webview_new_window(
    app: tauri::AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    provider_id: String,
    url: Option<String>,
    source_label: Option<String>,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_ai_webview_new_window: {provider_id} url={url:?} source={source_label:?}",
    );
    let provider = require_provider(&config, &provider_id).await?;
    ai_webview::open_new_instance(&app, provider, url, source_label)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn swap_ai_webview(
    app: tauri::AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    provider_id: String,
    from_label: String,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "swap_ai_webview: {provider_id} from={from_label}",
    );
    let provider = require_provider(&config, &provider_id).await?;
    ai_webview::swap_to_provider(&app, provider, &from_label)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn swap_to_conversation_dialog(
    app: tauri::AppHandle,
    from_label: String,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "swap_to_conversation_dialog: from={from_label}",
    );
    ai_webview::swap_to_conversation_dialog(&app, &from_label)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub fn navigate_ai_webview(
    app: tauri::AppHandle,
    provider_id: String,
    url: String,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "navigate_ai_webview: {provider_id} -> {url}",
    );
    ai_webview::navigate(&app, &provider_id, &url).map_err(Error::Other)
}

#[tauri::command]
pub fn close_ai_webview(app: tauri::AppHandle, provider_id: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "close_ai_webview: {provider_id}",
    );
    ai_webview::close(&app, &provider_id).map_err(Error::Other)?;
    ai_webview::focus_conversation_dialog(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_webview_providers(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<WebviewProvider>> {
    Ok(config.lock().await.settings().webview_providers.clone())
}

#[tauri::command]
pub async fn get_webview_provider(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    provider_id: String,
) -> crate::Result<Option<WebviewProvider>> {
    Ok(config
        .lock()
        .await
        .settings()
        .find_webview_provider(&provider_id)
        .cloned())
}

#[tauri::command]
pub fn get_active_provider(app: tauri::AppHandle, host_label: String) -> Option<String> {
    ai_webview::active_provider_for(&app, &host_label)
}

#[tauri::command]
pub fn take_pending_provider(
    app: tauri::AppHandle,
    host_label: String,
) -> Option<String> {
    let pending = app
        .try_state::<ai_webview::AiWebviewState>()
        .and_then(|s| s.take_pending_provider(&host_label));
    log::debug!(
        target: "app_lib::commands::ai_webview",
        "take_pending_provider: host={host_label} -> {pending:?}",
    );
    pending
}

#[tauri::command]
pub fn new_chat_in_host(app: tauri::AppHandle, host_label: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "new_chat_in_host host={host_label}",
    );
    ai_webview::new_chat_in_host(&app, &host_label).map_err(Error::Other)
}

#[tauri::command]
pub fn reload_active_in_host(app: tauri::AppHandle, host_label: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "reload_active_in_host host={host_label}",
    );
    ai_webview::reload_active_in_host(&app, &host_label).map_err(Error::Other)
}

#[tauri::command]
pub async fn open_palette(app: tauri::AppHandle, host_label: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_palette host={host_label}",
    );
    ai_webview::swap_to_palette(&app, &host_label)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn close_palette(
    app: tauri::AppHandle,
    host_label: String,
    selected_provider_id: Option<String>,
) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "close_palette host={host_label} selected={selected_provider_id:?}",
    );
    ai_webview::swap_from_palette(&app, &host_label, selected_provider_id)
        .await
        .map_err(Error::Other)
}
