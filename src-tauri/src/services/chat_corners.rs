use std::sync::atomic::{AtomicBool, Ordering};

pub const CORNER_RADIUS: f64 = 10.0;

// One flag for the whole process: it assumes a single chat host window. It stops
// being correct once a second window calls `install`; upgrade path: keep the state
// per window in the closures of that window.
static ROUNDED: AtomicBool = AtomicBool::new(false);

pub fn is_rounded() -> bool {
    ROUNDED.load(Ordering::Relaxed)
}

/// Transparent margin of the chat host surface around its visible window, per side,
/// in logical px (`gdk_window_set_shadow_width`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowWidth {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl ShadowWidth {
    pub const NONE: ShadowWidth = ShadowWidth { left: 0, right: 0, top: 0, bottom: 0 };
}

// Each side ends 28 px, two standard deviations of GTK's 14 px blur, past the edge of
// the shadow box (offset 10 px down); GTK draws the blur out to 40 px, so the shadow
// is cut at the surface border at about 1 % alpha. Stops being correct when a hard
// edge shows at the surface border; upgrade path: GTK's own extents, 30 top, 40
// sides, 50 bottom.
const ROUNDED_SHADOW_WIDTH: ShadowWidth = ShadowWidth { left: 28, right: 28, top: 18, bottom: 38 };

/// Resize grab outside the visible window edge, inside the shadow margin; it never
/// exceeds the smallest side of `ROUNDED_SHADOW_WIDTH`.
#[cfg(target_os = "linux")]
pub const RESIZE_BAND_IN_SHADOW: i32 = 10;

fn shadow_width_for(rounded: bool) -> ShadowWidth {
    if rounded {
        ROUNDED_SHADOW_WIDTH
    } else {
        ShadowWidth::NONE
    }
}

/// Shadow margin the chat host surface has now; the host's `inner_size` includes it.
pub fn shadow_width() -> ShadowWidth {
    shadow_width_for(is_rounded())
}

#[cfg(target_os = "linux")]
const SHADOW_CLASS: &str = "chat-window-shadow";

/// GTK blurs with a standard deviation about equal to the CSS blur radius, browsers
/// with half of it: 14px here looks like the 28px of the mockup.
#[cfg(target_os = "linux")]
fn shadow_css() -> String {
    format!(
        ".{SHADOW_CLASS} {{ border-radius: {CORNER_RADIUS}px; \
box-shadow: 0 10px 14px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(0, 0, 0, 0.35); }}"
    )
}

#[cfg(target_os = "linux")]
pub fn rounded_for_state(state: gtk::gdk::WindowState) -> bool {
    use gtk::gdk::WindowState;
    !state.intersects(
        WindowState::MAXIMIZED
            | WindowState::FULLSCREEN
            | WindowState::TILED
            | WindowState::TOP_TILED
            | WindowState::RIGHT_TILED
            | WindowState::BOTTOM_TILED
            | WindowState::LEFT_TILED,
    )
}

#[cfg(target_os = "linux")]
pub fn install(window: &tauri::Window) {
    if !super::gnome_shell::is_gnome_wayland() {
        return;
    }
    let host = window.clone();
    if let Err(e) = window.run_on_main_thread(move || install_on_main_thread(&host)) {
        log::warn!(
            target: "app_lib::services::chat_corners",
            "run_on_main_thread failed for {}: {e}",
            window.label(),
        );
    }
}

#[cfg(not(target_os = "linux"))]
pub fn install(_window: &tauri::Window) {}

