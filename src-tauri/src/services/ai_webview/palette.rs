use std::collections::HashMap;
use std::sync::Arc;

use tauri::{Emitter, LogicalPosition, LogicalSize, Manager};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::dialog::{SHELL_TOOLBAR_LABEL, TOOLBAR_HEIGHT};

use super::external_links::open_external_url;
use super::lifecycle::{host_window_for_webview, CONVERSATION_DIALOG_LABEL};
use super::provider_send::handle_send_outcome;
use super::provider_swap::{
    activate_adjacent_tab, activate_tab_at, close_active_tab, new_tab, new_tab_like_active,
};
use super::tabs::TabKind;
use super::AiWebviewState;

pub const PALETTE_TITLE: &str = "Promptheus Palette";
pub const PALETTE_BACKDROP_TITLE: &str = "Promptheus Palette Backdrop";

const PALETTE_LABEL: &str = "palette";
const PALETTE_BACKDROP_LABEL: &str = "palette-backdrop";
pub(super) const PROMPTHEUS_PROVIDER_ID: &str = "promptheus";

#[derive(Debug)]
pub(super) enum RouterMessage {
    OpenPalette,
    OpenExternal { url: String },
    RequestPasteText,
    TabNext,
    TabPrev,
    TabSelect { position: u32 },
    TabNew,
    TabClose,
    SendOutcome { status: SendStatus, run: u64, uploads: Option<u32>, waited_ms: Option<u64> },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SendStatus {
    Sent,
    Typed,
    NoComposer,
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
        "request_paste_text" => Some(RouterMessage::RequestPasteText),
        "tab_next" => Some(RouterMessage::TabNext),
        "tab_prev" => Some(RouterMessage::TabPrev),
        "tab_select" => {
            let position: u32 = params.remove("position")?.parse().ok()?;
            (1..=9).contains(&position).then_some(RouterMessage::TabSelect { position })
        }
        "tab_new" => Some(RouterMessage::TabNew),
        "tab_close" => Some(RouterMessage::TabClose),
        "webview_send_outcome" => {
            let status = match params.remove("status")?.as_str() {
                "sent" => SendStatus::Sent,
                "typed" => SendStatus::Typed,
                "no_composer" => SendStatus::NoComposer,
                _ => return None,
            };
            let run = params.remove("run")?.parse().ok()?;
            let uploads = params.remove("uploads").and_then(|v| v.parse().ok());
            let waited_ms = params.remove("waitedMs").and_then(|v| v.parse().ok());
            Some(RouterMessage::SendOutcome { status, run, uploads, waited_ms })
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
        RouterMessage::RequestPasteText => {
            deliver_clipboard_text_to_webview(app, webview_label);
        }
        RouterMessage::TabNext => log_tab_failure(activate_adjacent_tab(app, 1)),
        RouterMessage::TabPrev => log_tab_failure(activate_adjacent_tab(app, -1)),
        RouterMessage::TabSelect { position } => log_tab_failure(activate_tab_at(app, position)),
        RouterMessage::TabNew => log_tab_failure(new_tab_like_active(app).await),
        RouterMessage::TabClose => log_tab_failure(close_active_tab(app)),
        RouterMessage::SendOutcome { status, run, uploads, waited_ms } => {
            handle_send_outcome(app, webview_label, status, run, uploads, waited_ms).await;
        }
    }
}

fn log_tab_failure(result: Result<(), String>) {
    if let Err(e) = result {
        log::warn!(target: "app_lib::services::ai_webview", "tab shortcut failed: {e}");
    }
}

fn deliver_clipboard_text_to_webview(app: &tauri::AppHandle, webview_label: &str) {
    let Some(clipboard) = app.try_state::<ClipboardService>() else {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "request_paste_text: clipboard service missing",
        );
        return;
    };
    let text = match clipboard.get_text_raw() {
        Ok(text) => text,
        Err(e) => {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "request_paste_text: clipboard read failed on {webview_label}: {e}",
            );
            return;
        }
    };
    let Some(webview) = app.get_webview(webview_label) else {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "request_paste_text: webview {webview_label} not found",
        );
        return;
    };
    let payload = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!(
        "window.__promptheus && window.__promptheus.__deliverPasteText && window.__promptheus.__deliverPasteText({payload});"
    );
    if let Err(e) = webview.eval(&script) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "request_paste_text: deliver eval failed on {webview_label}: {e}",
        );
    }
}

