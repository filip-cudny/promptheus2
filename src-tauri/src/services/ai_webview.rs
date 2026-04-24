use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::Mutex;

use super::ai_providers::{self, AiProvider, PROVIDERS};
use super::dialog::{self, focus_window, save_geometry, DialogConfig};
use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

const DEFAULT_WIDTH: f64 = 1000.0;
const DEFAULT_HEIGHT: f64 = 720.0;
const ROUTER_SENTINEL: &str = "https://promptheus-ai-webview-router.invalid/";
const TOOLBAR_ELEMENT_ID: &str = "__promptheus-toolbar";
const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";

#[derive(Default)]
pub struct AiWebviewState {
    current_provider: StdMutex<HashMap<String, &'static str>>,
}

impl AiWebviewState {
    fn set(&self, label: &str, provider_id: &'static str) {
        self.current_provider
            .lock()
            .unwrap()
            .insert(label.to_string(), provider_id);
    }

    fn get(&self, label: &str) -> Option<&'static str> {
        self.current_provider.lock().unwrap().get(label).copied()
    }

    fn remove(&self, label: &str) {
        self.current_provider.lock().unwrap().remove(label);
    }
}

pub fn window_label(provider: &AiProvider) -> String {
    format!("ai-webview-{}", provider.id)
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
                state.set(&label, provider.id);
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

pub async fn swap_to_provider(
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

pub async fn swap_to_conversation_dialog(
    app: &tauri::AppHandle,
    from_label: &str,
) -> Result<(), String> {
    if from_label == CONVERSATION_DIALOG_LABEL {
        if let Some(win) = app.get_webview_window(CONVERSATION_DIALOG_LABEL) {
            return focus_window(&win);
        }
    }

    let target_already_open = app
        .get_webview_window(CONVERSATION_DIALOG_LABEL)
        .is_some();
    if !target_already_open {
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
    }

    if from_label != CONVERSATION_DIALOG_LABEL && app.get_webview_window(from_label).is_some() {
        close_by_label(app, from_label);
    }

    let config = DialogConfig {
        label: CONVERSATION_DIALOG_LABEL,
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat",
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: CONVERSATION_DIALOG_LABEL,
    };
    let (win, _) = dialog::open_or_focus(app, &config).await?;
    focus_window(&win)
}

fn read_window_geometry(app: &tauri::AppHandle, label: &str) -> Option<WindowGeometry> {
    let win = app.get_webview_window(label)?;
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
        webview_state.set(&label_owned, provider.id);
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
                state.remove(&label_for_event);
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
    if let Some(win) = app.get_webview_window("conversation-dialog") {
        if let Err(e) = focus_window(&win) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "failed to focus conversation-dialog: {e}",
            );
        }
    }
}

#[derive(Debug)]
enum RouterMessage {
    BackNav,
    ToolbarAction(ToolbarAction),
}

#[derive(Debug)]
enum ToolbarAction {
    OpenProvider { provider_id: String },
    NewChat,
    OpenInNewWindow,
    OpenPromptheus,
}

