use std::collections::HashMap;
use std::sync::Arc;

use tauri::{Emitter, LogicalPosition, LogicalSize, Manager};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::config::ConfigService;
use crate::services::dialog::{focus_window, shell_toolbar_label_for, TOOLBAR_HEIGHT};

use super::external_links::open_external_url;
use super::lifecycle::{host_window_for_webview, CONVERSATION_DIALOG_LABEL};
use super::provider_swap::{hosted_swap_to_conversation_dialog, hosted_swap_to_provider};
use super::AiWebviewState;

const PALETTE_LABEL: &str = "palette";
const PALETTE_BACKDROP_LABEL: &str = "palette-backdrop";
pub(super) const PROMPTHEUS_PROVIDER_ID: &str = "promptheus";

#[derive(Debug)]
pub(super) enum RouterMessage {
    OpenPalette,
    OpenExternal { url: String },
}

pub(super) fn parse_router_message(url: &tauri::Url) -> Option<RouterMessage> {
    let mut params: HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let kind = params.remove("kind")?;
    match kind.as_str() {
        "open_palette" => Some(RouterMessage::OpenPalette),
        "open_external" => {
            let url = params.remove("url")?;
            if url.is_empty() {
                return None;
            }
            Some(RouterMessage::OpenExternal { url })
        }
        _ => None,
    }
}

pub(super) async fn handle_router_message(
    app: &tauri::AppHandle,
    webview_label: &str,
    msg: RouterMessage,
) {
    log::info!(
        target: "app_lib::services::ai_webview",
        "router message on {webview_label}: {msg:?}",
    );
    let host_label = host_window_for_webview(app, webview_label).unwrap_or_default();

    match msg {
        RouterMessage::OpenPalette => {
            let target_host = if host_label.is_empty() {
                CONVERSATION_DIALOG_LABEL.to_string()
            } else {
                host_label
            };
            if let Err(e) = swap_to_palette(app, &target_host).await {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "swap_to_palette failed: {e}",
                );
            }
        }
        RouterMessage::OpenExternal { url } => {
            open_external_url(app, &url);
        }
    }
}

pub async fn swap_to_palette(app: &tauri::AppHandle, host_label: &str) -> Result<(), String> {
    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_to_palette: ENTER host={host_label}",
    );

    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;

    let palette_win = app
        .get_webview_window(PALETTE_LABEL)
        .ok_or_else(|| "palette window not found".to_string())?;

    if state.is_palette_open(host_label) {
        log::debug!(
            target: "app_lib::services::ai_webview",
            "swap_to_palette: already open host={host_label} (refocus)",
        );
        let _ = focus_window(&palette_win);
        return Ok(());
    }

    state.set_palette_open(host_label, true);

    let active_label = state
        .get_active(host_label)
        .unwrap_or_else(|| host_label.to_string());
    let active_provider_id = if active_label == host_label {
        PROMPTHEUS_PROVIDER_ID.to_string()
    } else {
        state
            .snapshot(&active_label)
            .map(|p| p.id)
            .unwrap_or_else(|| PROMPTHEUS_PROVIDER_ID.to_string())
    };

    let providers = app
        .state::<Arc<Mutex<ConfigService>>>()
        .lock()
        .await
        .settings()
        .webview_providers
        .clone();

    let scale = host.scale_factor().map_err(|e| e.to_string())?;
    let inner_pos = host.inner_position().map_err(|e| e.to_string())?;
    let inner_size = host.inner_size().map_err(|e| e.to_string())?;
    let host_logical_x = inner_pos.x as f64 / scale;
    let host_logical_y = inner_pos.y as f64 / scale;
    let host_logical_w = inner_size.width as f64 / scale;
    let host_logical_h = inner_size.height as f64 / scale;
    let palette_y = host_logical_y + TOOLBAR_HEIGHT;
    let palette_h = (host_logical_h - TOOLBAR_HEIGHT).max(0.0);

    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_to_palette: positioning host={host_label} pos=({host_logical_x},{palette_y}) size=({host_logical_w}x{palette_h})",
    );

    palette_win
        .set_size(LogicalSize::new(host_logical_w, palette_h))
        .map_err(|e| e.to_string())?;
    palette_win
        .set_position(LogicalPosition::new(host_logical_x, palette_y))
        .map_err(|e| e.to_string())?;

    if let Some(backdrop) = app.get_webview_window(PALETTE_BACKDROP_LABEL) {
        if let Err(e) = backdrop.set_size(LogicalSize::new(host_logical_w, palette_h)) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "backdrop set_size failed: {e}",
            );
        }
        if let Err(e) = backdrop.set_position(LogicalPosition::new(host_logical_x, palette_y)) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "backdrop set_position failed: {e}",
            );
        }
        if let Err(e) = backdrop.show() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "backdrop show failed: {e}",
            );
        }
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::WidgetExt;
            if let Ok(gtk_win) = backdrop.gtk_window() {
                gtk_win.set_opacity(0.68);
            }
        }
    }

    let payload = serde_json::json!({
        "host_label": host_label,
        "active_id": active_provider_id,
        "providers": providers,
    });
    if let Err(e) = app.emit_to(PALETTE_LABEL, "palette:show", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to(palette) palette:show failed: {e}",
        );
    }

    palette_win.show().map_err(|e| e.to_string())?;
    if let Err(e) = focus_window(&palette_win) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "focus_window(palette) failed: {e}",
        );
    }

    let opened_payload = serde_json::json!({ "active_label": active_label });
    let toolbar_label = shell_toolbar_label_for(host_label);
    if let Err(e) = app.emit_to(host_label, "shell:palette-opened", opened_payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:palette-opened failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(toolbar_label.as_str(), "shell:palette-opened", opened_payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({toolbar_label}) shell:palette-opened failed: {e}",
        );
    }

    Ok(())
}