#[cfg(target_os = "linux")]
fn is_host_relative_platform() -> bool {
    crate::services::gnome_shell::is_gnome_wayland()
}

#[cfg(not(target_os = "linux"))]
fn is_host_relative_platform() -> bool {
    false
}

#[cfg(target_os = "linux")]
async fn show_over_host(
    win: &tauri::WebviewWindow,
    host: &tauri::Window,
    title: &str,
    activate: bool,
) {
    use crate::services::gnome_shell::{place_window_on_parent, set_transient_parent};

    if let Err(e) = set_transient_parent(win, host).await {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_transient_parent({}) failed: {e}",
            win.label(),
        );
    }
    if let Err(e) =
        place_window_on_parent(title, 0, TOOLBAR_HEIGHT as i32, activate, Some(win)).await
    {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "place_window_on_parent({title}) failed: {e}",
        );
        if let Err(e) = win.show() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "show({}) failed: {e}",
                win.label(),
            );
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
        let _ = palette_win.set_focus();
        return Ok(());
    }

    state.set_palette_open(host_label, true);

    let active_label = state.active_view_label();
    let (active_provider_id, tabs, current_title) = state.with_tabs(|tabs| {
        let active = tabs.active_id().and_then(|id| tabs.get(id));
        let provider_id = match active.map(|tab| &tab.kind) {
            Some(TabKind::Provider { provider }) => provider.id.clone(),
            _ => PROMPTHEUS_PROVIDER_ID.to_string(),
        };
        let title = active.map(|tab| tab.title.clone()).unwrap_or_default();
        (provider_id, tabs.snapshot(), title)
    });

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
    let host_relative = is_host_relative_platform();
    if !host_relative {
        palette_win
            .set_position(LogicalPosition::new(host_logical_x, palette_y))
            .map_err(|e| e.to_string())?;
    }

    if let Some(backdrop) = app.get_webview_window(PALETTE_BACKDROP_LABEL) {
        if let Err(e) = backdrop.set_size(LogicalSize::new(host_logical_w, palette_h)) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "backdrop set_size failed: {e}",
            );
        }
        if host_relative {
            #[cfg(target_os = "linux")]
            show_over_host(&backdrop, &host, PALETTE_BACKDROP_TITLE, false).await;
        } else {
            if let Err(e) = backdrop.set_position(LogicalPosition::new(host_logical_x, palette_y))
            {
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
        }
    }

    let payload = serde_json::json!({
        "host_label": host_label,
        "active_id": active_provider_id,
        "providers": providers,
        "tabs": tabs,
        "current_title": current_title,
    });
    if let Err(e) = app.emit_to(PALETTE_LABEL, "palette:show", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to(palette) palette:show failed: {e}",
        );
    }

    if host_relative {
        #[cfg(target_os = "linux")]
        show_over_host(&palette_win, &host, PALETTE_TITLE, true).await;
    } else {
        palette_win.show().map_err(|e| e.to_string())?;
        if let Err(e) = palette_win.set_focus() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "set_focus(palette) failed: {e}",
            );
        }
    }

    let opened_payload = serde_json::json!({ "active_label": active_label });
    if let Err(e) = app.emit_to(host_label, "shell:palette-opened", opened_payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:palette-opened failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(SHELL_TOOLBAR_LABEL, "shell:palette-opened", opened_payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({SHELL_TOOLBAR_LABEL}) shell:palette-opened failed: {e}",
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
    if let Err(e) = app.emit_to(host_label, "shell:palette-closed", payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:palette-closed failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(SHELL_TOOLBAR_LABEL, "shell:palette-closed", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({SHELL_TOOLBAR_LABEL}) shell:palette-closed failed: {e}",
        );
    }

    match selected_provider_id.as_deref() {
        Some(id) => {
            if let Err(e) = new_tab(app, id).await {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "palette: new tab for {id} failed: {e}",
                );
                refocus_active_webview(app);
                return Err(e);
            }
        }
        None => {
            log::debug!(
                target: "app_lib::services::ai_webview",
                "swap_from_palette: no selection host={host_label} (view unchanged)",
            );
            refocus_active_webview(app);
        }
    }

    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_from_palette: DONE host={host_label}",
    );
    Ok(())
}