fn parse_router_message(url: &tauri::Url) -> Option<RouterMessage> {
    let mut params: HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let kind = params.remove("kind")?;
    match kind.as_str() {
        "back_nav" => Some(RouterMessage::BackNav),
        "toolbar_action" => {
            let action = params.remove("action")?;
            match action.as_str() {
                "open_provider" => {
                    let provider_id = params.remove("provider_id")?;
                    Some(RouterMessage::ToolbarAction(ToolbarAction::OpenProvider {
                        provider_id,
                    }))
                }
                "new_chat" => Some(RouterMessage::ToolbarAction(ToolbarAction::NewChat)),
                "open_in_new_window" => {
                    Some(RouterMessage::ToolbarAction(ToolbarAction::OpenInNewWindow))
                }
                "open_promptheus" => {
                    Some(RouterMessage::ToolbarAction(ToolbarAction::OpenPromptheus))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

async fn handle_router_message(app: &tauri::AppHandle, label: &str, msg: RouterMessage) {
    log::info!(
        target: "app_lib::services::ai_webview",
        "router message on {label}: {msg:?}",
    );
    match msg {
        RouterMessage::BackNav => {
            close_by_label(app, label);
            if let Some(state) = app.try_state::<AiWebviewState>() {
                state.remove(label);
            }
            focus_conversation_dialog(app);
        }
        RouterMessage::ToolbarAction(ToolbarAction::OpenProvider { provider_id }) => {
            let Some(provider) = ai_providers::find(&provider_id) else {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "unknown provider in open_provider: {provider_id}",
                );
                return;
            };
            let Some(win) = app.get_webview_window(label) else {
                return;
            };
            if let Err(e) = navigate_webview(&win, provider.url) {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "switch provider failed: {e}",
                );
                return;
            }
            if let Some(state) = app.try_state::<AiWebviewState>() {
                state.set(label, provider.id);
            }
        }
        RouterMessage::ToolbarAction(ToolbarAction::NewChat) => {
            let Some(provider) = current_provider_for(app, label) else {
                return;
            };
            let Some(win) = app.get_webview_window(label) else {
                return;
            };
            if let Err(e) = navigate_webview(&win, provider.url) {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "new chat failed: {e}",
                );
            }
        }
        RouterMessage::ToolbarAction(ToolbarAction::OpenInNewWindow) => {
            let Some(provider) = current_provider_for(app, label) else {
                return;
            };
            if let Err(e) = open_new_instance(app, provider, None).await {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "open in new window failed: {e}",
                );
            }
        }
        RouterMessage::ToolbarAction(ToolbarAction::OpenPromptheus) => {
            if let Err(e) = swap_to_conversation_dialog(app, label).await {
                log::warn!(
                    target: "app_lib::services::ai_webview",
                    "swap to conversation-dialog failed: {e}",
                );
            }
        }
    }
}

fn current_provider_for(app: &tauri::AppHandle, label: &str) -> Option<&'static AiProvider> {
    let id = app
        .try_state::<AiWebviewState>()
        .and_then(|s| s.get(label))
        .or_else(|| label.strip_prefix("ai-webview-").map(derive_base_provider_id))?;
    ai_providers::find(id)
}

fn derive_base_provider_id(rest: &str) -> &str {
    if let Some(provider) = PROVIDERS.iter().find(|p| p.id == rest) {
        return provider.id;
    }
    if let Some((head, _)) = rest.rsplit_once('-') {
        if let Some(provider) = PROVIDERS.iter().find(|p| p.id == head) {
            return provider.id;
        }
    }
    rest
}

fn initialization_script(provider: &AiProvider) -> String {
    let providers_json = serde_json::to_string(
        &PROVIDERS
            .iter()
            .map(|p| serde_json::json!({ "id": p.id, "name": p.name }))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());
    let provider_id_json =
        serde_json::to_string(provider.id).unwrap_or_else(|_| "\"\"".to_string());
    let sentinel_json =
        serde_json::to_string(ROUTER_SENTINEL).unwrap_or_else(|_| "\"\"".to_string());
    let toolbar_element_id_json =
        serde_json::to_string(TOOLBAR_ELEMENT_ID).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"
        (function() {{
            if (window.__promptheus && window.__promptheus.__installed) return;
            const g = window.__promptheus = window.__promptheus || {{}};
            g.__installed = true;
            g.providerId = {provider_id_json};
            g.providers = {providers_json};
            g.routerSentinel = {sentinel_json};
            g.toolbarId = {toolbar_element_id_json};
            {toolbar}
        }})();
        "#,
        provider_id_json = provider_id_json,
        providers_json = providers_json,
        sentinel_json = sentinel_json,
        toolbar_element_id_json = toolbar_element_id_json,
        toolbar = TOOLBAR_BOOTSTRAP,
    )
}

fn reinject_script() -> String {
    format!(
        r#"
        (function() {{
            if (!window.__promptheus || !window.__promptheus.ensureToolbar) return;
            window.__promptheus.ensureToolbar();
        }})();
        "#
    )
}

const TOOLBAR_BOOTSTRAP: &str = r##"
const S = g.routerSentinel;
const TOOLBAR_ID = g.toolbarId;

function sendAction(params) {
    const qs = new URLSearchParams(params).toString();
    window.location.href = S + "?" + qs;
}

function applyStyle(el, styles) {
    for (const k in styles) el.style.setProperty(k, styles[k]);
}

