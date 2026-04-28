use std::collections::{HashMap, HashSet};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::window::Color;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::Mutex;

use crate::models::settings::WebviewProvider;
use crate::services::notification::NotificationLevel;
use super::dialog::{
    self, attach_undecorated_resize_handler, configure_linux_child_packing, content_layout,
    focus_host_window, focus_window, is_conversation_dialog_host, is_shell_toolbar_label,
    save_geometry, shell_toolbar_label_for, uses_custom_titlebar, DialogConfig, LinuxChildRole,
};
use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

const DEFAULT_WIDTH: f64 = 1000.0;
const DEFAULT_HEIGHT: f64 = 720.0;
const AI_WEBVIEW_BG: Color = Color(0x1e, 0x1e, 0x1e, 0xff);
const ROUTER_SENTINEL: &str = "https://promptheus-ai-webview-router.invalid/";
const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
const CONVERSATION_DIALOG_TITLE: &str = "Promptheus — chat";

const COLD_SUSPEND_IDLE_THRESHOLD: Duration = Duration::from_secs(90 * 60);
pub const COLD_SUSPEND_POLL_INTERVAL: Duration = Duration::from_secs(60);
const UNRESPONSIVE_GRACE_SECONDS: u32 = 20;

fn is_suspendable_webview_label(label: &str) -> bool {
    label.starts_with("ai-webview-") || label.contains("::ai-webview-")
}

#[derive(Default)]
pub struct AiWebviewState {
    hosted: StdMutex<HashMap<String, HashSet<String>>>,
    current_provider: StdMutex<HashMap<String, String>>,
    provider_snapshots: StdMutex<HashMap<String, WebviewProvider>>,
    active_webview: StdMutex<HashMap<String, String>>,
    previous_active: StdMutex<HashMap<String, String>>,
    palette_open: StdMutex<HashMap<String, bool>>,
    pending_provider: StdMutex<HashMap<String, String>>,
    last_active: StdMutex<HashMap<String, Instant>>,
    suspended: StdMutex<HashMap<String, String>>,
}

impl AiWebviewState {
    fn set_provider(&self, label: &str, provider: &WebviewProvider) {
        self.current_provider
            .lock()
            .unwrap()
            .insert(label.to_string(), provider.id.clone());
        self.provider_snapshots
            .lock()
            .unwrap()
            .insert(label.to_string(), provider.clone());
    }

    fn remove_provider(&self, label: &str) {
        self.current_provider.lock().unwrap().remove(label);
        self.provider_snapshots.lock().unwrap().remove(label);
    }

    fn snapshot(&self, label: &str) -> Option<WebviewProvider> {
        self.provider_snapshots.lock().unwrap().get(label).cloned()
    }

    fn has_hosted_child(&self, host_label: &str, provider_id: &str) -> bool {
        self.hosted
            .lock()
            .unwrap()
            .get(host_label)
            .map(|set| set.contains(provider_id))
            .unwrap_or(false)
    }

    fn mark_hosted_child(&self, host_label: &str, provider_id: &str) {
        self.hosted
            .lock()
            .unwrap()
            .entry(host_label.to_string())
            .or_default()
            .insert(provider_id.to_string());
    }

    fn unmark_hosted_child(&self, host_label: &str, provider_id: &str) {
        if let Some(set) = self.hosted.lock().unwrap().get_mut(host_label) {
            set.remove(provider_id);
        }
    }

    fn drop_host(&self, host_label: &str) {
        self.hosted.lock().unwrap().remove(host_label);
        self.active_webview.lock().unwrap().remove(host_label);
        self.previous_active.lock().unwrap().remove(host_label);
        self.palette_open.lock().unwrap().remove(host_label);
        let prefix = format!("{host_label}::");
        self.last_active
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix) && k != host_label);
        self.suspended
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix) && k != host_label);
    }

    pub fn set_pending_provider(&self, host_label: &str, provider_id: &str) {
        self.pending_provider
            .lock()
            .unwrap()
            .insert(host_label.to_string(), provider_id.to_string());
    }

    pub fn take_pending_provider(&self, host_label: &str) -> Option<String> {
        self.pending_provider.lock().unwrap().remove(host_label)
    }

    fn set_active(&self, host_label: &str, webview_label: &str) {
        self.active_webview
            .lock()
            .unwrap()
            .insert(host_label.to_string(), webview_label.to_string());
        self.last_active
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), Instant::now());
    }

    fn touch_last_active(&self, webview_label: &str) {
        self.last_active
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), Instant::now());
    }

    pub fn is_suspended(&self, webview_label: &str) -> bool {
        self.suspended.lock().unwrap().contains_key(webview_label)
    }

    fn mark_suspended(&self, webview_label: &str, restore_url: String) {
        self.suspended
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), restore_url);
    }

    fn take_suspended_url(&self, webview_label: &str) -> Option<String> {
        self.suspended.lock().unwrap().remove(webview_label)
    }

    fn unmark_suspended(&self, webview_label: &str) {
        self.suspended.lock().unwrap().remove(webview_label);
    }

    fn cold_suspend_candidates(&self, idle_threshold: Duration) -> Vec<String> {
        let now = Instant::now();
        let active: HashSet<String> = self
            .active_webview
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        let suspended = self.suspended.lock().unwrap();
        self.last_active
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(label, last)| {
                if !is_suspendable_webview_label(label) {
                    return None;
                }
                if active.contains(label) {
                    return None;
                }
                if suspended.contains_key(label) {
                    return None;
                }
                if now.duration_since(*last) < idle_threshold {
                    return None;
                }
                Some(label.clone())
            })
            .collect()
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
        .cloned();
    provider
}

