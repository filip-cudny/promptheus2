use std::time::Instant;

use serde::Serialize;
use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::{Emitter, Manager, WebviewUrl};

use crate::commands::conversation_dialog::NewConversationEvent;
use crate::models::settings::WebviewProvider;
use crate::services::dialog::{
    attach_undecorated_resize_handler, configure_linux_child_packing, content_layout,
    focus_host_window, is_shell_toolbar_label, LinuxChildRole, SHELL_TOOLBAR_LABEL,
};

use super::lifecycle::{
    enable_media_for_webview, host_logical_size, AI_WEBVIEW_BG, CONVERSATION_DIALOG_LABEL,
    CONVERSATION_DIALOG_TITLE,
};
use super::palette::{handle_router_message, lookup_webview_provider, parse_router_message};
use super::scripts::{initialization_script, reinject_script};
use super::surface::{surface, SurfaceTarget};
use super::tabs::{NativeTabReport, Tab, TabKind, TabsSnapshot};
use super::unload::unload_tab;
use super::AiWebviewState;

pub const ROUTER_SENTINEL: &str = "https://promptheus-ai-webview-router.invalid/";

#[derive(Serialize, Clone)]
struct ChatActivateTabEvent {
    tab_id: String,
}

#[derive(Serialize, Clone)]
struct ChatCloseTabEvent {
    tab_id: String,
    next_tab_id: Option<String>,
}

fn state_of(app: &tauri::AppHandle) -> Result<tauri::State<'_, AiWebviewState>, String> {
    app.try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())
}

fn host_of(app: &tauri::AppHandle) -> Result<tauri::Window, String> {
    app.get_window(CONVERSATION_DIALOG_LABEL)
        .ok_or_else(|| format!("missing host window: {CONVERSATION_DIALOG_LABEL}"))
}

pub fn get_tabs(app: &tauri::AppHandle) -> Result<TabsSnapshot, String> {
    Ok(state_of(app)?.with_tabs(|tabs| tabs.snapshot()))
}

pub(super) fn emit_tabs_changed(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let snapshot = state.with_tabs(|tabs| tabs.snapshot());
    if let Err(e) = app.emit_to(SHELL_TOOLBAR_LABEL, "shell:tabs-changed", snapshot) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit shell:tabs-changed failed: {e}",
        );
    }
}

fn emit_to_chat<S: Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: S) {
    if let Err(e) = app.emit_to(CONVERSATION_DIALOG_LABEL, event, payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit {event} failed: {e}",
        );
    }
}

fn emit_active_changed(app: &tauri::AppHandle, provider_id: Option<&str>) {
    emit_to_chat(
        app,
        "shell:active-changed",
        serde_json::json!({ "provider_id": provider_id }),
    );
}

fn sync_host_title(app: &tauri::AppHandle) {
    let (Some(state), Some(host)) = (
        app.try_state::<AiWebviewState>(),
        app.get_window(CONVERSATION_DIALOG_LABEL),
    ) else {
        return;
    };
    let title = state
        .with_tabs(|tabs| {
            tabs.active_id()
                .and_then(|id| tabs.get(id))
                .map(|tab| tab.title.clone())
        })
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| CONVERSATION_DIALOG_TITLE.to_string());
    if let Err(e) = host.set_title(&title) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_title failed: {e}",
        );
    }
}

fn show_only(host: &tauri::Window, target: &tauri::Webview) -> Result<(), String> {
    target.show().map_err(|e| e.to_string())?;
    for webview in host.webviews() {
        let label = webview.label();
        if label == target.label() || is_shell_toolbar_label(label) {
            continue;
        }
        if let Err(e) = webview.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "hide {label} failed: {e}",
            );
        }
    }
    if let Err(e) = target.set_focus() {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_focus {} failed: {e}",
            target.label(),
        );
    }
    Ok(())
}

pub fn show_native_view(app: &tauri::AppHandle) -> Result<(), String> {
    let state = state_of(app)?;
    let host = host_of(app)?;
    let chat = app
        .get_webview(CONVERSATION_DIALOG_LABEL)
        .ok_or_else(|| format!("missing webview: {CONVERSATION_DIALOG_LABEL}"))?;
    state.with_tabs(|tabs| tabs.set_native_view(Instant::now()));
    show_only(&host, &chat)?;
    sync_host_title(app);
    emit_active_changed(app, None);
    emit_tabs_changed(app);
    Ok(())
}