fn refocus_active_webview(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AiWebviewState>() else {
        return;
    };
    let active_label = state.active_view_label();
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

    fn parse_outcome(query: &str) -> Option<RouterMessage> {
        let url = tauri::Url::parse(&format!(
            "https://promptheus-ai-webview-router.invalid/?kind=webview_send_outcome{query}"
        ))
        .unwrap();
        parse_router_message(&url)
    }

    #[test]
    fn parse_router_message_send_outcome_upload_fields() {
        assert!(matches!(
            parse_outcome("&status=sent&run=7&uploads=2&waitedMs=1350"),
            Some(RouterMessage::SendOutcome { uploads: Some(2), waited_ms: Some(1350), .. })
        ));
        assert!(matches!(
            parse_outcome("&status=typed&run=8"),
            Some(RouterMessage::SendOutcome { uploads: None, waited_ms: None, .. })
        ));
    }

    #[test]
    fn parse_router_message_send_outcome_statuses() {
        assert!(matches!(
            parse_outcome("&status=sent&run=7"),
            Some(RouterMessage::SendOutcome { status: SendStatus::Sent, run: 7, .. })
        ));
        assert!(matches!(
            parse_outcome("&status=typed&run=8"),
            Some(RouterMessage::SendOutcome { status: SendStatus::Typed, run: 8, .. })
        ));
        assert!(matches!(
            parse_outcome("&status=no_composer&run=9"),
            Some(RouterMessage::SendOutcome { status: SendStatus::NoComposer, run: 9, .. })
        ));
    }

    #[test]
    fn parse_router_message_send_outcome_invalid() {
        assert!(parse_outcome("&run=1").is_none());
        assert!(parse_outcome("&status=bogus&run=1").is_none());
        assert!(parse_outcome("&status=sent").is_none());
        assert!(parse_outcome("&status=sent&run=x").is_none());
    }

    fn parse_kind(query: &str) -> Option<RouterMessage> {
        let url = tauri::Url::parse(&format!(
            "https://promptheus-ai-webview-router.invalid/?kind={query}"
        ))
        .unwrap();
        parse_router_message(&url)
    }

    #[test]
    fn parse_router_message_tab_kinds() {
        assert!(matches!(parse_kind("tab_next"), Some(RouterMessage::TabNext)));
        assert!(matches!(parse_kind("tab_prev"), Some(RouterMessage::TabPrev)));
        assert!(matches!(parse_kind("tab_new"), Some(RouterMessage::TabNew)));
        assert!(matches!(parse_kind("tab_close"), Some(RouterMessage::TabClose)));
        assert!(matches!(
            parse_kind("tab_select&position=3"),
            Some(RouterMessage::TabSelect { position: 3 })
        ));
    }

    #[test]
    fn parse_router_message_tab_select_invalid() {
        assert!(parse_kind("tab_select").is_none());
        assert!(parse_kind("tab_select&position=0").is_none());
        assert!(parse_kind("tab_select&position=10").is_none());
        assert!(parse_kind("tab_select&position=x").is_none());
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
    fn parse_router_message_request_paste_text() {
        let url = tauri::Url::parse(
            "https://promptheus-ai-webview-router.invalid/?kind=request_paste_text",
        )
        .unwrap();
        assert!(matches!(
            parse_router_message(&url),
            Some(RouterMessage::RequestPasteText)
        ));
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
