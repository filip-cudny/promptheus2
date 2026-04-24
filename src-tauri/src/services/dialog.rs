use tauri::{LogicalPosition, LogicalSize, Manager, Rect, Window, WindowEvent};
use tokio::sync::Mutex;

use super::ai_webview;
use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
pub const SHELL_TOOLBAR_LABEL: &str = "shell-toolbar";
pub const TOOLBAR_HEIGHT: f64 = 40.0;

pub struct DialogConfig {
    pub label: &'static str,
    pub url: String,
    pub title: &'static str,
    pub default_width: f64,
    pub default_height: f64,
    pub geometry_key: &'static str,
}

pub fn uses_custom_titlebar(label: &str) -> bool {
    label == CONVERSATION_DIALOG_LABEL
}

fn logical_window_size(win: &tauri::Window) -> Result<(f64, f64), String> {
    let physical = win.inner_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    Ok((
        physical.width as f64 / scale,
        physical.height as f64 / scale,
    ))
}

pub fn toolbar_layout(logical_w: f64) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    (
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(logical_w, TOOLBAR_HEIGHT),
    )
}

pub fn content_layout(
    logical_w: f64,
    logical_h: f64,
) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let h = (logical_h - TOOLBAR_HEIGHT).max(0.0);
    (LogicalPosition::new(0.0, TOOLBAR_HEIGHT), LogicalSize::new(logical_w, h))
}

#[cfg(target_os = "linux")]
pub fn configure_linux_child_packing(webview: &tauri::Webview, role: LinuxChildRole) {
    let role = role;
    if let Err(e) = webview.with_webview(move |pv| {
        use gtk::prelude::{BoxExt, WidgetExt};
        let wv = pv.inner();
        let Some(parent) = WidgetExt::parent(&wv) else {
            return;
        };
        use gtk::glib::object::Cast;
        let Ok(vbox) = parent.downcast::<gtk::Box>() else {
            return;
        };
        match role {
            LinuxChildRole::Toolbar => {
                vbox.set_child_packing(&wv, false, false, 0, gtk::PackType::Start);
                WidgetExt::set_size_request(&wv, -1, TOOLBAR_HEIGHT as i32);
            }
            LinuxChildRole::Content => {
                vbox.set_child_packing(&wv, true, true, 0, gtk::PackType::Start);
                WidgetExt::set_size_request(&wv, -1, -1);
            }
        }
    }) {
        log::warn!(
            target: "app_lib::services::dialog",
            "configure_linux_child_packing failed for {}: {e}",
            webview.label(),
        );
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
pub enum LinuxChildRole {
    Toolbar,
    Content,
}

#[cfg(not(target_os = "linux"))]
#[derive(Clone, Copy)]
pub enum LinuxChildRole {
    Toolbar,
    Content,
}

#[cfg(not(target_os = "linux"))]
pub fn configure_linux_child_packing(_webview: &tauri::Webview, _role: LinuxChildRole) {}

pub async fn open_or_focus(
    app: &tauri::AppHandle,
    config: &DialogConfig,
) -> Result<(tauri::WebviewWindow, bool), String> {
    if let Some(existing) = app.get_webview_window(config.label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok((existing, false));
    }
    if app.get_window(config.label).is_some() {
        return Err(format!(
            "window {} exists as a multi-webview host; caller must handle reuse explicitly",
            config.label
        ));
    }

    let state = app.state::<Mutex<AppState>>();
    let geometry = state.lock().await.ui_state.get_geometry(config.geometry_key);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((config.default_width, config.default_height));

    let custom_titlebar = uses_custom_titlebar(config.label);

    let mut window_builder = Window::builder(app, config.label)
        .title(config.title)
        .inner_size(width, height)
        .resizable(true);

    #[cfg(target_os = "macos")]
    {
        window_builder = window_builder.decorations(true);
        if custom_titlebar {
            window_builder = window_builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        window_builder = window_builder.decorations(!custom_titlebar);
    }

    if let Some(g) = &geometry {
        window_builder = window_builder.position(g.x, g.y);
    }

    let window = window_builder.build().map_err(|e| e.to_string())?;

    let (logical_w, logical_h) = logical_window_size(&window)?;

    if custom_titlebar {
        let (toolbar_pos, toolbar_size) = toolbar_layout(logical_w);
        let toolbar_builder = tauri::webview::WebviewBuilder::new(
            SHELL_TOOLBAR_LABEL,
            tauri::WebviewUrl::App("shell-toolbar.html".into()),
        )
        .auto_resize();
        let toolbar_webview = window
            .add_child(toolbar_builder, toolbar_pos, toolbar_size)
            .map_err(|e| e.to_string())?;
        configure_linux_child_packing(&toolbar_webview, LinuxChildRole::Toolbar);

        let (content_pos, content_size) = content_layout(logical_w, logical_h);
        let content_builder = tauri::webview::WebviewBuilder::new(
            config.label,
            tauri::WebviewUrl::App(config.url.clone().into()),
        )
        .auto_resize();
        let content_webview = window
            .add_child(content_builder, content_pos, content_size)
            .map_err(|e| e.to_string())?;
        configure_linux_child_packing(&content_webview, LinuxChildRole::Content);

        ai_webview::mark_active_webview(app, config.label, config.label);
    } else {
        let content_builder = tauri::webview::WebviewBuilder::new(
            config.label,
            tauri::WebviewUrl::App(config.url.clone().into()),
        )
        .auto_resize();
        window
            .add_child(
                content_builder,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(logical_w, logical_h),
            )
            .map_err(|e| e.to_string())?;
    }

    let win = app
        .get_webview_window(config.label)
        .ok_or_else(|| format!("failed to attach webview for {}", config.label))?;

    let dock = app.state::<DockManager>();
    dock.dialog_opened(app);

    let app_handle = app.clone();
    let label = config.label;
    let geometry_key = config.geometry_key;
    let resize_label = config.label.to_string();
    let resize_app = app.clone();
    let resize_custom_titlebar = custom_titlebar;

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } => {
            save_geometry(&app_handle, label, geometry_key);
        }
        WindowEvent::Destroyed => {
            ai_webview::cleanup_host_state(&app_handle, label);
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
        WindowEvent::Resized(size) => {
            let Some(host) = resize_app.get_window(&resize_label) else {
                return;
            };
            let scale = host.scale_factor().unwrap_or(1.0);
            let logical_w = size.width as f64 / scale;
            let logical_h = size.height as f64 / scale;
            apply_layout(&host, resize_custom_titlebar, logical_w, logical_h);
        }
        _ => {}
    });

    Ok((win, true))
}

pub fn apply_layout(
    host: &tauri::Window,
    custom_titlebar: bool,
    logical_w: f64,
    logical_h: f64,
) {
    let (toolbar_pos, toolbar_size) = toolbar_layout(logical_w);
    let (content_pos, content_size) = content_layout(logical_w, logical_h);
    let full_pos = LogicalPosition::new(0.0, 0.0);
    let full_size = LogicalSize::new(logical_w, logical_h);

    for webview in host.webviews() {
        let label = webview.label().to_string();
        let (pos, size) = if custom_titlebar && label == SHELL_TOOLBAR_LABEL {
            (toolbar_pos, toolbar_size)
        } else if custom_titlebar {
            (content_pos, content_size)
        } else {
            (full_pos, full_size)
        };

        if let Err(e) = webview.set_bounds(Rect {
            position: pos.into(),
            size: size.into(),
        }) {
            log::warn!(
                target: "app_lib::services::dialog",
                "set_bounds {label} failed: {e}",
            );
        }
    }
}

pub fn save_geometry(app: &tauri::AppHandle, window_label: &str, geometry_key: &str) {
    let Some(win) = app.get_window(window_label) else {
        return;
    };

    let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
        return;
    };

    let scale = win.scale_factor().unwrap_or(1.0);
    let geom = WindowGeometry {
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    };

    let app = app.clone();
    let key = geometry_key.to_string();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Mutex<AppState>>();
        let mut s = state.lock().await;
        if let Err(e) = s.ui_state.set_geometry(&key, geom) {
            log::warn!("failed to save window geometry: {e}");
        }
    });
}