pub async fn swap_from_palette(
    app: &tauri::AppHandle,
    host_label: &str,
    selected_provider_id: Option<String>,
) -> Result<(), String> {
    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_from_palette: ENTER host={host_label} selected={selected_provider_id:?}",
    );

    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;

    if !state.try_consume_palette_open(host_label) {
        log::debug!(
            target: "app_lib::services::ai_webview",
            "swap_from_palette: palette not open host={host_label} (no-op)",
        );
        return Ok(());
    }

    if let Some(palette_win) = app.get_webview_window(PALETTE_LABEL) {
        if let Err(e) = palette_win.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "palette hide failed: {e}",
            );
        }
    }
    if let Some(backdrop) = app.get_webview_window(PALETTE_BACKDROP_LABEL) {
        if let Err(e) = backdrop.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "palette backdrop hide failed: {e}",
            );
        }
    }

    let payload = serde_json::json!({});
    let toolbar_label = shell_toolbar_label_for(host_label);
    if let Err(e) = app.emit_to(host_label, "shell:palette-closed", payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:palette-closed failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(toolbar_label.as_str(), "shell:palette-closed", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({toolbar_label}) shell:palette-closed failed: {e}",
        );
    }

    match selected_provider_id.as_deref() {
        Some(PROMPTHEUS_PROVIDER_ID) => {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "swap_from_palette: routing to hosted_swap_to_conversation_dialog host={host_label}",
            );
            hosted_swap_to_conversation_dialog(app, host_label)?;
        }
        Some(id) => {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "swap_from_palette: looking up provider id={id}",
            );
            let provider = lookup_webview_provider(app, id).await;
            let Some(provider) = provider else {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "palette: unknown provider {id}",
                );
                refocus_active_webview(app, host_label);
                return Ok(());
            };
            log::debug!(
                target: "app_lib::services::ai_webview",
                "swap_from_palette: routing to hosted_swap_to_provider host={host_label} provider_id={}",
                provider.id,
            );
            hosted_swap_to_provider(app, host_label, provider).await?;
        }
        None => {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "swap_from_palette: no selection host={host_label} (view unchanged)",
            );
            refocus_active_webview(app, host_label);
        }
    }

    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_from_palette: DONE host={host_label}",
    );
    Ok(())
}

fn refocus_active_webview(app: &tauri::AppHandle, host_label: &str) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let Some(active_label) = state.get_active(host_label) else {
        return;
    };
    if let Some(webview) = app.get_webview(&active_label) {
        if let Err(e) = webview.set_focus() {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "refocus_active_webview: set_focus({active_label}) failed: {e}",
            );
        }
    }
}

pub async fn lookup_webview_provider(
    app: &tauri::AppHandle,
    id: &str,
) -> Option<WebviewProvider> {
    app.state::<Arc<Mutex<ConfigService>>>()
        .lock()
        .await
        .settings()
        .find_webview_provider(id)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_router_message_open_palette() {
        let url = tauri::Url::parse(
            "https://promptheus-ai-webview-router.invalid/?kind=open_palette",
        )
        .unwrap();
        let msg = parse_router_message(&url).unwrap();
        assert!(matches!(msg, RouterMessage::OpenPalette));
    }

    #[test]
    fn parse_router_message_unknown_kind() {
        let url =
            tauri::Url::parse("https://promptheus-ai-webview-router.invalid/?kind=bogus").unwrap();
        assert!(parse_router_message(&url).is_none());
    }

    #[test]
    fn parse_router_message_no_kind() {
        let url = tauri::Url::parse("https://promptheus-ai-webview-router.invalid/").unwrap();
        assert!(parse_router_message(&url).is_none());
    }

    #[test]
    fn parse_router_message_empty_query() {
        let url = tauri::Url::parse("https://promptheus-ai-webview-router.invalid/?").unwrap();
        assert!(parse_router_message(&url).is_none());
    }

    #[test]
    fn parse_router_message_open_external() {
        let url = tauri::Url::parse(
            "https://promptheus-ai-webview-router.invalid/?kind=open_external&url=https%3A%2F%2Fexample.com%2Fpath",
        )
        .unwrap();
        match parse_router_message(&url) {
            Some(RouterMessage::OpenExternal { url }) => {
                assert_eq!(url, "https://example.com/path");
            }
            other => panic!("expected OpenExternal, got {other:?}"),
        }
    }

    #[test]
    fn parse_router_message_open_external_missing_url() {
        let url = tauri::Url::parse(
            "https://promptheus-ai-webview-router.invalid/?kind=open_external",
        )
        .unwrap();
        assert!(parse_router_message(&url).is_none());
    }

    #[test]
    fn parse_router_message_open_external_empty_url() {
        let url = tauri::Url::parse(
            "https://promptheus-ai-webview-router.invalid/?kind=open_external&url=",
        )
        .unwrap();
        assert!(parse_router_message(&url).is_none());
    }
}