#[cfg(target_os = "linux")]
fn install_on_main_thread(window: &tauri::Window) {
    use gtk::gdk::WindowState;
    use gtk::glib::object::Cast;
    use gtk::glib::Propagation;
    use gtk::prelude::{CssProviderExt, StyleContextExt, WidgetExt};
    use tauri::Manager;

    let gtk_window = match window.gtk_window() {
        Ok(gtk_window) => gtk_window,
        Err(e) => {
            log::warn!(
                target: "app_lib::services::chat_corners",
                "no gtk window for {}: {e}",
                window.label(),
            );
            return;
        }
    };

    let vbox = match window.default_vbox() {
        Ok(vbox) => vbox,
        Err(e) => {
            log::warn!(
                target: "app_lib::services::chat_corners",
                "no default vbox for {}: {e}",
                window.label(),
            );
            return;
        }
    };
    let provider = gtk::CssProvider::new();
    if let Err(e) = provider.load_from_data(shadow_css().as_bytes()) {
        log::warn!(target: "app_lib::services::chat_corners", "shadow css not loaded: {e}");
    }
    vbox.style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    gtk_window.connect_configure_event(|gtk_window, event| {
        let (width, height) = event.size();
        gtk_window.input_shape_combine_region(input_region(width as i32, height as i32).as_ref());
        false
    });
    super::dialog::attach_shadow_resize_handler(gtk_window.upcast_ref());

    let app = window.app_handle().clone();
    let state_vbox = vbox.clone();
    gtk_window.connect_window_state_event(move |gtk_window, event| {
        let rounded = rounded_for_state(event.new_window_state());
        if rounded != is_rounded() {
            apply(&app, rounded);
            apply_shadow(gtk_window, &state_vbox, rounded);
        }
        Propagation::Proceed
    });

    let initial_state = WidgetExt::window(&gtk_window)
        .map(|w| w.state())
        .unwrap_or_else(WindowState::empty);
    let rounded = rounded_for_state(initial_state);
    apply(window.app_handle(), rounded);
    apply_shadow(&gtk_window, &vbox, rounded);
}

/// The surface grows or shrinks by the margin change on `set_shadow_width`, keeping
/// the visible size; GDK then sends the visible rect as the xdg window geometry.
/// A `gtk_window_set_decorated` call on the host resets the shadow width to 0.
/// `set_shadow_width` stores the size used for the first configure of the next map
/// with the old margins; the same-size `resize` after it recomputes it with the new
/// ones, rounded only: in a maximized or tiled state it would break the restore size.
#[cfg(target_os = "linux")]
fn apply_shadow(gtk_window: &gtk::ApplicationWindow, vbox: &gtk::Box, rounded: bool) {
    use gtk::prelude::{StyleContextExt, WidgetExt};

    let shadow = shadow_width_for(rounded);
    let style = vbox.style_context();
    if rounded {
        style.add_class(SHADOW_CLASS);
    } else {
        style.remove_class(SHADOW_CLASS);
    }
    let old_horizontal = vbox.margin_start() + vbox.margin_end();
    let old_vertical = vbox.margin_top() + vbox.margin_bottom();
    vbox.set_margin_start(shadow.left);
    vbox.set_margin_end(shadow.right);
    vbox.set_margin_top(shadow.top);
    vbox.set_margin_bottom(shadow.bottom);
    match WidgetExt::window(gtk_window) {
        Some(gdk_window) => {
            let (width, height) = (gdk_window.width(), gdk_window.height());
            gdk_window.set_shadow_width(shadow.left, shadow.right, shadow.top, shadow.bottom);
            if rounded {
                gdk_window.resize(
                    width - old_horizontal + shadow.left + shadow.right,
                    height - old_vertical + shadow.top + shadow.bottom,
                );
            }
        }
        None => log::warn!(
            target: "app_lib::services::chat_corners",
            "no gdk window, shadow width not set",
        ),
    }
}

/// Visible window grown by the resize band; clicks on the rest of the shadow reach
/// the window below. Set on every configure, which GDK Wayland also sends after each
/// map: the region is lost with the surface on hide.
#[cfg(target_os = "linux")]
fn input_region(width: i32, height: i32) -> Option<gtk::cairo::Region> {
    if !is_rounded() {
        return None;
    }
    let shadow = ROUNDED_SHADOW_WIDTH;
    let band = RESIZE_BAND_IN_SHADOW;
    Some(gtk::cairo::Region::create_rectangle(&gtk::cairo::RectangleInt::new(
        shadow.left - band,
        shadow.top - band,
        width - shadow.left - shadow.right + 2 * band,
        height - shadow.top - shadow.bottom + 2 * band,
    )))
}

