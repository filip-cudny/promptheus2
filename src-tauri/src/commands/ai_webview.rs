use crate::services::ai_providers::{self, AiProviderDto, PROVIDERS};
use crate::services::ai_webview;

#[tauri::command]
pub async fn open_ai_webview(
    app: tauri::AppHandle,
    provider_id: String,
    url: Option<String>,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_ai_webview: {provider_id} url={url:?}",
    );
    let provider = ai_providers::find(&provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    ai_webview::open_or_focus(&app, provider, url).await
}

#[tauri::command]
pub async fn open_ai_webview_new_window(
    app: tauri::AppHandle,
    provider_id: String,
    url: Option<String>,
    source_label: Option<String>,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_ai_webview_new_window: {provider_id} url={url:?} source={source_label:?}",
    );
    let provider = ai_providers::find(&provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    ai_webview::open_new_instance(&app, provider, url, source_label).await
}

#[tauri::command]
pub async fn swap_ai_webview(
    app: tauri::AppHandle,
    provider_id: String,
    from_label: String,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "swap_ai_webview: {provider_id} from={from_label}",
    );
    let provider = ai_providers::find(&provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    ai_webview::swap_to_provider(&app, provider, &from_label).await
}

#[tauri::command]
pub async fn swap_to_conversation_dialog(
    app: tauri::AppHandle,
    from_label: String,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "swap_to_conversation_dialog: from={from_label}",
    );
    ai_webview::swap_to_conversation_dialog(&app, &from_label).await
}

#[tauri::command]
pub fn navigate_ai_webview(
    app: tauri::AppHandle,
    provider_id: String,
    url: String,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "navigate_ai_webview: {provider_id} -> {url}",
    );
    ai_webview::navigate(&app, &provider_id, &url)
}

#[tauri::command]
pub fn close_ai_webview(app: tauri::AppHandle, provider_id: String) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "close_ai_webview: {provider_id}",
    );
    let provider = ai_providers::find(&provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    ai_webview::close(&app, provider)?;
    ai_webview::focus_conversation_dialog(&app);
    Ok(())
}

#[tauri::command]
pub fn get_ai_providers() -> Vec<AiProviderDto> {
    PROVIDERS.iter().map(AiProviderDto::from).collect()
}

#[tauri::command]
pub fn get_ai_provider(provider_id: String) -> Option<AiProviderDto> {
    ai_providers::find(&provider_id).map(AiProviderDto::from)
}

#[tauri::command]
pub fn get_active_provider(app: tauri::AppHandle, host_label: String) -> Option<String> {
    ai_webview::active_provider_for(&app, &host_label)
}

#[tauri::command]
pub fn new_chat_in_host(app: tauri::AppHandle, host_label: String) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "new_chat_in_host host={host_label}",
    );
    ai_webview::new_chat_in_host(&app, &host_label)
}

#[tauri::command]
pub async fn open_palette(app: tauri::AppHandle, host_label: String) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_palette host={host_label}",
    );
    ai_webview::swap_to_palette(&app, &host_label).await
}

#[tauri::command]
pub async fn close_palette(
    app: tauri::AppHandle,
    host_label: String,
    selected_provider_id: Option<String>,
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "close_palette host={host_label} selected={selected_provider_id:?}",
    );
    ai_webview::swap_from_palette(&app, &host_label, selected_provider_id).await
}
