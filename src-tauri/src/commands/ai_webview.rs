use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::ai_webview::{self, NativeTabReport, TabsSnapshot};
use crate::services::config::ConfigService;

#[tauri::command]
pub async fn get_webview_providers(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<WebviewProvider>> {
    Ok(config.lock().await.settings().webview_providers.clone())
}

#[tauri::command(async)]
pub fn reload_active_in_host(app: tauri::AppHandle) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "reload_active_in_host");
    Ok(ai_webview::reload_active_in_host(&app)?)
}

#[tauri::command]
pub async fn open_palette(app: tauri::AppHandle, host_label: String) -> crate::Result<()> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_palette host={host_label}",
    );
    Ok(ai_webview::swap_to_palette(&app, &host_label).await?)
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
    Ok(ai_webview::swap_from_palette(&app, &host_label, selected_provider_id).await?)
}

#[tauri::command]
pub fn get_tabs(app: tauri::AppHandle) -> crate::Result<TabsSnapshot> {
    Ok(ai_webview::get_tabs(&app)?)
}

#[tauri::command(async)]
pub fn activate_tab(app: tauri::AppHandle, tab_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "activate_tab tab={tab_id}");
    Ok(ai_webview::activate_tab(&app, &tab_id)?)
}

#[tauri::command(async)]
pub fn close_tab(app: tauri::AppHandle, tab_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "close_tab tab={tab_id}");
    Ok(ai_webview::close_tab(&app, &tab_id)?)
}

#[tauri::command(async)]
pub fn activate_adjacent_tab(app: tauri::AppHandle, step: i32) -> crate::Result<()> {
    Ok(ai_webview::activate_adjacent_tab(&app, step)?)
}

#[tauri::command(async)]
pub fn activate_tab_at(app: tauri::AppHandle, position: u32) -> crate::Result<()> {
    Ok(ai_webview::activate_tab_at(&app, position)?)
}

#[tauri::command(async)]
pub fn close_active_tab(app: tauri::AppHandle) -> crate::Result<()> {
    Ok(ai_webview::close_active_tab(&app)?)
}

#[tauri::command]
pub async fn new_tab_like_active(app: tauri::AppHandle) -> crate::Result<()> {
    Ok(ai_webview::new_tab_like_active(&app).await?)
}

#[tauri::command]
pub async fn new_tab(app: tauri::AppHandle, provider_id: String) -> crate::Result<()> {
    log::info!(target: "app_lib::commands::ai_webview", "new_tab provider={provider_id}");
    Ok(ai_webview::new_tab(&app, &provider_id).await?)
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
    Ok(ai_webview::report_native_tabs(&app, &tabs, active_tab_id.as_deref())?)
}
