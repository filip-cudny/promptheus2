use std::sync::Arc;

use tauri::webview::PageLoadEvent;
use tauri::window::Color;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;

use crate::services::dialog;
use crate::services::dialog::{focus_host_window, focus_window, save_geometry};
use crate::services::dock::DockManager;
use crate::services::ui_state::{UiStateService, WindowGeometry};

use super::palette::{handle_router_message, parse_router_message};
use super::provider_swap::{
    hosted_swap_to_conversation_dialog, window_label, ROUTER_SENTINEL,
};
use super::scripts::{initialization_script, reinject_script};
use super::AiWebviewState;

pub(super) const DEFAULT_WIDTH: f64 = 1000.0;
pub(super) const DEFAULT_HEIGHT: f64 = 720.0;
pub(super) const AI_WEBVIEW_BG: Color = Color(0x1e, 0x1e, 0x1e, 0xff);
pub(super) const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
pub(super) const CONVERSATION_DIALOG_TITLE: &str = "Promptheus — chat";

pub async fn open_or_focus(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
    url: Option<String>,
) -> Result<(), String> {
    let label = window_label(&provider);

    if let Some(existing) = app.get_webview_window(&label) {
        log::info!(
            target: "app_lib::services::ai_webview",
            "focus existing window: {label}",
        );
        if let Some(u) = url.as_deref() {
            navigate_webview(&existing, u)?;
            if let Some(state) = app.try_state::<AiWebviewState>() {
                state.set_provider(&label, &provider);
                state.unmark_suspended(&label);
            }
        } else {
            super::cold_suspend::resume_if_suspended(app, &label);
        }
        if let Some(state) = app.try_state::<AiWebviewState>() {
            state.touch_last_active(&label);
        }
        return focus_window(&existing);
    }

    open_window(app, provider, &label, url).await
}

pub async fn open_new_instance(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
    url: Option<String>,
    source_label: Option<String>,
) -> Result<(), String> {
    let label = next_available_label(app, &provider);
    if let Some(src) = source_label.as_deref() {
        dialog::seed_geometry_from(app, src, &label).await;
    }
    open_window(app, provider, &label, url).await
}

pub fn navigate(app: &tauri::AppHandle, provider_id: &str, url: &str) -> Result<(), String> {
    let label = format!("ai-webview-{provider_id}");
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("no window: {label}"))?;
    navigate_webview(&win, url)
}

pub fn close(app: &tauri::AppHandle, provider_id: &str) -> Result<(), String> {
    let label = format!("ai-webview-{provider_id}");
    close_by_label(app, &label);
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.remove_provider(&label);
    }
    Ok(())
}

pub(super) fn close_by_label(app: &tauri::AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        log::info!(target: "app_lib::services::ai_webview", "closing {label}");
        if let Err(e) = win.close() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "close failed for {label}: {e}",
            );
        }
    }
}

pub fn focus_conversation_dialog(app: &tauri::AppHandle) {
    if app.get_window(CONVERSATION_DIALOG_LABEL).is_some() {
        if let Err(e) = focus_host_window(app, CONVERSATION_DIALOG_LABEL) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "failed to focus conversation-dialog: {e}",
            );
        }
    }
}

pub fn host_window_for_webview(app: &tauri::AppHandle, webview_label: &str) -> Option<String> {
    app.get_webview(webview_label)
        .map(|w| w.window().label().to_string())
}

pub fn cleanup_host_state(app: &tauri::AppHandle, host_label: &str) {
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.drop_host(host_label);
    }
}

pub(super) fn navigate_webview(win: &tauri::WebviewWindow, url: &str) -> Result<(), String> {
    let parsed = tauri::Url::parse(url).map_err(|e| e.to_string())?;
    win.navigate(parsed).map_err(|e| e.to_string())
}

fn next_available_label(app: &tauri::AppHandle, provider: &WebviewProvider) -> String {
    let base = window_label(provider);
    if app.get_webview_window(&base).is_none() {
        return base;
    }
    let mut i = 2u32;
    loop {
        let candidate = format!("{base}-{i}");
        if app.get_webview_window(&candidate).is_none() {
            return candidate;
        }
        i += 1;
    }
}