#[cfg(target_os = "linux")]
fn apply(app: &tauri::AppHandle, rounded: bool) {
    use tauri::{Emitter, Manager};

    use super::dialog::{CONVERSATION_DIALOG_LABEL, SHELL_TOOLBAR_LABEL};

    ROUNDED.store(rounded, Ordering::Relaxed);
    log::debug!(target: "app_lib::services::chat_corners", "rounded={rounded}");
    for label in [SHELL_TOOLBAR_LABEL, CONVERSATION_DIALOG_LABEL] {
        if let Err(e) = app.emit_to(label, "shell:corners", serde_json::json!({ "rounded": rounded })) {
            log::warn!(
                target: "app_lib::services::chat_corners",
                "emit_to({label}) shell:corners failed: {e}",
            );
        }
    }
    let Some(host) = app.get_window(CONVERSATION_DIALOG_LABEL) else {
        return;
    };
    let script = provider_corners_script(rounded);
    for webview in host.webviews() {
        let label = webview.label();
        if label == SHELL_TOOLBAR_LABEL || label == CONVERSATION_DIALOG_LABEL {
            continue;
        }
        if let Err(e) = webview.eval(&script) {
            log::warn!(
                target: "app_lib::services::chat_corners",
                "corners eval failed for {label}: {e}",
            );
        }
    }
}

const STYLE_ELEMENT_ID: &str = "__promptheus-corners";

const PROVIDER_CORNERS_CSS: &str = "html { clip-path: inset(0 round 0 0 __R__px __R__px) !important; } \
html::after { content: \"\"; position: fixed; inset: 0; box-sizing: border-box; \
border: 1px solid rgba(255, 255, 255, 0.1); border-top: none; border-radius: 0 0 __R__px __R__px; \
pointer-events: none; z-index: 2147483647; }";

pub fn provider_corners_script(rounded: bool) -> String {
    if !rounded {
        return format!(
            "(function(){{var e=document.getElementById('{STYLE_ELEMENT_ID}');if(e)e.remove();}})();"
        );
    }
    let css = PROVIDER_CORNERS_CSS.replace("__R__", &CORNER_RADIUS.to_string());
    let css_literal = serde_json::Value::String(css);
    format!(
        "(function(){{function put(){{if(document.getElementById('{STYLE_ELEMENT_ID}'))return;\
var s=document.createElement('style');s.id='{STYLE_ELEMENT_ID}';s.textContent={css_literal};\
document.documentElement.appendChild(s);}}\
if(document.documentElement)put();else document.addEventListener('DOMContentLoaded',put,{{once:true}});}})();"
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use gtk::gdk::WindowState;

    #[test]
    fn each_square_flag_alone_is_not_rounded() {
        for flag in [
            WindowState::MAXIMIZED,
            WindowState::FULLSCREEN,
            WindowState::TILED,
            WindowState::TOP_TILED,
            WindowState::RIGHT_TILED,
            WindowState::BOTTOM_TILED,
            WindowState::LEFT_TILED,
        ] {
            assert!(!rounded_for_state(flag), "{flag:?}");
        }
    }

    #[test]
    fn square_flag_with_focus_is_not_rounded() {
        assert!(!rounded_for_state(WindowState::FOCUSED | WindowState::TOP_TILED));
    }

    #[test]
    fn empty_state_is_rounded() {
        assert!(rounded_for_state(WindowState::empty()));
    }

    #[test]
    fn focused_is_rounded() {
        assert!(rounded_for_state(WindowState::FOCUSED));
    }

    #[test]
    fn resizable_flags_do_not_square_the_window() {
        assert!(rounded_for_state(
            WindowState::TOP_RESIZABLE
                | WindowState::RIGHT_RESIZABLE
                | WindowState::BOTTOM_RESIZABLE
                | WindowState::LEFT_RESIZABLE
                | WindowState::FOCUSED
        ));
    }

    #[test]
    fn rounded_script_installs_clip_and_border_style() {
        let script = provider_corners_script(true);
        assert!(script.contains("__promptheus-corners"));
        assert!(script.contains("clip-path: inset(0 round 0 0 10px 10px)"));
        assert!(script.contains("rgba(255, 255, 255, 0.1)"));
    }

    #[test]
    fn square_script_only_removes_the_style() {
        let script = provider_corners_script(false);
        assert!(script.contains("__promptheus-corners"));
        assert!(!script.contains("clip-path"));
    }
}
