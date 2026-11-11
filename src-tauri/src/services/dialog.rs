use std::sync::Arc;

use tauri::window::Color;
use tauri::{LogicalPosition, LogicalSize, Manager, Rect, WebviewWindow, Window, WindowEvent};
use tokio::sync::Mutex;

use super::ai_webview;
#[cfg(target_os = "linux")]
use super::chat_corners::{ShadowWidth, RESIZE_BAND_IN_SHADOW};
use super::dock::DockManager;
use super::settings_agent::{self, engine::{lock, SharedSession}};
use super::speech;
use super::ui_state::{UiStateService, WindowGeometry};

pub const CONVERSATION_DIALOG_LABEL: &str = "conversation-dialog";
pub const SETTINGS_DIALOG_LABEL: &str = "settings-dialog";
pub const SHELL_TOOLBAR_LABEL: &str = "shell-toolbar";
pub const TOOLBAR_HEIGHT: f64 = 40.0;
pub const SHELL_BG: Color = Color(0x1e, 0x1e, 0x1e, 0xff);
const TRANSPARENT: Color = Color(0, 0, 0, 0);

pub struct DialogConfig {
    pub label: String,
    pub url: String,
    pub title: String,
    pub default_width: f64,
    pub default_height: f64,
    pub geometry_key: String,
    pub added_width: f64,
}

pub fn window(app: &tauri::AppHandle, label: &str) -> crate::Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| crate::Error::Other(format!("{label} window not found")))
}

pub fn is_conversation_dialog_host(label: &str) -> bool {
    label == CONVERSATION_DIALOG_LABEL
}

pub fn uses_custom_titlebar(label: &str) -> bool {
    is_conversation_dialog_host(label)
}

pub fn is_shell_toolbar_label(label: &str) -> bool {
    label == SHELL_TOOLBAR_LABEL
}

pub fn logical_inner_size(win: &Window) -> Result<LogicalSize<f64>, String> {
    let physical = win.inner_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    Ok(physical.to_logical(scale))
}

fn add_child_webview(
    window: &Window,
    label: &str,
    app_url: &str,
    position: LogicalPosition<f64>,
    size: LogicalSize<f64>,
    background: Color,
) -> Result<tauri::Webview, String> {
    let builder = tauri::webview::WebviewBuilder::new(label, tauri::WebviewUrl::App(app_url.into()))
        .auto_resize()
        .background_color(background);
    window
        .add_child(builder, position, size)
        .map_err(|e| e.to_string())
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
        use gtk::gdk::EventMask;
        use gtk::glib::Propagation;
        use gtk::prelude::{WidgetExt, WidgetExtManual};

        let wv = pv.inner();
        WidgetExtManual::add_events(
            &wv,
            EventMask::BUTTON1_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::POINTER_MOTION_MASK,
        );

        wv.connect_button_press_event(move |wv, event| {
            if let Some(window) = ancestor_gtk_window(wv) {
                begin_edge_resize(&window, event);
            }
            Propagation::Proceed
        });

        wv.connect_motion_notify_event(move |wv, event| {
            if let Some(window) = ancestor_gtk_window(wv) {
                set_edge_cursor(&window, event);
            }
            Propagation::Proceed
        });
    }) {
        log::warn!(
            target: "app_lib::services::dialog",
            "attach_undecorated_resize_handler failed for {label}: {e}",
        );
    }
}

/// Resize from the band of the window's shadow margin outside the child webviews.
/// Connected after tao's handlers on the same window, so its cursor wins.
#[cfg(target_os = "linux")]
pub fn attach_shadow_resize_handler(window: &gtk::Window) {
    use gtk::glib::Propagation;
    use gtk::prelude::WidgetExt;

    window.connect_button_press_event(|window, event| {
        begin_edge_resize(window, event);
        Propagation::Proceed
    });
    window.connect_motion_notify_event(|window, event| {
        set_edge_cursor(window, event);
        Propagation::Proceed
    });
}

#[cfg(target_os = "linux")]
fn resizable_gdk_window(window: &gtk::Window) -> Option<gtk::gdk::Window> {
    use gtk::prelude::{GtkWindowExt, WidgetExt};
    if window.is_decorated() || !window.is_resizable() || window.is_maximized() {
        return None;
    }
    WidgetExt::window(window)
}

