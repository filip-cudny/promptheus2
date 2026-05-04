use std::sync::Arc;

use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::{Emitter, Manager, WebviewUrl};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::dialog::{
    attach_undecorated_resize_handler, configure_linux_child_packing, content_layout,
    focus_host_window, focus_window, is_conversation_dialog_host, is_shell_toolbar_label,
    shell_toolbar_label_for, uses_custom_titlebar, LinuxChildRole,
};
use crate::services::ui_state::UiStateService;

use super::cold_suspend::resume_if_suspended;
use super::lifecycle::{
    close_by_label, enable_media_for_webview, host_logical_size, open_or_focus,
    read_window_geometry, swap_to_conversation_dialog_standalone, AI_WEBVIEW_BG,
    CONVERSATION_DIALOG_TITLE,
};
use super::palette::{handle_router_message, parse_router_message};
use super::scripts::{initialization_script, reinject_script};
use super::AiWebviewState;

pub const ROUTER_SENTINEL: &str = "https://promptheus-ai-webview-router.invalid/";

pub fn window_label(provider: &WebviewProvider) -> String {
    format!("ai-webview-{}", provider.id)
}

fn hosted_child_label(host_label: &str, provider: &WebviewProvider) -> String {
    format!("{host_label}::ai-webview-{}", provider.id)
}

fn is_toolbar_label(label: &str) -> bool {
    is_shell_toolbar_label(label)
}

pub fn active_provider_for(app: &tauri::AppHandle, host_label: &str) -> Option<String> {
    let state = app.try_state::<AiWebviewState>()?;
    let active = state.get_active(host_label)?;
    if active == host_label {
        return None;
    }
    state.current_provider_for(&active)
}

pub fn new_chat_in_host(app: &tauri::AppHandle, host_label: &str) -> Result<(), String> {
    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;
    let active_label = state
        .get_active(host_label)
        .ok_or_else(|| "no active webview".to_string())?;
    if active_label == host_label {
        if app.get_webview_window(host_label).is_none() {
            return Err(format!("missing host window: {host_label}"));
        }
        return app
            .emit_to(
                host_label,
                "new-conversation",
                crate::commands::conversation_dialog::NewConversationEvent {
                    reason: "shell".into(),
                },
            )
            .map_err(|e| e.to_string());
    }
    let provider = state
        .snapshot(&active_label)
        .ok_or_else(|| format!("no provider snapshot for {active_label}"))?;
    let webview = app
        .get_webview(&active_label)
        .ok_or_else(|| format!("missing webview: {active_label}"))?;
    let parsed = tauri::Url::parse(&provider.url).map_err(|e| e.to_string())?;
    webview.navigate(parsed).map_err(|e| e.to_string())
}

pub fn reload_active_in_host(app: &tauri::AppHandle, host_label: &str) -> Result<(), String> {
    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;
    let active_label = state
        .get_active(host_label)
        .unwrap_or_else(|| host_label.to_string());
    let webview = app
        .get_webview(&active_label)
        .ok_or_else(|| format!("missing webview: {active_label}"))?;
    log::info!(
        target: "app_lib::services::ai_webview",
        "reload_active_in_host: host={host_label} active={active_label}",
    );
    webview
        .eval("window.location.reload()")
        .map_err(|e| e.to_string())
}

pub async fn swap_to_provider(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
    from_label: &str,
) -> Result<(), String> {
    if is_conversation_dialog_host(from_label) && app.get_window(from_label).is_some() {
        return hosted_swap_to_provider(app, from_label, provider).await;
    }

    swap_to_provider_standalone(app, provider, from_label).await
}

pub async fn swap_to_conversation_dialog(
    app: &tauri::AppHandle,
    from_label: &str,
) -> Result<(), String> {
    if is_conversation_dialog_host(from_label) && app.get_window(from_label).is_some() {
        return hosted_swap_to_conversation_dialog(app, from_label);
    }

    swap_to_conversation_dialog_standalone(app, from_label).await
}