pub fn focus_window(win: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use gtk::glib::object::Cast;
        use gtk::prelude::GtkWindowExt;
        use gtk::prelude::WidgetExt;

        if let Ok(gtk_win) = win.gtk_window() {
            if let Some(gdk_win) = gtk_win.window() {
                if let Ok(x11_win) = gdk_win.downcast::<gdkx11::X11Window>() {
                    let timestamp = gdkx11::functions::x11_get_server_time(&x11_win);
                    log::debug!("focus_window({}): present_with_time({})", win.label(), timestamp);
                    gtk_win.present_with_time(timestamp);
                    return Ok(());
                }
            }
        }
    }

    win.set_focus().map_err(|e| e.to_string())
}

pub fn focus_host_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use gtk::glib::object::Cast;
        use gtk::prelude::GtkWindowExt;
        use gtk::prelude::WidgetExt;

        if let Some(win) = app.get_window(label) {
            if let Ok(gtk_win) = win.gtk_window() {
                if let Some(gdk_win) = gtk_win.window() {
                    if let Ok(x11_win) = gdk_win.downcast::<gdkx11::X11Window>() {
                        let timestamp = gdkx11::functions::x11_get_server_time(&x11_win);
                        gtk_win.present_with_time(timestamp);
                        return Ok(());
                    }
                }
            }
        }
    }

    if let Some(win) = app.get_window(label) {
        return win.set_focus().map_err(|e| e.to_string());
    }
    Err(format!("no window: {label}"))
}
