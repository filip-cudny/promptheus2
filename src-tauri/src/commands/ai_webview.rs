use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::ai_webview::{self, NativeTabReport, TabsSnapshot};
use crate::services::config::ConfigService;
use crate::Error;

#[tauri::command]
pub async fn get_webview_providers(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<WebviewProvider>> {
    Ok(config.lock().await.settings().webview_providers.clone())
}

#[tauri::command(async)]
pub fn reload_active_in_host(app: tauri::AppHandle, host_label: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "reload_active_in_host host={host_label}",
    );
    ai_webview::reload_active_in_host(&app).map_err(Error::Other)
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

#[tauri::command]
pub fn get_tabs(app: tauri::AppHandle) -> crate::Result<TabsSnapshot> {
    ai_webview::get_tabs(&app).map_err(Error::Other)
}

#[tauri::command(async)]
pub fn activate_tab(app: tauri::AppHandle, tab_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "activate_tab tab={tab_id}");
    ai_webview::activate_tab(&app, &tab_id).map_err(Error::Other)
}

#[tauri::command(async)]
pub fn close_tab(app: tauri::AppHandle, tab_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "close_tab tab={tab_id}");
    ai_webview::close_tab(&app, &tab_id).map_err(Error::Other)
}

#[tauri::command(async)]
pub fn activate_adjacent_tab(app: tauri::AppHandle, step: i32) -> crate::Result<()> {
    ai_webview::activate_adjacent_tab(&app, step).map_err(Error::Other)
}

#[tauri::command(async)]
pub fn activate_tab_at(app: tauri::AppHandle, position: u32) -> crate::Result<()> {
    ai_webview::activate_tab_at(&app, position).map_err(Error::Other)
}

#[tauri::command(async)]
pub fn close_active_tab(app: tauri::AppHandle) -> crate::Result<()> {
    ai_webview::close_active_tab(&app).map_err(Error::Other)
}

#[tauri::command]
pub async fn new_tab_like_active(app: tauri::AppHandle) -> crate::Result<()> {
    ai_webview::new_tab_like_active(&app)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn new_tab(app: tauri::AppHandle, provider_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "new_tab provider={provider_id}");
    ai_webview::new_tab(&app, &provider_id)
        .await
        .map_err(Error::Other)
}

#[tauri::command]
pub fn report_native_tabs(
    app: tauri::AppHandle,
    tabs: Vec<NativeTabReport>,
    active_tab_id: Option<String>,
) -> crate::Result<()> {
    log::trace!(
        target: "app_lib::commands::ai_webview",
        "report_native_tabs count={} active={active_tab_id:?}",
        tabs.len(),
    );
    ai_webview::report_native_tabs(&app, &tabs, active_tab_id.as_deref()).map_err(Error::Other)
}
