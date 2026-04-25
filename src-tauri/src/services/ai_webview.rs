use std::collections::{HashMap, HashSet};
use std::sync::Mutex as StdMutex;

use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::window::Color;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::Mutex;

use super::ai_providers::AiProvider;
use super::dialog::{
    self, attach_undecorated_resize_handler, configure_linux_child_packing, content_layout,
    focus_host_window, focus_window, is_shell_toolbar_label, save_geometry, uses_custom_titlebar,
    DialogConfig, LinuxChildRole,
};
use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

const DEFAULT_WIDTH: f64 = 1000.0;
const DEFAULT_HEIGHT: f64 = 720.0;
const AI_WEBVIEW_BG: Color = Color(0xff, 0xff, 0xff, 0xff);
const ROUTER_SENTINEL: &str = "https://promptheus-ai-webview-router.invalid/";
const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
const CONVERSATION_DIALOG_TITLE: &str = "Promptheus — chat";

#[derive(Default)]
pub struct AiWebviewState {
    hosted: StdMutex<HashMap<String, HashSet<&'static str>>>,
    current_provider: StdMutex<HashMap<String, &'static str>>,
    active_webview: StdMutex<HashMap<String, String>>,
    previous_active: StdMutex<HashMap<String, String>>,
    palette_open: StdMutex<HashMap<String, bool>>,
}

impl AiWebviewState {
    fn set_provider(&self, label: &str, provider_id: &'static str) {
        self.current_provider
            .lock()
            .unwrap()
            .insert(label.to_string(), provider_id);
    }

    fn remove_provider(&self, label: &str) {
        self.current_provider.lock().unwrap().remove(label);
    }

    fn has_hosted_child(&self, host_label: &str, provider_id: &'static str) -> bool {
        self.hosted
            .lock()
            .unwrap()
            .get(host_label)
            .map(|set| set.contains(provider_id))
            .unwrap_or(false)
    }

    fn mark_hosted_child(&self, host_label: &str, provider_id: &'static str) {
        self.hosted
            .lock()
            .unwrap()
            .entry(host_label.to_string())
            .or_default()
            .insert(provider_id);
    }

    fn drop_host(&self, host_label: &str) {
        self.hosted.lock().unwrap().remove(host_label);
        self.active_webview.lock().unwrap().remove(host_label);
        self.previous_active.lock().unwrap().remove(host_label);
        self.palette_open.lock().unwrap().remove(host_label);
    }

    fn set_active(&self, host_label: &str, webview_label: &str) {
        self.active_webview
            .lock()
            .unwrap()
            .insert(host_label.to_string(), webview_label.to_string());
    }

    fn get_active(&self, host_label: &str) -> Option<String> {
        self.active_webview
            .lock()
            .unwrap()
            .get(host_label)
            .cloned()
    }

    fn set_previous_active(&self, host_label: &str, webview_label: &str) {
        self.previous_active
            .lock()
            .unwrap()
            .insert(host_label.to_string(), webview_label.to_string());
    }

    fn take_previous_active(&self, host_label: &str) -> Option<String> {
        self.previous_active.lock().unwrap().remove(host_label)
    }

    fn set_palette_open(&self, host_label: &str, open: bool) {
        self.palette_open
            .lock()
            .unwrap()
            .insert(host_label.to_string(), open);
    }

    fn is_palette_open(&self, host_label: &str) -> bool {
        self.palette_open
            .lock()
            .unwrap()
            .get(host_label)
            .copied()
            .unwrap_or(false)
    }
}

pub fn is_palette_open(app: &tauri::AppHandle, host_label: &str) -> bool {
    app.try_state::<AiWebviewState>()
        .map(|s| s.is_palette_open(host_label))
        .unwrap_or(false)
}

pub fn active_provider_for(app: &tauri::AppHandle, host_label: &str) -> Option<String> {
    let state = app.try_state::<AiWebviewState>()?;
    let active = state.get_active(host_label)?;
    if active == host_label {
        return None;
    }
    let provider = state
        .current_provider
        .lock()
        .unwrap()
        .get(&active)
        .map(|s| s.to_string());
    provider
}