pub fn reload_active_in_host(app: &tauri::AppHandle) -> Result<(), String> {
    let label = state_of(app)?.active_view_label();
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| format!("missing webview: {label}"))?;
    log::info!(
        target: "app_lib::services::ai_webview",
        "reload_active_in_host: active={label}",
    );
    webview
        .eval("window.location.reload()")
        .map_err(|e| e.to_string())
}

fn deliver_pending_message(webview: &tauri::Webview) {
    let Some(host) = webview.url().ok().and_then(|u| u.host_str().map(str::to_string)) else {
        return;
    };
    let Some(state) = webview.app_handle().try_state::<AiWebviewState>() else {
        return;
    };
    let Some((run, script)) = state.take_delivery(webview.label(), &host) else {
        return;
    };
    log::debug!(
        target: "app_lib::services::ai_webview",
        "delivering pending message: webview={} run={run}",
        webview.label(),
    );
    if let Err(e) = webview.eval(&script) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "pending message eval failed on {}: {e}",
            webview.label(),
        );
    }
}

fn on_child_title_changed(app: &tauri::AppHandle, label: &str, title: &str) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let title = title.trim();
    if title.is_empty() {
        return;
    }
    let is_active = state.with_tabs(|tabs| {
        let tab_id = tabs.by_child_label(label)?.id.clone();
        tabs.set_title(&tab_id, title.to_string());
        Some(tabs.active_id() == Some(tab_id.as_str()))
    });
    log::trace!(
        target: "app_lib::services::ai_webview",
        "child title changed: label={label} title={title:?}",
    );
    let Some(is_active) = is_active else {
        return;
    };
    if is_active {
        sync_host_title(app);
    }
    emit_tabs_changed(app);
}

fn create_provider_child(
    app: &tauri::AppHandle,
    host: &tauri::Window,
    tab: &Tab,
    provider: &WebviewProvider,
) -> Result<String, String> {
    let state = state_of(app)?;
    let child_label = state.child_label_for_new_child(&tab.id, &provider.id);
    let (logical_w, logical_h) = host_logical_size(host)?;
    let (pos, size) = content_layout(logical_w, logical_h);
    let start_url = tab.last_url.as_deref().unwrap_or(provider.url.as_str());
    log::debug!(
        target: "app_lib::services::ai_webview",
        "creating child: tab={} label={child_label} url={start_url} size=({},{})",
        tab.id,
        size.width,
        size.height,
    );
    let content_url = tauri::Url::parse(start_url).map_err(|e| e.to_string())?;
    let init_script = initialization_script(provider);
    let reinject = reinject_script();

    let app_for_nav = app.clone();
    let nav_webview_label = child_label.clone();
    let title_webview_label = child_label.clone();

    let builder = WebviewBuilder::new(&child_label, WebviewUrl::External(content_url))
        .auto_resize()
        .background_color(AI_WEBVIEW_BG)
        .enable_clipboard_access()
        .disable_drag_drop_handler()
        .initialization_script(&init_script)
        .on_navigation(move |url| {
            if !url.as_str().starts_with(ROUTER_SENTINEL) {
                return true;
            }
            log::debug!(
                target: "app_lib::services::ai_webview",
                "on_navigation: router sentinel detected webview={nav_webview_label} url={url}",
            );
            match parse_router_message(url) {
                Some(msg) => {
                    let app = app_for_nav.clone();
                    let label = nav_webview_label.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        tauri::async_runtime::block_on(async move {
                            handle_router_message(&app, &label, msg).await;
                        });
                    });
                }
                None => {
                    log::warn!(
                        target: "app_lib::services::ai_webview",
                        "unparseable router url: {url}",
                    );
                }
            }
            false
        })
        .on_document_title_changed(move |webview, title| {
            on_child_title_changed(webview.app_handle(), &title_webview_label, &title);
        })
        .on_page_load(move |webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
                log::debug!(
                    target: "app_lib::services::ai_webview",
                    "on_page_load Finished: webview={} url={url}",
                    webview.label(),
                );
                if let Some(state) = webview.app_handle().try_state::<AiWebviewState>() {
                    state.set_last_page_url(webview.label(), url);
                }
                if let Err(e) = webview.eval(&reinject) {
                    log::warn!(
                        target: "app_lib::services::ai_webview",
                        "failed to re-inject keybind script: {e}",
                    );
                }
                deliver_pending_message(&webview);
            }
        });

    state.with_tabs(|tabs| tabs.set_live(&tab.id, Some(child_label.clone())));
    let child_webview = match host.add_child(builder, pos, size) {
        Ok(w) => w,
        Err(e) => {
            state.with_tabs(|tabs| tabs.set_live(&tab.id, None));
            log::error!(
                target: "app_lib::services::ai_webview",
                "add_child FAILED tab={} label={child_label}: {e}",
                tab.id,
            );
            return Err(e.to_string());
        }
    };
    enable_media_for_webview(&child_webview);
    configure_linux_child_packing(&child_webview, LinuxChildRole::Content);
    if cfg!(target_os = "linux") {
        attach_undecorated_resize_handler(&child_webview);
    }
    log::info!(
        target: "app_lib::services::ai_webview",
        "child created: tab={} provider={} label={child_label}",
        tab.id,
        provider.id,
    );
    Ok(child_label)
}

