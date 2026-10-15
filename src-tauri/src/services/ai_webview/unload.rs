use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::Manager;
use tokio::sync::Mutex;

use crate::services::config::ConfigService;
use crate::services::notification::{NotificationLevel, NotificationService};

use super::provider_send::fallback;
use super::provider_swap::emit_tabs_changed;
use super::AiWebviewState;

pub const UNLOAD_POLL_INTERVAL: Duration = Duration::from_secs(60);
pub(super) const UNRESPONSIVE_GRACE_SECONDS: u32 = 20;

pub(super) fn unload_tab(app: &tauri::AppHandle, tab_id: &str) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let Some(label) = state.with_tabs(|tabs| tabs.get(tab_id).and_then(|t| t.child_label.clone()))
    else {
        return;
    };
    let provider = state.provider_for_label(&label);
    let webview = app.get_webview(&label);
    let url = webview
        .as_ref()
        .and_then(|w| w.url().ok())
        .map(|u| u.to_string());
    log::debug!(
        target: "app_lib::services::ai_webview",
        "unload_tab: tab={tab_id} label={label} url={url:?}",
    );
    let message = state.mark_unloaded(tab_id, url);
    if let Some(webview) = webview {
        if let Err(e) = webview.close() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "unload_tab: close failed for {label}: {e}",
            );
        }
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "unloaded tab={tab_id} label={label}",
    );
    if let (Some(message), Some(provider)) = (message, provider) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            fallback(
                &app,
                &provider.id,
                &provider.name,
                Some(message.run),
                message.transcript,
                "tab unloaded",
            )
            .await;
        });
    }
    emit_tabs_changed(app);
}

pub fn run_unload_pass(app: tauri::AppHandle) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let armed = state.armed_tab_ids();
    let candidates = state.with_tabs(|tabs| tabs.idle_unload_candidates(Instant::now(), &armed));
    if candidates.is_empty() {
        return;
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "unload pass: candidates={candidates:?}",
    );
    for tab_id in candidates {
        unload_tab(&app, &tab_id);
    }
}

pub(super) fn provider_display_name(app: &tauri::AppHandle, webview_label: &str) -> String {
    app.try_state::<AiWebviewState>()
        .and_then(|s| s.provider_for_label(webview_label))
        .map(|p| p.name)
        .unwrap_or_else(|| "Provider".to_string())
}

pub(super) fn emit_provider_lifecycle_toast(
    app: &tauri::AppHandle,
    event_name: &'static str,
    level: NotificationLevel,
    title: String,
    message: Option<String>,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let settings = app
            .state::<Arc<Mutex<ConfigService>>>()
            .lock()
            .await
            .settings()
            .notifications
            .clone();
        if let Err(e) = app.state::<NotificationService>().notify(
            event_name,
            level,
            title,
            message,
            &settings,
        ) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "lifecycle toast emit failed: {e}",
            );
        }
    });
}