pub fn window_label(provider: &AiProvider) -> String {
    format!("ai-webview-{}", provider.id)
}

fn host_logical_size(host: &tauri::Window) -> Result<(f64, f64), String> {
    let physical = host.inner_size().map_err(|e| e.to_string())?;
    let scale = host.scale_factor().map_err(|e| e.to_string())?;
    Ok((
        physical.width as f64 / scale,
        physical.height as f64 / scale,
    ))
}

fn is_toolbar_label(label: &str) -> bool {
    is_shell_toolbar_label(label)
}

fn hosted_child_label(host_label: &str, provider: &AiProvider) -> String {
    format!("{host_label}::ai-webview-{}", provider.id)
}

pub async fn open_or_focus(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
    url: Option<String>,
) -> Result<(), String> {
    let label = window_label(provider);

    if let Some(existing) = app.get_webview_window(&label) {
        log::info!(
            target: "app_lib::services::ai_webview",
            "focus existing window: {label}",
        );
        if let Some(u) = url.as_deref() {
            navigate_webview(&existing, u)?;
            if let Some(state) = app.try_state::<AiWebviewState>() {
                state.set_provider(&label, provider.id);
            }
        }
        return focus_window(&existing);
    }

    open_window(app, provider, &label, url).await
}

pub async fn open_new_instance(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
    url: Option<String>,
) -> Result<(), String> {
    let label = next_available_label(app, provider);
    open_window(app, provider, &label, url).await
}

pub fn navigate(app: &tauri::AppHandle, provider_id: &str, url: &str) -> Result<(), String> {
    let label = format!("ai-webview-{provider_id}");
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("no window: {label}"))?;
    navigate_webview(&win, url)
}

pub fn new_chat_in_host(app: &tauri::AppHandle, host_label: &str) -> Result<(), String> {
    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;
    let active_label = state
        .get_active(host_label)
        .ok_or_else(|| "no active webview".to_string())?;
    if active_label == host_label {
        return app
            .emit_to(host_label, "new-conversation", serde_json::json!({}))
            .map_err(|e| e.to_string());
    }
    let provider_id = {
        let guard = state.current_provider.lock().unwrap();
        guard
            .get(&active_label)
            .copied()
            .ok_or_else(|| format!("no provider for {active_label}"))?
    };
    let provider = super::ai_providers::find(provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;
    let webview = app
        .get_webview(&active_label)
        .ok_or_else(|| format!("missing webview: {active_label}"))?;
    let parsed = tauri::Url::parse(provider.url).map_err(|e| e.to_string())?;
    webview.navigate(parsed).map_err(|e| e.to_string())
}

pub async fn swap_to_provider(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
    from_label: &str,
) -> Result<(), String> {
    if from_label == CONVERSATION_DIALOG_LABEL
        && app.get_window(CONVERSATION_DIALOG_LABEL).is_some()
    {
        return hosted_swap_to_provider(app, provider).await;
    }

    swap_to_provider_standalone(app, provider, from_label).await
}

pub async fn swap_to_conversation_dialog(
    app: &tauri::AppHandle,
    from_label: &str,
) -> Result<(), String> {
    if from_label == CONVERSATION_DIALOG_LABEL
        && app.get_window(CONVERSATION_DIALOG_LABEL).is_some()
    {
        return hosted_swap_to_conversation_dialog(app);
    }

    swap_to_conversation_dialog_standalone(app, from_label).await
}

async fn hosted_swap_to_provider(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
) -> Result<(), String> {
    let host_label = CONVERSATION_DIALOG_LABEL;
    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let child_label = hosted_child_label(host_label, provider);

    let already_created = app
        .try_state::<AiWebviewState>()
        .map(|s| s.has_hosted_child(host_label, provider.id))
        .unwrap_or(false);

    if !already_created {
        let (logical_w, logical_h) = host_logical_size(&host)?;
        let (pos, size) = content_layout(logical_w, logical_h);
        let content_url = tauri::Url::parse(provider.url).map_err(|e| e.to_string())?;
        let init_script = initialization_script(provider);
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
                    if let Err(e) = webview.eval(&reinject) {
                        log::warn!(
                            target: "app_lib::services::ai_webview",
                            "failed to re-inject keybind script: {e}",
                        );
                    }
                }
            });

        let child_webview = host
            .add_child(builder, pos, size)
            .map_err(|e| e.to_string())?;
        configure_linux_child_packing(&child_webview, LinuxChildRole::Content);
        if cfg!(target_os = "linux") && uses_custom_titlebar(host_label) {
            attach_undecorated_resize_handler(&child_webview);
        }

        if let Some(state) = app.try_state::<AiWebviewState>() {
            state.mark_hosted_child(host_label, provider.id);
            state.set_provider(&child_label, provider.id);
        }

        log::info!(
            target: "app_lib::services::ai_webview",
            "hosted child created: host={host_label} label={child_label} url={}",
            provider.url,
        );
    }

    let target = app
        .get_webview(&child_label)
        .ok_or_else(|| format!("missing child webview: {child_label}"))?;
    target.show().map_err(|e| e.to_string())?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == child_label || is_toolbar_label(label) {
            continue;
        }
        if let Err(e) = webview.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "hide {label} failed: {e}",
            );
        }
    }

    target.set_focus().map_err(|e| e.to_string())?;

    if let Err(e) = host.set_title(&format!("{} — Promptheus", provider.name)) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "set_title failed: {e}",
        );
    }

    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, &child_label);
    }
    emit_active_changed(app, Some(provider.id));

    focus_host_window(app, host_label)
}