pub(super) async fn hosted_swap_to_provider(
    app: &tauri::AppHandle,
    host_label: &str,
    provider: WebviewProvider,
) -> Result<(), String> {
    log::debug!(
        target: "app_lib::services::ai_webview",
        "hosted_swap_to_provider: ENTER host={host_label} provider_id={} provider_url={}",
        provider.id,
        provider.url,
    );

    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let pre_webviews: Vec<String> = host.webviews().iter().map(|w| w.label().to_string()).collect();
    log::debug!(
        target: "app_lib::services::ai_webview",
        "hosted_swap_to_provider: pre-state host={host_label} webviews={pre_webviews:?}",
    );

    let child_label = hosted_child_label(host_label, &provider);

    let state_says_created = app
        .try_state::<AiWebviewState>()
        .map(|s| s.has_hosted_child(host_label, &provider.id))
        .unwrap_or(false);
    let webview_exists = app.get_webview(&child_label).is_some();
    let already_created = webview_exists;

    if state_says_created && !webview_exists {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "hosted_swap_to_provider: stale hosted-state for {child_label} (state says created but webview missing) — clearing",
        );
        if let Some(s) = app.try_state::<AiWebviewState>() {
            s.unmark_hosted_child(host_label, &provider.id);
            s.remove_provider(&child_label);
        }
    }

    log::debug!(
        target: "app_lib::services::ai_webview",
        "hosted_swap_to_provider: child_label={child_label} already_created={already_created} (state_says={state_says_created}, webview_exists={webview_exists})",
    );

    if !already_created {
        let (logical_w, logical_h) = host_logical_size(&host)?;
        let (pos, size) = content_layout(logical_w, logical_h);
        log::debug!(
            target: "app_lib::services::ai_webview",
            "hosted_swap_to_provider: creating child host_size=({logical_w},{logical_h}) pos=({},{}) size=({},{})",
            pos.x, pos.y, size.width, size.height,
        );
        let content_url = tauri::Url::parse(&provider.url).map_err(|e| e.to_string())?;
        let init_script = initialization_script(&provider);
        let reinject = reinject_script();

        let app_for_nav = app.clone();
        let nav_webview_label = child_label.clone();

        let builder = WebviewBuilder::new(&child_label, WebviewUrl::External(content_url))
            .auto_resize()
            .background_color(AI_WEBVIEW_BG)
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
            .on_page_load(move |webview, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    log::debug!(
                        target: "app_lib::services::ai_webview",
                        "on_page_load Finished: webview={} url={}",
                        webview.label(),
                        webview.url().map(|u| u.to_string()).unwrap_or_default(),
                    );
                    if let Err(e) = webview.eval(&reinject) {
                        log::warn!(
                            target: "app_lib::services::ai_webview",
                            "failed to re-inject keybind script: {e}",
                        );
                    }
                }
            });

        let child_webview = match host.add_child(builder, pos, size) {
            Ok(w) => {
                log::info!(
                    target: "app_lib::services::ai_webview",
                    "hosted_swap_to_provider: add_child OK host={host_label} label={child_label}",
                );
                w
            }
            Err(e) => {
                log::error!(
                    target: "app_lib::services::ai_webview",
                    "hosted_swap_to_provider: add_child FAILED host={host_label} label={child_label}: {e}",
                );
                return Err(e.to_string());
            }
        };
        enable_media_for_webview(&child_webview);
        configure_linux_child_packing(&child_webview, LinuxChildRole::Content);
        if cfg!(target_os = "linux") && uses_custom_titlebar(host_label) {
            attach_undecorated_resize_handler(&child_webview);
        }

        if let Some(state) = app.try_state::<AiWebviewState>() {
            state.mark_hosted_child(host_label, &provider.id);
            state.set_provider(&child_label, &provider);
        }

        let post_create_webviews: Vec<String> =
            host.webviews().iter().map(|w| w.label().to_string()).collect();
        log::info!(
            target: "app_lib::services::ai_webview",
            "hosted child created: host={host_label} label={child_label} url={} host_webviews_now={post_create_webviews:?}",
            provider.url,
        );
    }

    let target = app
        .get_webview(&child_label)
        .ok_or_else(|| format!("missing child webview: {child_label}"))?;
    resume_if_suspended(app, &child_label);
    match target.show() {
        Ok(_) => log::debug!(
            target: "app_lib::services::ai_webview",
            "hosted_swap_to_provider: target.show OK label={child_label}",
        ),
        Err(e) => {
            log::error!(
                target: "app_lib::services::ai_webview",
                "hosted_swap_to_provider: target.show FAILED label={child_label}: {e}",
            );
            return Err(e.to_string());
        }
    }

    for webview in host.webviews() {
        let label = webview.label();
        if label == child_label || is_toolbar_label(label) {
            log::trace!(
                target: "app_lib::services::ai_webview",
                "hosted_swap_to_provider: skip-hide {label} (target or toolbar)",
            );
            continue;
        }
        match webview.hide() {
            Ok(_) => log::debug!(
                target: "app_lib::services::ai_webview",
                "hosted_swap_to_provider: hide OK label={label}",
            ),
            Err(e) => log::warn!(
                target: "app_lib::services::ai_webview",
                "hosted_swap_to_provider: hide FAILED label={label}: {e}",
            ),
        }
    }

    match target.set_focus() {
        Ok(_) => log::debug!(
            target: "app_lib::services::ai_webview",
            "hosted_swap_to_provider: set_focus OK label={child_label}",
        ),
        Err(e) => {
            log::error!(
                target: "app_lib::services::ai_webview",
                "hosted_swap_to_provider: set_focus FAILED label={child_label}: {e}",
            );
            return Err(e.to_string());
        }
    }

    if let Err(e) = host.set_title(&format!("{} — Promptheus", provider.name)) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_title failed: {e}",
        );
    }

    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, &child_label);
    }
    emit_active_changed(app, host_label, Some(&provider.id));

    focus_host_window(app, host_label)
}

