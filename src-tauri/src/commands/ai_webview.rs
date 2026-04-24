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
) -> Result<(), String> {
    log::info!(
        target: "app_lib::commands::ai_webview",
        "open_ai_webview_new_window: {provider_id} url={url:?}",
    );
    let provider = ai_providers::find(&provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    ai_webview::open_new_instance(&app, provider, url).await
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