fn hosted_swap_to_conversation_dialog(app: &tauri::AppHandle) -> Result<(), String> {
    let host_label = CONVERSATION_DIALOG_LABEL;
    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let cd_webview = app
        .get_webview(CONVERSATION_DIALOG_LABEL)
        .ok_or_else(|| format!("missing webview: {CONVERSATION_DIALOG_LABEL}"))?;
    cd_webview.show().map_err(|e| e.to_string())?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == CONVERSATION_DIALOG_LABEL || is_toolbar_label(label) {
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
        state.set_active(host_label, CONVERSATION_DIALOG_LABEL);
    }
    emit_active_changed(app, None);

    focus_host_window(app, host_label)
}

fn emit_active_changed(app: &tauri::AppHandle, provider_id: Option<&str>) {
    let payload = serde_json::json!({ "provider_id": provider_id });
    if let Err(e) = app.emit("shell:active-changed", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit shell:active-changed failed: {e}",
        );
    }
}

pub fn emit_active_changed_for(app: &tauri::AppHandle, provider_id: Option<&str>) {
    emit_active_changed(app, provider_id);
}

pub fn mark_active_webview(app: &tauri::AppHandle, host_label: &str, webview_label: &str) {
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, webview_label);
    }
}

pub async fn swap_to_palette(app: &tauri::AppHandle, host_label: &str) -> Result<(), String> {
    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;

    if state.is_palette_open(host_label) {
        if let Some(cd) = app.get_webview(CONVERSATION_DIALOG_LABEL) {
            let _ = cd.set_focus();
        }
        return Ok(());
    }

    let previous = state
        .get_active(host_label)
        .unwrap_or_else(|| CONVERSATION_DIALOG_LABEL.to_string());
    state.set_previous_active(host_label, &previous);
    state.set_palette_open(host_label, true);

    let cd = app
        .get_webview(CONVERSATION_DIALOG_LABEL)
        .ok_or_else(|| format!("missing webview: {CONVERSATION_DIALOG_LABEL}"))?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == CONVERSATION_DIALOG_LABEL || is_toolbar_label(label) {
            continue;
        }
        if let Err(e) = webview.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "palette hide {label} failed: {e}",
            );
        }
    }

    cd.show().map_err(|e| e.to_string())?;
    cd.set_focus().map_err(|e| e.to_string())?;

    let payload = serde_json::json!({ "previous_label": previous });
    if let Err(e) = app.emit("shell:palette-opened", payload) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit shell:palette-opened failed: {e}",
        );
    }

    Ok(())
}