pub(super) fn read_window_geometry(app: &tauri::AppHandle, label: &str) -> Option<WindowGeometry> {
    let win = app.get_window(label)?;
    let pos = win.outer_position().ok()?;
    let size = win.inner_size().ok()?;
    let scale = win.scale_factor().unwrap_or(1.0);
    Some(WindowGeometry {
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    })
}

pub(super) fn host_logical_size(host: &tauri::Window) -> Result<(f64, f64), String> {
    let physical = host.inner_size().map_err(|e| e.to_string())?;
    let scale = host.scale_factor().map_err(|e| e.to_string())?;
    Ok((
        physical.width as f64 / scale,
        physical.height as f64 / scale,
    ))
}

pub(super) async fn swap_to_conversation_dialog_standalone(
    app: &tauri::AppHandle,
    from_label: &str,
) -> Result<(), String> {
    if app.get_window(CONVERSATION_DIALOG_LABEL).is_some() {
        hosted_swap_to_conversation_dialog(app, CONVERSATION_DIALOG_LABEL)?;
        if from_label != CONVERSATION_DIALOG_LABEL && app.get_window(from_label).is_some() {
            close_by_label(app, from_label);
        }
        return Ok(());
    }

    if let Some(geom) = read_window_geometry(app, from_label) {
        if let Err(e) = app
            .state::<Arc<Mutex<UiStateService>>>()
            .lock()
            .await
            .set_geometry(CONVERSATION_DIALOG_LABEL, geom)
        {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "failed to seed conversation-dialog geometry: {e}",
            );
        }
    }

    if from_label != CONVERSATION_DIALOG_LABEL && app.get_window(from_label).is_some() {
        close_by_label(app, from_label);
    }

    let config = dialog::DialogConfig {
        label: CONVERSATION_DIALOG_LABEL.into(),
        url: "conversation-dialog.html".into(),
        title: CONVERSATION_DIALOG_TITLE.into(),
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: CONVERSATION_DIALOG_LABEL.into(),
    };
    let (win, _) = dialog::open_or_focus(app, &config).await?;
    focus_window(&win)
}

async fn open_window(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
    label: &str,
    url: Option<String>,
) -> Result<(), String> {
    let geometry = app
        .state::<Arc<Mutex<UiStateService>>>()
        .lock()
        .await
        .get_geometry(label);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));

    let content = url.as_deref().unwrap_or(provider.url.as_str()).to_string();
    let content_url = tauri::Url::parse(&content).map_err(|e| e.to_string())?;

    let init_script = initialization_script(&provider);
    let reinject = reinject_script();

    let label_owned = label.to_string();
    let app_for_nav = app.clone();
    let label_for_nav = label_owned.clone();
    let label_for_title = label_owned.clone();

    let mut builder =
        WebviewWindowBuilder::new(app, &label_owned, WebviewUrl::External(content_url))
            .title(format!("{} — Promptheus", provider.name))
            .inner_size(width, height)
            .resizable(true)
            .background_color(AI_WEBVIEW_BG)
            .initialization_script(&init_script)
            .on_navigation(move |url| {
                if !url.as_str().starts_with(ROUTER_SENTINEL) {
                    return true;
                }
                match parse_router_message(url) {
                    Some(msg) => {
                        let app = app_for_nav.clone();
                        let label = label_for_nav.clone();
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
            .on_document_title_changed(move |window, title| {
                if let Some(state) = window.app_handle().try_state::<AiWebviewState>() {
                    state.set_page_title(&label_for_title, &title);
                }
            })
            .on_page_load(move |webview, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    if let Err(e) = webview.eval(&reinject) {
                        log::warn!(
                            target: "app_lib::services::ai_webview",
                            "failed to re-inject toolbar: {e}",
                        );
                    }
                }
            });

    if let Some(g) = &geometry {
        builder = builder.position(g.x, g.y);
    }

    let win = builder.build().map_err(|e| e.to_string())?;
    enable_media_for_window(&win);

    if let Some(webview_state) = app.try_state::<AiWebviewState>() {
        webview_state.set_provider(&label_owned, &provider);
    }

    let dock = app.state::<DockManager>();
    dock.dialog_opened(app);

    let app_handle = app.clone();
    let label_for_event = label_owned.clone();

    win.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } => {
            save_geometry(&app_handle, &label_for_event, &label_for_event);
        }
        WindowEvent::Destroyed => {
            if let Some(state) = app_handle.try_state::<AiWebviewState>() {
                state.remove_provider(&label_for_event);
            }
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        WindowEvent::Focused(true) => {
            if let Some(state) = app_handle.try_state::<AiWebviewState>() {
                state.touch_host_focus(&label_for_event);
            }
        }
        _ => {}
    });

    focus_window(&win)?;
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.touch_host_focus(&label_owned);
    }

    log::info!(
        target: "app_lib::services::ai_webview",
        "opened {label_owned} -> {content}",
    );
    Ok(())
}

