use std::sync::atomic::{AtomicBool, Ordering};

pub const CORNER_RADIUS: f64 = 10.0;

// One flag for the whole process: it assumes a single chat host window. It stops
// being correct once a second window calls `install`; upgrade path: keep the state
// per window in the closures of that window.
static ROUNDED: AtomicBool = AtomicBool::new(false);

pub fn is_rounded() -> bool {
    ROUNDED.load(Ordering::Relaxed)
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
    use gtk::glib::Propagation;
    use gtk::prelude::WidgetExt;
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

    let app = window.app_handle().clone();
    gtk_window.connect_window_state_event(move |_, event| {
        let rounded = rounded_for_state(event.new_window_state());
        if rounded != is_rounded() {
            apply(&app, rounded);
        }
        Propagation::Proceed
    });

    let initial_state = WidgetExt::window(&gtk_window)
        .map(|w| w.state())
        .unwrap_or_else(WindowState::empty);
    apply(window.app_handle(), rounded_for_state(initial_state));
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