pub async fn swap_from_palette(
    app: &tauri::AppHandle,
    host_label: &str,
    selected_provider_id: Option<String>,
) -> Result<(), String> {
    let state = app
        .try_state::<AiWebviewState>()
        .ok_or_else(|| "ai webview state missing".to_string())?;

    if !state.is_palette_open(host_label) {
        return Ok(());
    }

    state.set_palette_open(host_label, false);

    let previous = state
        .take_previous_active(host_label)
        .unwrap_or_else(|| CONVERSATION_DIALOG_LABEL.to_string());

    if let Err(e) = app.emit("shell:palette-closed", serde_json::json!({})) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit shell:palette-closed failed: {e}",
        );
    }

    match selected_provider_id.as_deref() {
        Some(PROMPTHEUS_PROVIDER_ID) => {
            hosted_swap_to_conversation_dialog(app)?;
        }
        Some(id) => {
            let Some(provider) = super::ai_providers::find(id) else {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "palette: unknown provider {id}",
                );
                return restore_previous_webview(app, host_label, &previous);
            };
            hosted_swap_to_provider(app, provider).await?;
        }
        None => {
            restore_previous_webview(app, host_label, &previous)?;
        }
    }

    Ok(())
}

fn restore_previous_webview(
    app: &tauri::AppHandle,
    host_label: &str,
    previous_label: &str,
) -> Result<(), String> {
    let host = app
        .get_window(host_label)
        .ok_or_else(|| format!("missing host window: {host_label}"))?;

    let target = app
        .get_webview(previous_label)
        .ok_or_else(|| format!("missing webview: {previous_label}"))?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == previous_label || is_toolbar_label(label) {
            continue;
        }
        if let Err(e) = webview.hide() {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "restore hide {label} failed: {e}",
            );
        }
    }

    target.show().map_err(|e| e.to_string())?;
    target.set_focus().map_err(|e| e.to_string())?;

    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, previous_label);
    }

    let provider_id = if previous_label == CONVERSATION_DIALOG_LABEL {
        None
    } else {
        app.try_state::<AiWebviewState>()
            .and_then(|s| s.current_provider.lock().unwrap().get(previous_label).copied())
    };
    emit_active_changed(app, provider_id);

    Ok(())
}

