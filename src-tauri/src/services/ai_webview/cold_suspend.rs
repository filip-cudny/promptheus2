use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::services::config::ConfigService;
use crate::services::notification::{NotificationLevel, NotificationService};

use super::AiWebviewState;

const COLD_SUSPEND_IDLE_THRESHOLD: Duration = Duration::from_secs(90 * 60);
pub const COLD_SUSPEND_POLL_INTERVAL: Duration = Duration::from_secs(60);
pub(super) const UNRESPONSIVE_GRACE_SECONDS: u32 = 20;

pub(super) fn is_suspendable_webview_label(label: &str) -> bool {
    label.starts_with("ai-webview-") || label.contains("::ai-webview-")
}

pub(super) fn resume_if_suspended(app: &tauri::AppHandle, webview_label: &str) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    if !state.is_suspended(webview_label) {
        return;
    }
    let Some(webview) = app.get_webview(webview_label) else {
        state.unmark_suspended(webview_label);
        return;
    };
    let restore_url = state
        .take_suspended_url(webview_label)
        .or_else(|| state.snapshot(webview_label).map(|p| p.url));
    let Some(url_str) = restore_url else {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "resume_if_suspended: no restore URL for {webview_label}",
        );
        return;
    };
    let url = match tauri::Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "resume_if_suspended: invalid restore URL '{url_str}' for {webview_label}: {e}",
            );
            return;
        }
    };
    if let Err(e) = webview.navigate(url) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "resume_if_suspended: navigate failed for {webview_label}: {e}",
        );
        return;
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "resumed suspended webview: {webview_label} -> {url_str}",
    );
}

pub fn run_cold_suspend_pass(app: tauri::AppHandle) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let candidates = state.cold_suspend_candidates(COLD_SUSPEND_IDLE_THRESHOLD);
    if candidates.is_empty() {
        return;
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "cold-suspend pass: candidates={candidates:?}",
    );
    for label in candidates {
        cold_suspend_one(&app, &label);
    }
}

fn cold_suspend_one(app: &tauri::AppHandle, webview_label: &str) {
    let Some(webview) = app.get_webview(webview_label) else {
        if let Some(state) = app.try_state::<AiWebviewState>() {
            state.unmark_suspended(webview_label);
        }
        return;
    };
    let current_url = match webview.url() {
        Ok(u) => u.to_string(),
        Err(e) => {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "cold-suspend: cannot read current URL for {webview_label}: {e}",
            );
            return;
        }
    };
    let blank = match tauri::Url::parse("about:blank") {
        Ok(u) => u,
        Err(_) => return,
    };
    if let Err(e) = webview.navigate(blank) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "cold-suspend: navigate to about:blank failed for {webview_label}: {e}",
        );
        return;
    }
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.mark_suspended(webview_label, current_url.clone());
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "cold-suspend: webview={webview_label} parked (saved {current_url})",
    );
}

pub(super) fn provider_display_name(app: &tauri::AppHandle, webview_label: &str) -> String {
    app.try_state::<AiWebviewState>()
        .and_then(|s| s.snapshot(webview_label))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspendable_label_matches_prefix() {
        assert!(is_suspendable_webview_label("ai-webview-claude"));
        assert!(is_suspendable_webview_label(
            "conversation-dialog::ai-webview-claude"
        ));
    }

    #[test]
    fn suspendable_label_rejects_unrelated() {
        assert!(!is_suspendable_webview_label("conversation-dialog"));
        assert!(!is_suspendable_webview_label("palette"));
        assert!(!is_suspendable_webview_label("shell-toolbar-foo"));
    }
}