pub(super) fn enable_media_for_window(win: &tauri::WebviewWindow) {
    #[cfg(target_os = "linux")]
    {
        let label = win.label().to_string();
        let app = win.app_handle().clone();
        let label_for_install = label.clone();
        if let Err(e) = win.with_webview(move |pv| {
            install_media_permissions(pv, app, label_for_install);
        }) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "install media permissions failed for window {label}: {e}",
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = win;
    }
}

pub(super) fn enable_media_for_webview(webview: &tauri::Webview) {
    #[cfg(target_os = "linux")]
    {
        let label = webview.label().to_string();
        let app = webview.app_handle().clone();
        let label_for_install = label.clone();
        if let Err(e) = webview.with_webview(move |pv| {
            install_media_permissions(pv, app, label_for_install);
        }) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "install media permissions failed for webview {label}: {e}",
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = webview;
    }
}

#[cfg(target_os = "linux")]
fn install_media_permissions(
    pv: tauri::webview::PlatformWebview,
    app: tauri::AppHandle,
    webview_label: String,
) {
    use webkit2gtk::glib;
    use webkit2gtk::glib::object::ObjectExt;
    use webkit2gtk::{
        DeviceInfoPermissionRequest, PermissionRequestExt, SettingsExt,
        UserMediaPermissionRequest, WebViewExt,
    };

    let wk = pv.inner();
    if let Some(settings) = WebViewExt::settings(&wk) {
        settings.set_enable_media_stream(true);
        settings.set_enable_mediasource(true);
        settings.set_enable_media_capabilities(true);
        settings.set_enable_webrtc(true);
    }

    wk.connect_permission_request(|_, req| {
        if req.is::<UserMediaPermissionRequest>() || req.is::<DeviceInfoPermissionRequest>() {
            req.allow();
            true
        } else {
            false
        }
    });

    let label_for_term = webview_label.clone();
    wk.connect_web_process_terminated(move |_, reason| {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "web process terminated: webview={label_for_term} reason={reason:?}",
        );
    });

    let label_for_watch = webview_label;
    let app_for_watch = app;
    wk.connect_is_web_process_responsive_notify(move |wv| {
        let responsive = wv.is_web_process_responsive();
        log::warn!(
            target: "app_lib::services::ai_webview",
            "responsiveness changed: webview={label_for_watch} responsive={responsive}",
        );
        if responsive {
            return;
        }
        let label = label_for_watch.clone();
        let app = app_for_watch.clone();
        let wv_weak = wv.downgrade();
        glib::timeout_add_seconds_local_once(
            super::cold_suspend::UNRESPONSIVE_GRACE_SECONDS,
            move || {
                let Some(wv) = wv_weak.upgrade() else { return };
                if wv.is_web_process_responsive() {
                    log::info!(
                        target: "app_lib::services::ai_webview",
                        "responsiveness recovered: webview={label}",
                    );
                    return;
                }
                log::error!(
                    target: "app_lib::services::ai_webview",
                    "web process unresponsive >{}s, restarting: webview={label}",
                    super::cold_suspend::UNRESPONSIVE_GRACE_SECONDS,
                );
                wv.terminate_web_process();
                wv.reload();
                let provider_name = super::cold_suspend::provider_display_name(&app, &label);
                super::cold_suspend::emit_provider_lifecycle_toast(
                    &app,
                    "provider_restarted",
                    crate::services::notification::NotificationLevel::Warning,
                    format!("{provider_name} was unresponsive"),
                    Some(format!(
                        "Restarted to recover (was unresponsive {}s+)",
                        super::cold_suspend::UNRESPONSIVE_GRACE_SECONDS,
                    )),
                );
            },
        );
    });
}