async fn swap_to_provider_standalone(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
    from_label: &str,
) -> Result<(), String> {
    let target_label = window_label(provider);

    if from_label == target_label {
        if let Some(win) = app.get_webview_window(&target_label) {
            return focus_window(&win);
        }
        return open_or_focus(app, provider, None).await;
    }

    let target_already_open = app.get_webview_window(&target_label).is_some();
    if !target_already_open {
        if let Some(geom) = read_window_geometry(app, from_label) {
            let state = app.state::<Mutex<AppState>>();
            let mut guard = state.lock().await;
            if let Err(e) = guard.ui_state.set_geometry(&target_label, geom) {
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

async fn swap_to_conversation_dialog_standalone(
    app: &tauri::AppHandle,
    from_label: &str,
) -> Result<(), String> {
    if app.get_window(CONVERSATION_DIALOG_LABEL).is_some() {
        hosted_swap_to_conversation_dialog(app)?;
        if from_label != CONVERSATION_DIALOG_LABEL
            && app.get_window(from_label).is_some()
        {
            close_by_label(app, from_label);
        }
        return Ok(());
    }

    if let Some(geom) = read_window_geometry(app, from_label) {
        let state = app.state::<Mutex<AppState>>();
        let mut guard = state.lock().await;
        if let Err(e) = guard
            .ui_state
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

    let config = DialogConfig {
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

fn read_window_geometry(app: &tauri::AppHandle, label: &str) -> Option<WindowGeometry> {
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

fn navigate_webview(win: &tauri::WebviewWindow, url: &str) -> Result<(), String> {
    let parsed = tauri::Url::parse(url).map_err(|e| e.to_string())?;
    win.navigate(parsed).map_err(|e| e.to_string())
}

fn next_available_label(app: &tauri::AppHandle, provider: &AiProvider) -> String {
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

async fn open_window(
    app: &tauri::AppHandle,
    provider: &'static AiProvider,
    label: &str,
    url: Option<String>,
) -> Result<(), String> {
    let geometry = {
        let state = app.state::<Mutex<AppState>>();
        let guard = state.lock().await;
        guard.ui_state.get_geometry(label)
    };

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));

    let content = url.as_deref().unwrap_or(provider.url);
    let content_url = tauri::Url::parse(content).map_err(|e| e.to_string())?;

    let init_script = initialization_script(provider);
    let reinject = reinject_script();

    let label_owned = label.to_string();
    let app_for_nav = app.clone();
    let label_for_nav = label_owned.clone();

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

    if let Some(webview_state) = app.try_state::<AiWebviewState>() {
        webview_state.set_provider(&label_owned, provider.id);
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
        _ => {}
    });

    focus_window(&win)?;

    log::info!(
        target: "app_lib::services::ai_webview",
        "opened {label_owned} -> {content}",
    );
    Ok(())
}

pub fn close(app: &tauri::AppHandle, provider: &AiProvider) -> Result<(), String> {
    close_by_label(app, &window_label(provider));
    Ok(())
}

fn close_by_label(app: &tauri::AppHandle, label: &str) {
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

const PROMPTHEUS_PROVIDER_ID: &str = "promptheus";

#[derive(Debug)]
enum RouterMessage {
    OpenPalette,
}

fn parse_router_message(url: &tauri::Url) -> Option<RouterMessage> {
    let mut params: HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let kind = params.remove("kind")?;
    match kind.as_str() {
        "open_palette" => Some(RouterMessage::OpenPalette),
        _ => None,
    }
}

async fn handle_router_message(app: &tauri::AppHandle, webview_label: &str, msg: RouterMessage) {
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
    }
}

fn initialization_script(provider: &AiProvider) -> String {
    let provider_id_json =
        serde_json::to_string(provider.id).unwrap_or_else(|_| "\"\"".to_string());
    let sentinel_json =
        serde_json::to_string(ROUTER_SENTINEL).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"
        {dark_mode}
        (function() {{
            if (window.__promptheus && window.__promptheus.__installed) return;
            const g = window.__promptheus = window.__promptheus || {{}};
            g.__installed = true;
            g.providerId = {provider_id_json};
            g.routerSentinel = {sentinel_json};
            {palette}
        }})();
        "#,
        provider_id_json = provider_id_json,
        sentinel_json = sentinel_json,
        palette = PALETTE_KEYBIND_JS,
        dark_mode = DARK_MODE_JS,
    )
}

fn reinject_script() -> String {
    r#"
    (function() {
        if (!window.__promptheus || !window.__promptheus.ensurePaletteKeybind) return;
        window.__promptheus.ensurePaletteKeybind();
    })();
    "#
    .to_string()
}

const DARK_MODE_JS: &str = r##"
(function() {
    try {
        const native = window.matchMedia ? window.matchMedia.bind(window) : null;
        if (native) {
            window.matchMedia = function(query) {
                const result = native(query);
                if (typeof query === "string" && query.indexOf("prefers-color-scheme") !== -1) {
                    const wantsDark = query.indexOf("dark") !== -1;
                    try {
                        Object.defineProperty(result, "matches", { value: wantsDark, configurable: true });
                    } catch (_) {}
                }
                return result;
            };
        }
    } catch (_) {}
    try {
        if (document.documentElement) {
            document.documentElement.style.colorScheme = "dark";
        }
    } catch (_) {}
    try {
        if (window.localStorage) {
            window.localStorage.setItem("theme", "dark");
        }
    } catch (_) {}
})();
"##;

const PALETTE_KEYBIND_JS: &str = r##"
const S = g.routerSentinel;

function sendRouter(params) {
    const qs = new URLSearchParams(params).toString();
    window.location.href = S + "?" + qs;
}

function paletteKeydown(e) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.shiftKey || e.altKey) return;
    const key = typeof e.key === "string" ? e.key.toLowerCase() : "";
    if (key !== "p") return;
    e.preventDefault();
    e.stopPropagation();
    sendRouter({ kind: "open_palette" });
}

function ensurePaletteKeybind() {
    if (g.__paletteInstalled) return;
    g.__paletteInstalled = true;
    document.addEventListener("keydown", paletteKeydown, true);
    window.addEventListener("keydown", paletteKeydown, true);
}

g.ensurePaletteKeybind = ensurePaletteKeybind;
ensurePaletteKeybind();
"##;