fn ensure_live_child(
    app: &tauri::AppHandle,
    host: &tauri::Window,
    tab: &Tab,
    provider: &WebviewProvider,
) -> Result<String, String> {
    if let Some(label) = &tab.child_label {
        return Ok(label.clone());
    }
    let state = state_of(app)?;
    let armed = state.armed_tab_ids();
    if let Some(victim) = state.with_tabs(|tabs| tabs.limit_unload_victim(&tab.id, &armed)) {
        log::info!(
            target: "app_lib::services::ai_webview",
            "live limit: unloading tab={victim} for tab={}",
            tab.id,
        );
        unload_tab(app, &victim);
    }
    create_provider_child(app, host, tab, provider)
}

pub fn activate_tab(app: &tauri::AppHandle, tab_id: &str) -> Result<(), String> {
    let state = state_of(app)?;
    let host = host_of(app)?;
    let tab = state
        .with_tabs(|tabs| tabs.get(tab_id).cloned())
        .ok_or_else(|| format!("unknown tab: {tab_id}"))?;

    let (target_label, provider_id, native_tab_id) = match &tab.kind {
        TabKind::Native { tab_id: native } => {
            (CONVERSATION_DIALOG_LABEL.to_string(), None, Some(native.clone()))
        }
        TabKind::Provider { provider } => (
            ensure_live_child(app, &host, &tab, provider)?,
            Some(provider.id.clone()),
            None,
        ),
    };
    let target = app
        .get_webview(&target_label)
        .ok_or_else(|| format!("missing webview: {target_label}"))?;

    show_only(&host, &target)?;
    state.with_tabs(|tabs| tabs.activate(tab_id, Instant::now()));
    sync_host_title(app);
    if let Some(native_tab_id) = native_tab_id {
        emit_to_chat(app, "chat:activate-tab", ChatActivateTabEvent { tab_id: native_tab_id });
    }
    emit_active_changed(app, provider_id.as_deref());
    emit_tabs_changed(app);
    log::info!(
        target: "app_lib::services::ai_webview",
        "activated tab={tab_id} view={target_label}",
    );
    focus_host_window(app, CONVERSATION_DIALOG_LABEL)
}

