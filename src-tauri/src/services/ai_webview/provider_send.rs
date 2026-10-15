use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::commands::conversation_dialog::conversation_dialog_config;
use crate::models::context::ContextItem;
use crate::models::settings::WebviewProvider;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::dialog;
use crate::services::notification::{NotificationLevel, NotificationService};

use super::lifecycle::CONVERSATION_DIALOG_LABEL;
use super::palette::{lookup_webview_provider, SendStatus};
use super::provider_swap::{hosted_child_label, new_chat_in_host, swap_to_provider};
use super::scripts::delivery_script;
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
    let child_label = hosted_child_label(CONVERSATION_DIALOG_LABEL, &provider);
    let mut message = Some(PendingMessage {
        run,
        transcript: transcript.clone(),
        script,
        host,
        delivered: false,
    });

    let result = open_chat(app, &provider, &child_label, &mut message).await;
    if let Err(reason) = result {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "send_to_provider_chat failed: provider={} run={run}: {reason}",
            provider.id,
        );
        if message.is_none() {
            if let Some(state) = app.try_state::<AiWebviewState>() {
                state.finish_message(&child_label, run);
            }
        }
        fallback(app, &provider.id, &provider.name, Some(run), transcript, &reason).await;
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
    let provider_id = state.current_provider_for(webview_label).unwrap_or_default();
    let provider_name = state
        .snapshot(webview_label)
        .map(|p| p.name)
        .unwrap_or_else(|| provider_id.clone());
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

async fn open_chat(
    app: &tauri::AppHandle,
    provider: &WebviewProvider,
    child_label: &str,
    message: &mut Option<PendingMessage>,
) -> Result<(), String> {
    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;

    if app.get_window(CONVERSATION_DIALOG_LABEL).is_none() {
        state.set_pending_provider(CONVERSATION_DIALOG_LABEL, &provider.id);
        dialog::open_or_focus(app, &conversation_dialog_config()).await?;
        arm(app, provider, child_label, message).await;
        return Ok(());
    }

    let created = app.get_webview(child_label).is_some();
    state.take_suspended_url(child_label);
    if !created {
        arm(app, provider, child_label, message).await;
    }
    swap_to_provider(app, provider.clone(), CONVERSATION_DIALOG_LABEL).await?;
    if created {
        new_chat_in_host(app, CONVERSATION_DIALOG_LABEL)?;
        arm(app, provider, child_label, message).await;
    }
    Ok(())
}

async fn arm(
    app: &tauri::AppHandle,
    provider: &WebviewProvider,
    child_label: &str,
    message: &mut Option<PendingMessage>,
) {
    let Some(message) = message.take() else {
        return;
    };
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let run = message.run;
    if let Some(replaced) = state.arm_message(child_label, message) {
        fallback(
            app,
            &provider.id,
            &provider.name,
            Some(replaced.run),
            replaced.transcript,
            "replaced by a newer message",
        )
        .await;
    }
    spawn_deadline(app.clone(), provider.clone(), child_label.to_string(), run);
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

async fn fallback(
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