#[cfg(target_os = "linux")]
fn begin_edge_resize(window: &gtk::Window, event: &gtk::gdk::EventButton) {
    use gtk::prelude::GtkWindowExt;
    if event.button() != 1 {
        return;
    }
    let Some(gdk_win) = resizable_gdk_window(window) else {
        return;
    };
    let Some(edge) = edge_for_pointer(window, &gdk_win, event.root()) else {
        return;
    };
    let (root_x, root_y) = event.root();
    window.begin_resize_drag(edge, 1, root_x as i32, root_y as i32, event.time());
}

#[cfg(target_os = "linux")]
fn set_edge_cursor(window: &gtk::Window, event: &gtk::gdk::EventMotion) {
    use gtk::gdk::WindowEdge;
    let Some(gdk_win) = resizable_gdk_window(window) else {
        return;
    };
    let cursor_name = match edge_for_pointer(window, &gdk_win, event.root()) {
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
    window: &gtk::Window,
    gdk_win: &gtk::gdk::Window,
    root: (f64, f64),
) -> Option<gtk::gdk::WindowEdge> {
    use gtk::prelude::{BinExt, WidgetExt};
    let (wx, wy) = gdk_win.position();
    let point = (root.0 - wx as f64, root.1 - wy as f64);
    let size = (gdk_win.width() as f64, gdk_win.height() as f64);
    let shadow = window.child().map_or(ShadowWidth::NONE, |child| ShadowWidth {
        left: child.margin_start(),
        right: child.margin_end(),
        top: child.margin_top(),
        bottom: child.margin_bottom(),
    });
    let inset = gdk_win.scale_factor() as f64 * BORDERLESS_RESIZE_INSET;
    edge_at(size, shadow, point, inset)
}

/// Edge under `point` (surface coordinates) of a surface of `size` whose visible
/// window lies inside `shadow`: from `RESIZE_BAND_IN_SHADOW` outside the visible
/// edge to `inset` inside it.
#[cfg(target_os = "linux")]
fn edge_at(
    size: (f64, f64),
    shadow: ShadowWidth,
    point: (f64, f64),
    inset: f64,
) -> Option<gtk::gdk::WindowEdge> {
    use gtk::gdk::WindowEdge;
    let (x, y) = point;
    let band = f64::from(RESIZE_BAND_IN_SHADOW);
    let visible_left = f64::from(shadow.left);
    let visible_top = f64::from(shadow.top);
    let visible_right = size.0 - f64::from(shadow.right);
    let visible_bottom = size.1 - f64::from(shadow.bottom);
    if x < visible_left - band
        || x >= visible_right + band
        || y < visible_top - band
        || y >= visible_bottom + band
    {
        return None;
    }
    let left = x < visible_left + inset;
    let right = x >= visible_right - inset;
    let top = y < visible_top + inset;
    let bottom = y >= visible_bottom - inset;
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
) -> Result<(Window, bool), String> {
    if let Some(existing) = app.get_webview_window(&config.label) {
        let window = existing.as_ref().window();
        activate_window(&window, config.title.clone())?;
        return Ok((window, false));
    }
    if app.get_window(&config.label).is_some() {
        return Err(format!(
            "window {} exists as a multi-webview host; caller must handle reuse explicitly",
            config.label
        ));
    }

    ai_webview::cleanup_host_state(app, &config.label);

    let geometry = app
        .state::<Arc<Mutex<UiStateService>>>()
        .lock()
        .await
        .get_geometry(&config.geometry_key);

    let (width, height) = geometry
        .as_ref()
        .map(|g| (g.width, g.height))
        .unwrap_or((config.default_width, config.default_height));
    let width = width + config.added_width;

    let custom_titlebar = uses_custom_titlebar(&config.label);

    let mut window_builder = Window::builder(app, &config.label)
        .title(&config.title)
        .inner_size(width, height)
        .resizable(true)
        .background_color(SHELL_BG);

    #[cfg(target_os = "linux")]
    let rounded_host = custom_titlebar && super::gnome_shell::is_gnome_wayland();
    #[cfg(not(target_os = "linux"))]
    let rounded_host = false;

    #[cfg(target_os = "linux")]
    if rounded_host {
        window_builder = window_builder.transparent(true).background_color(TRANSPARENT);
    }

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
            dock.dialog_closed(app);
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

    let LogicalSize {
        width: logical_w,
        height: logical_h,
    } = logical_inner_size(&window)?;

    let needs_undecorated_resize = cfg!(target_os = "linux") && custom_titlebar;

    if custom_titlebar {
        let (toolbar_pos, toolbar_size) = toolbar_layout(logical_w);
        let toolbar_webview = add_child_webview(
            &window,
            SHELL_TOOLBAR_LABEL,
            "shell-toolbar.html",
            toolbar_pos,
            toolbar_size,
            if rounded_host { TRANSPARENT } else { SHELL_BG },
        )?;
        configure_linux_child_packing(&toolbar_webview, LinuxChildRole::Toolbar);
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&toolbar_webview);
        }

        let (content_pos, content_size) = content_layout(logical_w, logical_h);
        let content_webview = add_child_webview(
            &window,
            &config.label,
            &config.url,
            content_pos,
            content_size,
            if rounded_host { TRANSPARENT } else { SHELL_BG },
        )?;
        configure_linux_child_packing(&content_webview, LinuxChildRole::Content);
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&content_webview);
        }

        if let Err(e) = ai_webview::show_native_view(app) {
            log::warn!(
                target: "app_lib::services::dialog",
                "show_native_view failed for {}: {e}",
                config.label,
            );
        }
    } else {
        let content_webview = add_child_webview(
            &window,
            &config.label,
            &config.url,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(logical_w, logical_h),
            SHELL_BG,
        )?;
        if needs_undecorated_resize {
            attach_undecorated_resize_handler(&content_webview);
        }
    }

    if rounded_host {
        super::chat_corners::install(&window);
    }

    let app_handle = app.clone();
    let label = config.label.clone();
    let geometry_key = config.geometry_key.clone();
    let resize_label = config.label.clone();
    let destroy_label = config.label.clone();
    let resize_app = app.clone();
    let resize_custom_titlebar = custom_titlebar;

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            save_geometry(&app_handle, &label, &geometry_key);
            if is_conversation_dialog_host(&label) {
                api.prevent_close();
                ai_webview::hide_host(&app_handle);
            }
        }
        WindowEvent::Destroyed => {
            ai_webview::cleanup_host_state(&app_handle, &destroy_label);
            if destroy_label == SETTINGS_DIALOG_LABEL {
                settings_agent::panel::reset(&app_handle);
                speech::composer::cancel_owned_by(&app_handle, SETTINGS_DIALOG_LABEL);
                if let Some(session) = app_handle.try_state::<SharedSession>() {
                    lock(&session).reset();
                }
            }
            let dock = app_handle.state::<DockManager>();
            if is_conversation_dialog_host(&destroy_label) {
                dock.host_destroyed(&app_handle);
            } else {
                dock.dialog_closed(&app_handle);
            }
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

    #[cfg(target_os = "macos")]
    if let Err(e) = window.set_focus() {
        log::warn!(
            target: "app_lib::services::dialog",
            "set_focus failed for {}: {e}",
            config.label,
        );
    }

    Ok((window, true))
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
    let mut geom = WindowGeometry {
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    };
    if window_label == SETTINGS_DIALOG_LABEL {
        geom = settings_agent::panel::geometry_without_panel(geom, settings_agent::panel::added_width(app));
    }
    if is_conversation_dialog_host(window_label) {
        let shadow = super::chat_corners::shadow_width();
        geom.width -= f64::from(shadow.left + shadow.right);
        geom.height -= f64::from(shadow.top + shadow.bottom);
    }

    let app = app.clone();
    let key = geometry_key.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = app
            .state::<Arc<Mutex<UiStateService>>>()
            .lock()
            .await
            .set_geometry(&key, geom)
        {
            log::warn!("failed to save window geometry: {e}");
        }
    });
}