pub fn window_label(provider: &WebviewProvider) -> String {
    format!("ai-webview-{}", provider.id)
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
        glib::timeout_add_seconds_local_once(UNRESPONSIVE_GRACE_SECONDS, move || {
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
                "web process unresponsive >{UNRESPONSIVE_GRACE_SECONDS}s, restarting: webview={label}",
            );
            wv.terminate_web_process();
            wv.reload();
            let provider_name = provider_display_name(&app, &label);
            emit_provider_lifecycle_toast(
                &app,
                "provider_restarted",
                NotificationLevel::Warning,
                format!("{provider_name} was unresponsive"),
                Some(format!("Restarted to recover (was unresponsive {UNRESPONSIVE_GRACE_SECONDS}s+)")),
            );
        });
    });
}

fn provider_display_name(app: &tauri::AppHandle, webview_label: &str) -> String {
    app.try_state::<AiWebviewState>()
        .and_then(|s| s.snapshot(webview_label))
        .map(|p| p.name)
        .unwrap_or_else(|| "Provider".to_string())
}

fn resume_if_suspended(app: &tauri::AppHandle, webview_label: &str) {
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

fn emit_provider_lifecycle_toast(
    app: &tauri::AppHandle,
    event_name: &'static str,
    level: NotificationLevel,
    title: String,
    message: Option<String>,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Mutex<AppState>>();
        let guard = state.lock().await;
        let settings = guard.config.settings().notifications.clone();
        if let Err(e) = guard
            .notifications
            .notify(event_name, level, title, message, &settings)
        {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "lifecycle toast emit failed: {e}",
            );
        }
    });
}

fn enable_media_for_window(win: &tauri::WebviewWindow) {
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

fn enable_media_for_webview(webview: &tauri::Webview) {
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

fn hosted_child_label(host_label: &str, provider: &WebviewProvider) -> String {
    format!("{host_label}::ai-webview-{}", provider.id)
}

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
            resume_if_suspended(app, &label);
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

async fn hosted_swap_to_provider(
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

fn hosted_swap_to_conversation_dialog(
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

    if state.is_palette_open(host_label) {
        log::debug!(
            target: "app_lib::services::ai_webview",
            "swap_to_palette: already open host={host_label} (no-op)",
        );
        if let Some(cd) = app.get_webview(host_label) {
            let _ = cd.set_focus();
        }
        return Ok(());
    }

    let previous = state
        .get_active(host_label)
        .unwrap_or_else(|| host_label.to_string());
    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_to_palette: previous_active={previous} host={host_label}",
    );
    state.set_previous_active(host_label, &previous);
    state.set_palette_open(host_label, true);

    let cd = app
        .get_webview(host_label)
        .ok_or_else(|| format!("missing webview: {host_label}"))?;

    for webview in host.webviews() {
        let label = webview.label();
        if label == host_label || is_toolbar_label(label) {
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
    let toolbar_label = shell_toolbar_label_for(host_label);
    if let Err(e) = app.emit_to(host_label, "shell:palette-opened", payload.clone()) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "emit_to({host_label}) shell:palette-opened failed: {e}",
        );
    }
    if let Err(e) = app.emit_to(toolbar_label.as_str(), "shell:palette-opened", payload) {
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

    if !state.is_palette_open(host_label) {
        log::debug!(
            target: "app_lib::services::ai_webview",
            "swap_from_palette: palette not open host={host_label} (no-op)",
        );
        return Ok(());
    }

    state.set_palette_open(host_label, false);

    let previous = state
        .take_previous_active(host_label)
        .unwrap_or_else(|| host_label.to_string());
    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_from_palette: previous_active={previous} host={host_label}",
    );

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
                return restore_previous_webview(app, host_label, &previous);
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
                "swap_from_palette: no selection, restoring previous={previous} host={host_label}",
            );
            restore_previous_webview(app, host_label, &previous)?;
        }
    }

    log::debug!(
        target: "app_lib::services::ai_webview",
        "swap_from_palette: DONE host={host_label}",
    );
    Ok(())
}

pub async fn lookup_webview_provider(
    app: &tauri::AppHandle,
    id: &str,
) -> Option<WebviewProvider> {
    let state = app.state::<Mutex<AppState>>();
    let guard = state.lock().await;
    guard.config.settings().find_webview_provider(id).cloned()
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

    resume_if_suspended(app, previous_label);
    target.show().map_err(|e| e.to_string())?;
    target.set_focus().map_err(|e| e.to_string())?;

    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.set_active(host_label, previous_label);
    }

    let provider_id = if previous_label == host_label {
        None
    } else {
        app.try_state::<AiWebviewState>()
            .and_then(|s| s.current_provider.lock().unwrap().get(previous_label).cloned())
    };
    emit_active_changed(app, host_label, provider_id.as_deref());

    Ok(())
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
        hosted_swap_to_conversation_dialog(app, CONVERSATION_DIALOG_LABEL)?;
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

async fn open_window(
    app: &tauri::AppHandle,
    provider: WebviewProvider,
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

    let content = url.as_deref().unwrap_or(provider.url.as_str()).to_string();
    let content_url = tauri::Url::parse(&content).map_err(|e| e.to_string())?;

    let init_script = initialization_script(&provider);
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
        _ => {}
    });

    focus_window(&win)?;

    log::info!(
        target: "app_lib::services::ai_webview",
        "opened {label_owned} -> {content}",
    );
    Ok(())
}

pub fn close(app: &tauri::AppHandle, provider_id: &str) -> Result<(), String> {
    let label = format!("ai-webview-{provider_id}");
    close_by_label(app, &label);
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.remove_provider(&label);
    }
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

fn initialization_script(provider: &WebviewProvider) -> String {
    let provider_id_json =
        serde_json::to_string(&provider.id).unwrap_or_else(|_| "\"\"".to_string());
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
