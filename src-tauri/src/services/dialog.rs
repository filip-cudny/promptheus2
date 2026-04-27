use tauri::window::Color;
use tauri::{LogicalPosition, LogicalSize, Manager, Rect, Window, WindowEvent};
use tokio::sync::Mutex;

use super::ai_webview;
use super::dock::DockManager;
use super::ui_state::WindowGeometry;
use crate::commands::settings::AppState;

pub const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
pub const SHELL_TOOLBAR_LABEL: &str = "shell-toolbar";
pub const TOOLBAR_HEIGHT: f64 = 40.0;
const SHELL_BG: Color = Color(0x1e, 0x1e, 0x1e, 0xff);

pub struct DialogConfig {
    pub label: String,
    pub url: String,
    pub title: String,
    pub default_width: f64,
    pub default_height: f64,
    pub geometry_key: String,
}

pub fn is_conversation_dialog_host(label: &str) -> bool {
    label == CONVERSATION_DIALOG_LABEL
        || label.starts_with(&format!("{CONVERSATION_DIALOG_LABEL}-"))
}

pub fn uses_custom_titlebar(label: &str) -> bool {
    is_conversation_dialog_host(label)
}

pub fn shell_toolbar_label_for(host_label: &str) -> String {
    if host_label == CONVERSATION_DIALOG_LABEL {
        SHELL_TOOLBAR_LABEL.to_string()
    } else if let Some(suffix) =
        host_label.strip_prefix(&format!("{CONVERSATION_DIALOG_LABEL}-"))
    {
        format!("{SHELL_TOOLBAR_LABEL}-{suffix}")
    } else {
        format!("{host_label}-toolbar")
    }
}