pub fn close_tab(app: &tauri::AppHandle, tab_id: &str) -> Result<(), String> {
    let state = state_of(app)?;
    let tab = state
        .with_tabs(|tabs| tabs.get(tab_id).cloned())
        .ok_or_else(|| format!("unknown tab: {tab_id}"))?;
    if matches!(tab.kind, TabKind::Provider { .. }) {
        unload_tab(app, tab_id);
    }
    let outcome = state.with_tabs(|tabs| tabs.close(tab_id, Instant::now()));
    log::info!(
        target: "app_lib::services::ai_webview",
        "closed tab={tab_id} successor={:?} list_empty={}",
        outcome.successor,
        outcome.list_empty,
    );
    let close_event = match &tab.kind {
        TabKind::Native { tab_id: native } => {
            let successor_tab = outcome
                .successor
                .as_deref()
                .and_then(|id| state.with_tabs(|tabs| tabs.get(id).cloned()));
            let next_tab_id = successor_tab.as_ref().and_then(|next| match &next.kind {
                TabKind::Native { tab_id } => Some(tab_id.clone()),
                TabKind::Provider { .. } => None,
            });
            let provider_successor = successor_tab.is_some() && next_tab_id.is_none();
            Some((ChatCloseTabEvent { tab_id: native.clone(), next_tab_id }, provider_successor))
        }
        TabKind::Provider { .. } => None,
    };
    match (close_event, &outcome.successor) {
        (Some((event, true)), Some(successor)) => {
            let activated = activate_tab(app, successor);
            emit_to_chat(app, "chat:close-tab", event);
            return activated;
        }
        (Some((event, _)), _) => emit_to_chat(app, "chat:close-tab", event),
        (None, _) => {}
    }
    if outcome.list_empty {
        emit_tabs_changed(app);
        return host_of(app)?.close().map_err(|e| e.to_string());
    }
    match outcome.successor {
        Some(successor) => activate_tab(app, &successor),
        None => {
            emit_tabs_changed(app);
            Ok(())
        }
    }
}

pub async fn new_tab(app: &tauri::AppHandle, provider_id: &str) -> Result<(), String> {
    if provider_id == super::palette::PROMPTHEUS_PROVIDER_ID {
        surface(app, "new_tab", SurfaceTarget::NativeView).await?;
        emit_to_chat(app, "new-conversation", NewConversationEvent { reason: "tab".into() });
        return Ok(());
    }
    let provider = lookup_webview_provider(app, provider_id)
        .await
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    surface(app, "new_tab", SurfaceTarget::NewProviderTab(provider)).await
}

pub fn activate_adjacent_tab(app: &tauri::AppHandle, step: i32) -> Result<(), String> {
    let target = state_of(app)?.with_tabs(|tabs| tabs.adjacent(step));
    log::debug!(target: "app_lib::services::ai_webview", "adjacent tab step={step} target={target:?}");
    target.map_or(Ok(()), |id| activate_tab(app, &id))
}

pub fn activate_tab_at(app: &tauri::AppHandle, position: u32) -> Result<(), String> {
    let target = state_of(app)?.with_tabs(|tabs| tabs.at_position(position));
    log::debug!(target: "app_lib::services::ai_webview", "tab at position={position} target={target:?}");
    target.map_or(Ok(()), |id| activate_tab(app, &id))
}

pub fn close_active_tab(app: &tauri::AppHandle) -> Result<(), String> {
    let active = state_of(app)?.with_tabs(|tabs| tabs.active_id().map(str::to_string));
    log::debug!(target: "app_lib::services::ai_webview", "close active tab={active:?}");
    active.map_or(Ok(()), |id| close_tab(app, &id))
}

pub async fn new_tab_like_active(app: &tauri::AppHandle) -> Result<(), String> {
    let state = state_of(app)?;
    let (has_tabs, provider_id) = state.with_tabs(|tabs| {
        let provider_id = tabs
            .active_id()
            .and_then(|id| tabs.get(id))
            .and_then(|tab| match &tab.kind {
                TabKind::Provider { provider } => Some(provider.id.clone()),
                TabKind::Native { .. } => None,
            });
        (!tabs.tabs().is_empty(), provider_id)
    });
    if !has_tabs {
        return Ok(());
    }
    let provider_id = provider_id.unwrap_or_else(|| super::palette::PROMPTHEUS_PROVIDER_ID.to_string());
    log::debug!(target: "app_lib::services::ai_webview", "new tab like active provider={provider_id}");
    new_tab(app, &provider_id).await
}

pub fn report_native_tabs(
    app: &tauri::AppHandle,
    reports: &[NativeTabReport],
    active_tab_id: Option<&str>,
) -> Result<(), String> {
    state_of(app)?.with_tabs(|tabs| {
        tabs.apply_native_snapshot(reports, active_tab_id, Instant::now());
    });
    sync_host_title(app);
    emit_tabs_changed(app);
    Ok(())
}