function makeIconButton(label, title, onClick) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    if (title) b.title = title;
    b.addEventListener("click", onClick);
    applyStyle(b, {
        "background": "rgba(255,255,255,0.08)",
        "color": "#e8e8e8",
        "border": "1px solid rgba(255,255,255,0.15)",
        "border-radius": "6px",
        "padding": "4px 10px",
        "margin": "0 2px",
        "font": "500 12px/1.2 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif",
        "cursor": "pointer",
        "user-select": "none",
        "white-space": "nowrap",
    });
    b.addEventListener("mouseover", () => b.style.setProperty("background", "rgba(255,255,255,0.16)"));
    b.addEventListener("mouseout", () => b.style.setProperty("background", "rgba(255,255,255,0.08)"));
    return b;
}

function buildDropdown() {
    const wrap = document.createElement("div");
    applyStyle(wrap, { "position": "relative", "display": "inline-block" });

    const current = g.providers.find((p) => p.id === g.providerId);
    const btn = makeIconButton((current ? current.name : "AI") + " ▾", "Zmień providera", () => {
        const shown = menu.style.display === "block";
        menu.style.display = shown ? "none" : "block";
    });
    wrap.appendChild(btn);

    const menu = document.createElement("div");
    applyStyle(menu, {
        "position": "absolute",
        "top": "calc(100% + 4px)",
        "left": "0",
        "min-width": "140px",
        "background": "rgba(30,30,30,0.96)",
        "border": "1px solid rgba(255,255,255,0.15)",
        "border-radius": "6px",
        "box-shadow": "0 4px 12px rgba(0,0,0,0.3)",
        "padding": "4px 0",
        "display": "none",
        "z-index": "2147483647",
    });
    const items = [{ id: "__promptheus__", name: "Promptheus", promptheus: true }].concat(g.providers);
    items.forEach((p) => {
        const item = document.createElement("button");
        item.type = "button";
        item.textContent = p.name;
        applyStyle(item, {
            "display": "block",
            "width": "100%",
            "padding": "8px 12px",
            "background": "transparent",
            "border": "none",
            "color": "#e8e8e8",
            "font": "500 12px/1.2 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif",
            "cursor": "pointer",
            "text-align": "left",
            "white-space": "nowrap",
        });
        item.addEventListener("mouseover", () => item.style.setProperty("background", "rgba(255,255,255,0.08)"));
        item.addEventListener("mouseout", () => item.style.setProperty("background", "transparent"));
        item.addEventListener("click", () => {
            menu.style.display = "none";
            if (p.promptheus) {
                sendAction({ kind: "toolbar_action", action: "open_promptheus" });
            } else {
                sendAction({ kind: "toolbar_action", action: "open_provider", provider_id: p.id });
            }
        });
        menu.appendChild(item);
    });
    wrap.appendChild(menu);

    document.addEventListener("click", (e) => {
        if (!wrap.contains(e.target)) menu.style.display = "none";
    });

    return wrap;
}

function buildToolbar() {
    const bar = document.createElement("div");
    bar.id = TOOLBAR_ID;
    applyStyle(bar, {
        "position": "fixed",
        "top": "8px",
        "left": "8px",
        "z-index": "2147483647",
        "display": "flex",
        "align-items": "center",
        "gap": "4px",
        "padding": "4px 6px",
        "background": "rgba(25,25,25,0.92)",
        "border": "1px solid rgba(255,255,255,0.12)",
        "border-radius": "8px",
        "box-shadow": "0 2px 8px rgba(0,0,0,0.35)",
        "backdrop-filter": "blur(8px)",
        "-webkit-backdrop-filter": "blur(8px)",
        "font": "500 12px/1.2 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif",
        "user-select": "none",
    });

    bar.appendChild(makeIconButton("←", "Wróć do czatu", () => sendAction({ kind: "back_nav" })));
    bar.appendChild(buildDropdown());
    bar.appendChild(makeIconButton("+ New chat", "Nowy czat", () => sendAction({ kind: "toolbar_action", action: "new_chat" })));
    bar.appendChild(makeIconButton("⧉", "Otwórz w nowym oknie", () => sendAction({ kind: "toolbar_action", action: "open_in_new_window" })));

    return bar;
}

function ensureToolbar() {
    if (document.getElementById(TOOLBAR_ID)) return;
    const root = document.body || document.documentElement;
    if (!root) return;
    root.appendChild(buildToolbar());
}

g.ensureToolbar = ensureToolbar;

if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", ensureToolbar, { once: true });
} else {
    ensureToolbar();
}
setInterval(ensureToolbar, 1500);
"##;