pub fn is_shell_toolbar_label(label: &str) -> bool {
    label == SHELL_TOOLBAR_LABEL
        || label.starts_with(&format!("{SHELL_TOOLBAR_LABEL}-"))
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
const BORDERLESS_RESIZE_INSET: f64 = 5.0;

#[cfg(target_os = "linux")]
pub fn attach_undecorated_resize_handler(webview: &tauri::Webview) {
    let label = webview.label().to_string();
    if let Err(e) = webview.with_webview(move |pv| {
        use gtk::gdk::{EventMask, WindowEdge};
        use gtk::glib::object::Cast;
        use gtk::glib::Propagation;
        use gtk::prelude::{GtkWindowExt, WidgetExt, WidgetExtManual};

        let wv = pv.inner();
        WidgetExtManual::add_events(
            &wv,
            EventMask::BUTTON1_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::POINTER_MOTION_MASK,
        );

        wv.connect_button_press_event(move |wv, event| {
            if event.button() != 1 {
                return Propagation::Proceed;
            }
            let Some(window) = ancestor_gtk_window(wv) else {
                return Propagation::Proceed;
            };
            if window.is_decorated() || !window.is_resizable() || window.is_maximized() {
                return Propagation::Proceed;
            }
            let Some(gdk_win) = WidgetExt::window(&window) else {
                return Propagation::Proceed;
            };
            let Some(edge) = edge_for_pointer(&gdk_win, event.root()) else {
                return Propagation::Proceed;
            };
            let (root_x, root_y) = event.root();
            window.begin_resize_drag(edge, 1, root_x as i32, root_y as i32, event.time());
            Propagation::Proceed
        });

        wv.connect_motion_notify_event(move |wv, event| {
            let Some(window) = ancestor_gtk_window(wv) else {
                return Propagation::Proceed;
            };
            if window.is_decorated() || !window.is_resizable() || window.is_maximized() {
                return Propagation::Proceed;
            }
            let Some(gdk_win) = WidgetExt::window(&window) else {
                return Propagation::Proceed;
            };
            let cursor_name = match edge_for_pointer(&gdk_win, event.root()) {
                Some(WindowEdge::North) => "n-resize",
                Some(WindowEdge::South) => "s-resize",
                Some(WindowEdge::East) => "e-resize",
                Some(WindowEdge::West) => "w-resize",
                Some(WindowEdge::NorthEast) => "ne-resize",
                Some(WindowEdge::NorthWest) => "nw-resize",
                Some(WindowEdge::SouthEast) => "se-resize",
                Some(WindowEdge::SouthWest) => "sw-resize",
                _ => "default",
            };
            let cursor = gtk::gdk::Cursor::from_name(&gdk_win.display(), cursor_name);
            gdk_win.set_cursor(cursor.as_ref());
            Propagation::Proceed
        });
    }) {
        log::warn!(
            target: "app_lib::services::dialog",
            "attach_undecorated_resize_handler failed for {label}: {e}",
        );
    }
}

#[cfg(target_os = "linux")]
fn ancestor_gtk_window(wv: &webkit2gtk::WebView) -> Option<gtk::Window> {
    use gtk::glib::object::Cast;
    use gtk::prelude::WidgetExt;
    let mut node: Option<gtk::Widget> = WidgetExt::parent(wv);
    while let Some(widget) = node {
        if let Ok(window) = widget.clone().downcast::<gtk::Window>() {
            return Some(window);
        }
        node = WidgetExt::parent(&widget);
    }
    None
}

#[cfg(target_os = "linux")]
fn edge_for_pointer(
    gdk_win: &gtk::gdk::Window,
    root: (f64, f64),
) -> Option<gtk::gdk::WindowEdge> {
    use gtk::gdk::{prelude::*, WindowEdge};
    let (wx, wy) = gdk_win.position();
    let (cx, cy) = (root.0 - wx as f64, root.1 - wy as f64);
    let (w, h) = (gdk_win.width() as f64, gdk_win.height() as f64);
    let border = gdk_win.scale_factor() as f64 * BORDERLESS_RESIZE_INSET;
    let left = cx < border;
    let right = cx >= w - border;
    let top = cy < border;
    let bottom = cy >= h - border;
    Some(match (left, right, top, bottom) {
        (true, _, true, _) => WindowEdge::NorthWest,
        (_, true, true, _) => WindowEdge::NorthEast,
        (true, _, _, true) => WindowEdge::SouthWest,
        (_, true, _, true) => WindowEdge::SouthEast,
        (true, _, _, _) => WindowEdge::West,
        (_, true, _, _) => WindowEdge::East,
        (_, _, true, _) => WindowEdge::North,
        (_, _, _, true) => WindowEdge::South,
        _ => return None,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn attach_undecorated_resize_handler(_webview: &tauri::Webview) {}

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
    if let Some(existing) = app.get_webview_window(&config.label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok((existing, false));
    }
    if app.get_window(&config.label).is_some() {
        return Err(format!(
            "window {} exists as a multi-webview host; caller must handle reuse explicitly",
            config.label
        ));
    }

    ai_webview::cleanup_host_state(app, &config.label);

    let state = app.state::<Mutex<AppState>>();
    let geometry = state.lock().await.ui_state.get_geometry(&config.geometry_key);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((config.default_width, config.default_height));

    let custom_titlebar = uses_custom_titlebar(&config.label);
    let toolbar_label = shell_toolbar_label_for(&config.label);

    let mut window_builder = Window::builder(app, &config.label)
        .title(&config.title)
        .inner_size(width, height)
        .resizable(true)
        .background_color(SHELL_BG);

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

    let dock = app.state::<DockManager>();
    dock.dialog_opened(app);

    let window = match window_builder.build() {
        Ok(w) => w,
        Err(e) => {
            dock.rollback_open(app);
            return Err(e.to_string());
        }
    };

    #[cfg(target_os = "macos")]
    if custom_titlebar {
        let main_thread_window = window.clone();
        let label_for_log = config.label.clone();
        if let Err(e) = window.run_on_main_thread(move || {
            if let Err(err) = super::macos_panel::enable_fullscreen_primary(&main_thread_window) {
                log::warn!(
                    target: "app_lib::services::dialog",
                    "enable_fullscreen_primary failed for {label_for_log}: {err}",
                );
            }
        }) {
            log::warn!(
                target: "app_lib::services::dialog",
                "run_on_main_thread for enable_fullscreen_primary failed for {}: {e}",
                config.label,
            );
        }
    }

    let (logical_w, logical_h) = logical_window_size(&window)?;

    let needs_undecorated_resize = cfg!(target_os = "linux") && custom_titlebar;

    if custom_titlebar {
        let (toolbar_pos, toolbar_size) = toolbar_layout(logical_w);
        let toolbar_builder = tauri::webview::WebviewBuilder::new(
            &toolbar_label,
            tauri::WebviewUrl::App("shell-toolbar.html".into()),
        )
        .auto_resize()
        .background_color(SHELL_BG);
        let toolbar_webview = window
            .add_child(toolbar_builder, toolbar_pos, toolbar_size)
            .map_err(|e| e.to_string())?;
        configure_linux_child_packing(&toolbar_webview, LinuxChildRole::Toolbar);
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&toolbar_webview);
        }

        let (content_pos, content_size) = content_layout(logical_w, logical_h);
        let content_builder = tauri::webview::WebviewBuilder::new(
            &config.label,
            tauri::WebviewUrl::App(config.url.clone().into()),
        )
        .auto_resize()
        .background_color(SHELL_BG);
        let content_webview = window
            .add_child(content_builder, content_pos, content_size)
            .map_err(|e| e.to_string())?;
        configure_linux_child_packing(&content_webview, LinuxChildRole::Content);
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&content_webview);
        }

        ai_webview::mark_active_webview(app, &config.label, &config.label);
    } else {
        let content_builder = tauri::webview::WebviewBuilder::new(
            &config.label,
            tauri::WebviewUrl::App(config.url.clone().into()),
        )
        .auto_resize()
        .background_color(SHELL_BG);
        let content_webview = window
            .add_child(
                content_builder,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(logical_w, logical_h),
            )
            .map_err(|e| e.to_string())?;
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&content_webview);
        }
    }

    let win = app
        .get_webview_window(&config.label)
        .ok_or_else(|| format!("failed to attach webview for {}", config.label))?;

    let app_handle = app.clone();
    let label = config.label.clone();
    let geometry_key = config.geometry_key.clone();
    let resize_label = config.label.clone();
    let destroy_label = config.label.clone();
    let resize_app = app.clone();
    let resize_custom_titlebar = custom_titlebar;

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } => {
            save_geometry(&app_handle, &label, &geometry_key);
        }
        WindowEvent::Destroyed => {
            ai_webview::cleanup_host_state(&app_handle, &destroy_label);
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
        let (pos, size) = if custom_titlebar && is_shell_toolbar_label(&label) {
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

pub async fn seed_geometry_from(
    app: &tauri::AppHandle,
    source_label: &str,
    target_key: &str,
) {
    let Some(win) = app.get_window(source_label) else {
        log::debug!(
            target: "app_lib::services::dialog",
            "seed_geometry_from: no source window {source_label}",
        );
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

    let state = app.state::<Mutex<AppState>>();
    let mut guard = state.lock().await;
    if let Err(e) = guard.ui_state.set_geometry(target_key, geom) {
        log::warn!(
            target: "app_lib::services::dialog",
            "seed_geometry_from {source_label} -> {target_key} failed: {e}",
        );
    }
}

pub fn focus_window(win: &tauri::WebviewWindow) -> Result<(), String> {
    let label = win.label().to_string();
    log::debug!(target: "app_lib::services::dialog", "focus_window({label}): ENTER");
    #[cfg(target_os = "linux")]
    {
        use gtk::glib::object::Cast;
        use gtk::prelude::GtkWindowExt;
        use gtk::prelude::WidgetExt;

        let t0 = std::time::Instant::now();
        if let Ok(gtk_win) = win.gtk_window() {
            log::debug!(
                target: "app_lib::services::dialog",
                "focus_window({label}): gtk_window OK in {:?}",
                t0.elapsed(),
            );
            if let Some(gdk_win) = gtk_win.window() {
                if let Ok(x11_win) = gdk_win.downcast::<gdkx11::X11Window>() {
                    let t1 = std::time::Instant::now();
                    let timestamp = gdkx11::functions::x11_get_server_time(&x11_win);
                    log::debug!(
                        target: "app_lib::services::dialog",
                        "focus_window({label}): x11_get_server_time={timestamp} in {:?}",
                        t1.elapsed(),
                    );
                    let t2 = std::time::Instant::now();
                    gtk_win.present_with_time(timestamp);
                    log::debug!(
                        target: "app_lib::services::dialog",
                        "focus_window({label}): present_with_time done in {:?}",
                        t2.elapsed(),
                    );
                    return Ok(());
                }
            }
        }
    }

    win.set_focus().map_err(|e| e.to_string())
}

pub fn focus_host_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    log::debug!(target: "app_lib::services::dialog", "focus_host_window({label}): ENTER");
    #[cfg(target_os = "linux")]
    {
        use gtk::glib::object::Cast;
        use gtk::prelude::GtkWindowExt;
        use gtk::prelude::WidgetExt;

        if let Some(win) = app.get_window(label) {
            let t0 = std::time::Instant::now();
            if let Ok(gtk_win) = win.gtk_window() {
                log::debug!(
                    target: "app_lib::services::dialog",
                    "focus_host_window({label}): gtk_window OK in {:?}",
                    t0.elapsed(),
                );
                if let Some(gdk_win) = gtk_win.window() {
                    if let Ok(x11_win) = gdk_win.downcast::<gdkx11::X11Window>() {
                        let t1 = std::time::Instant::now();
                        let timestamp = gdkx11::functions::x11_get_server_time(&x11_win);
                        log::debug!(
                            target: "app_lib::services::dialog",
                            "focus_host_window({label}): x11_get_server_time={timestamp} in {:?}",
                            t1.elapsed(),
                        );
                        let t2 = std::time::Instant::now();
                        gtk_win.present_with_time(timestamp);
                        log::debug!(
                            target: "app_lib::services::dialog",
                            "focus_host_window({label}): present_with_time done in {:?}",
                            t2.elapsed(),
                        );
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
