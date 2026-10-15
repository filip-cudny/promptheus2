use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::models::context::ContextItem;
use crate::models::settings::WebviewProvider;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::notification::{NotificationLevel, NotificationService};

use super::palette::{lookup_webview_provider, SendStatus};
use super::provider_swap::activate_tab;
use super::scripts::delivery_script;
use super::surface::{surface, SurfaceTarget};
use super::{AiWebviewState, PendingMessage};

const DELIVERY_DEADLINE: Duration = Duration::from_secs(80);

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

pub async fn send_to_provider_chat(app: &tauri::AppHandle, provider_id: &str, transcript: String) {
    let Some(provider) = lookup_webview_provider(app, provider_id).await else {
        fallback(app, provider_id, provider_id, None, transcript, "provider not configured").await;
        return;
    };
    let Some(host) = tauri::Url::parse(&provider.url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
    else {
        fallback(app, &provider.id, &provider.name, None, transcript, "invalid provider url").await;
        return;
    };

    let (context_text, images) = read_context(app).await;
    let run = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    let script = delivery_script(run, &transcript, context_text.as_deref(), &images);

    if let Err(reason) = surface(app, "pill_webview_target", SurfaceTarget::KeepActive).await {
        fallback(app, &provider.id, &provider.name, Some(run), transcript, &reason).await;
        return;
    }
    let Some(state) = app.try_state::<AiWebviewState>() else {
        fallback(app, &provider.id, &provider.name, Some(run), transcript, "ai webview state missing").await;
        return;
    };

    let message = PendingMessage {
        run,
        transcript: transcript.clone(),
        script,
        host,
        delivered: false,
    };
    let (tab_id, child_label) = state.arm_in_new_tab(&provider, message);
    log::info!(
        target: "app_lib::services::ai_webview",
        "pill message armed: provider={} run={run} tab={tab_id} label={child_label}",
        provider.id,
    );
    spawn_deadline(app.clone(), provider.clone(), child_label.clone(), run);

    if let Err(reason) = activate_tab(app, &tab_id) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "send_to_provider_chat failed: provider={} run={run}: {reason}",
            provider.id,
        );
        if state.finish_message(&child_label, run).is_some() {
            fallback(app, &provider.id, &provider.name, Some(run), transcript, &reason).await;
        }
    }
}

pub(super) async fn handle_send_outcome(
    app: &tauri::AppHandle,
    webview_label: &str,
    status: SendStatus,
    run: u64,
    uploads: Option<u32>,
    waited_ms: Option<u64>,
) {
    log::info!(
        target: "app_lib::services::ai_webview",
        "send outcome: webview={webview_label} run={run} status={status:?} uploads={uploads:?} waited_ms={waited_ms:?}",
    );
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let Some(message) = state.finish_message(webview_label, run) else {
        log::debug!(
            target: "app_lib::services::ai_webview",
            "send outcome without pending message: webview={webview_label} run={run}",
        );
        return;
    };
    let reason = match status {
        SendStatus::Sent => return,
        SendStatus::Typed => "typed but not sent",
        SendStatus::NoComposer => "no message box found (logged out?)",
    };
    let (provider_id, provider_name) = state
        .provider_for_label(webview_label)
        .map(|p| (p.id, p.name))
        .unwrap_or_default();
    fallback(app, &provider_id, &provider_name, Some(run), message.transcript, reason).await;
}

async fn read_context(app: &tauri::AppHandle) -> (Option<String>, Vec<(String, String)>) {
    let context = app.state::<Arc<Mutex<ContextManagerService>>>();
    let context = context.lock().await;
    let images = context
        .get_items()
        .into_iter()
        .filter_map(|item| match item {
            ContextItem::Image { data, media_type, .. } => Some((media_type, data)),
            _ => None,
        })
        .collect();
    (context.labelled_context(), images)
}

fn spawn_deadline(app: tauri::AppHandle, provider: WebviewProvider, child_label: String, run: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DELIVERY_DEADLINE).await;
        let Some(state) = app.try_state::<AiWebviewState>() else {
            return;
        };
        let Some(message) = state.finish_message(&child_label, run) else {
            return;
        };
        fallback(
            &app,
            &provider.id,
            &provider.name,
            Some(run),
            message.transcript,
            "the page did not answer",
        )
        .await;
    });
}

pub(super) async fn fallback(
    app: &tauri::AppHandle,
    provider_id: &str,
    provider_name: &str,
    run: Option<u64>,
    transcript: String,
    reason: &str,
) {
    log::warn!(
        target: "app_lib::services::ai_webview",
        "message not sent: provider={provider_id} run={run:?}: {reason}",
    );
    let clipboard_app = app.clone();
    let copied = tauri::async_runtime::spawn_blocking(move || {
        crate::services::clipboard::write_text(&clipboard_app, &transcript)
    })
    .await;
    let outcome = match copied {
        Ok(Ok(())) => "transcript copied",
        _ => "copying the transcript failed",
    };
    let settings = app
        .state::<Arc<Mutex<ConfigService>>>()
        .lock()
        .await
        .settings()
        .notifications
        .clone();
    let title = if provider_name.is_empty() { provider_id } else { provider_name };
    if let Err(e) = app.state::<NotificationService>().notify(
        "webview_send_failed",
        NotificationLevel::Error,
        format!("Not sent to {title}"),
        Some(format!("{reason} — {outcome}")),
        &settings,
    ) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "send fallback toast failed: {e}",
        );
    }
}