pub fn focus_host_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    log::debug!(target: "app_lib::services::dialog", "focus_host_window({label}): ENTER");

    let Some(win) = app.get_window(label) else {
        return Err(format!("no window: {label}"));
    };

    let title = win.title().map_err(|e| e.to_string())?;
    activate_window(&win, title)
}

/// Brings an already mapped window to the front. On GNOME Wayland Mutter denies
/// `set_focus()` for it, so the shell extension activates the window by `title`;
/// the title is never logged (text previews carry user text).
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn activate_window(win: &Window, title: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use super::gnome_shell;

        if gnome_shell::is_gnome_wayland() {
            if win.is_focused().map_err(|e| e.to_string())? {
                return Ok(());
            }
            let win = win.clone();
            tauri::async_runtime::spawn(async move {
                let activated = match gnome_shell::proxy().await {
                    Ok(proxy) => proxy.activate_window(&title).await,
                    Err(e) => Err(e),
                };
                match activated {
                    Ok(true) => {}
                    Ok(false) => log::warn!("extension found no window to activate"),
                    Err(e) => {
                        log::warn!("ActivateWindow failed, falling back to set_focus: {e}");
                        if let Err(e) = win.set_focus() {
                            log::warn!("set_focus failed: {e}");
                        }
                    }
                }
            });
            return Ok(());
        }
    }
    win.set_focus().map_err(|e| e.to_string())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use gtk::gdk::WindowEdge;

    const SIZE: (f64, f64) = (856.0, 656.0);
    const SHADOW: ShadowWidth = ShadowWidth { left: 28, right: 28, top: 18, bottom: 38 };
    const INSET: f64 = 5.0;

    fn edge(x: f64, y: f64) -> Option<WindowEdge> {
        edge_at(SIZE, SHADOW, (x, y), INSET)
    }

    #[test]
    fn band_outside_the_visible_edge_resizes() {
        assert_eq!(edge(19.0, 300.0), Some(WindowEdge::West));
        assert_eq!(edge(837.0, 300.0), Some(WindowEdge::East));
        assert_eq!(edge(400.0, 8.0), Some(WindowEdge::North));
        assert_eq!(edge(400.0, 627.0), Some(WindowEdge::South));
    }

    #[test]
    fn inset_inside_the_visible_edge_resizes() {
        assert_eq!(edge(32.0, 300.0), Some(WindowEdge::West));
        assert_eq!(edge(824.0, 300.0), Some(WindowEdge::East));
        assert_eq!(edge(400.0, 22.0), Some(WindowEdge::North));
        assert_eq!(edge(400.0, 614.0), Some(WindowEdge::South));
    }

    #[test]
    fn shadow_beyond_the_band_does_not_resize() {
        assert_eq!(edge(17.0, 300.0), None);
        assert_eq!(edge(400.0, 7.0), None);
        assert_eq!(edge(400.0, 628.0), None);
        assert_eq!(edge(838.0, 300.0), None);
    }

    #[test]
    fn inside_the_visible_window_does_not_resize() {
        assert_eq!(edge(33.0, 300.0), None);
        assert_eq!(edge(400.0, 23.0), None);
    }

    #[test]
    fn corners_cover_the_band_and_the_inset() {
        assert_eq!(edge(20.0, 10.0), Some(WindowEdge::NorthWest));
        assert_eq!(edge(32.0, 22.0), Some(WindowEdge::NorthWest));
        assert_eq!(edge(836.0, 10.0), Some(WindowEdge::NorthEast));
        assert_eq!(edge(20.0, 626.0), Some(WindowEdge::SouthWest));
        assert_eq!(edge(836.0, 626.0), Some(WindowEdge::SouthEast));
    }

    #[test]
    fn without_shadow_only_the_inset_resizes() {
        let edge = |x, y| edge_at((800.0, 600.0), ShadowWidth::NONE, (x, y), INSET);
        assert_eq!(edge(0.0, 300.0), Some(WindowEdge::West));
        assert_eq!(edge(4.0, 4.0), Some(WindowEdge::NorthWest));
        assert_eq!(edge(795.0, 595.0), Some(WindowEdge::SouthEast));
        assert_eq!(edge(5.0, 300.0), None);
    }
}