pub(super) fn hosted_swap_to_conversation_dialog(
    app: &tauri::AppHandle,
    host_label: &str,
) -> Result<(), String> {
    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let cd_webview = app
        .get_webview(host_label)
        .ok_or_else(|| format!("missing webview: {host_label}"))?;
    cd_webview.show().map_err(|e| e.to_string())?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == host_label || is_toolbar_label(label) {
            continue;
        }
        if let Err(e) = webview.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "hide {label} failed: {e}",
            );
        }
    }

    cd_webview.set_focus().map_err(|e| e.to_string())?;

    if let Err(e) = host.set_title(CONVERSATION_DIALOG_TITLE) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_title failed: {e}",
        );
    }

    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, host_label);
    }
    emit_active_changed(app, host_label, None);

    focus_host_window(app, host_label)
}

pub fn emit_active_changed(app: &tauri::AppHandle, host_label: &str, provider_id: Option<&str>) {
    let payload = serde_json::json!({ "provider_id": provider_id });
    let toolbar_label = shell_toolbar_label_for(host_label);
    if let Err(e) = app.emit_to(host_label, "shell:active-changed", payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:active-changed failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(toolbar_label.as_str(), "shell:active-changed", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({toolbar_label}) shell:active-changed failed: {e}",
        );
    }
}

pub fn emit_active_changed_for(
    app: &tauri::AppHandle,
    host_label: &str,
    provider_id: Option<&str>,
) {
    emit_active_changed(app, host_label, provider_id);
}

pub fn mark_active_webview(app: &tauri::AppHandle, host_label: &str, webview_label: &str) {
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, webview_label);
    }
}


async fn swap_to_provider_standalone(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
    from_label: &str,
) -> Result<(), String> {
    debug_assert!(
        !is_conversation_dialog_host(from_label),
        "standalone swap should not run for conversation-dialog host: {from_label}",
    );
    let target_label = window_label(&provider);

    if from_label == target_label {
        if let Some(win) = app.get_webview_window(&target_label) {
            return focus_window(&win);
        }
        return open_or_focus(app, provider, None).await;
    }

    let target_already_open = app.get_webview_window(&target_label).is_some();
    if !target_already_open {
        if let Some(geom) = read_window_geometry(app, from_label) {
            if let Err(e) = app
                .state::<Arc<Mutex<UiStateService>>>()
                .lock()
                .await
                .set_geometry(&target_label, geom)
            {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "failed to seed target geometry: {e}",
                );
            }
        }
    }

    if app.get_webview_window(from_label).is_some() {
        close_by_label(app, from_label);
    }

    open_or_focus(app, provider, None).await
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_label_uses_provider_id() {
        let provider = WebviewProvider {
            id: "claude".into(),
            name: "Claude".into(),
            url: "https://claude.ai/".into(),
        };
        assert_eq!(window_label(&provider), "ai-webview-claude");
    }

    #[test]
    fn hosted_child_label_includes_host_prefix() {
        let provider = WebviewProvider {
            id: "claude".into(),
            name: "Claude".into(),
            url: "https://claude.ai/".into(),
        };
        assert_eq!(
            hosted_child_label("conversation-dialog", &provider),
            "conversation-dialog::ai-webview-claude"
        );
    }

}
