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
const BORDER_RGBA: (f64, f64, f64, f64) = (1.0, 1.0, 1.0, 0.1);

#[cfg(target_os = "linux")]
fn install_on_main_thread(window: &tauri::Window) {
    use gtk::gdk::WindowState;
    use gtk::glib::prelude::ObjectExt;
    use gtk::glib::Propagation;
    use gtk::prelude::WidgetExt;
    use tauri::Manager;

    let (vbox, gtk_window) = match (window.default_vbox(), window.gtk_window()) {
        (Ok(vbox), Ok(gtk_window)) => (vbox, gtk_window),
        (vbox, gtk_window) => {
            log::warn!(
                target: "app_lib::services::chat_corners",
                "no gtk window or box for {}: box={:?} window={:?}",
                window.label(),
                vbox.err(),
                gtk_window.err(),
            );
            return;
        }
    };

    vbox.connect_draw(|vbox, cr| {
        if is_rounded() {
            let (w, h) = (vbox.allocated_width() as f64, vbox.allocated_height() as f64);
            rounded_rect_path(cr, 0.0, 0.0, w, h, CORNER_RADIUS);
            cr.clip();
        }
        Propagation::Proceed
    });

    vbox.connect_local("draw", true, |values| {
        if is_rounded() {
            let vbox = values[0].get::<gtk::Widget>().ok()?;
            let cr = values[1].get::<gtk::cairo::Context>().ok()?;
            let (w, h) = (vbox.allocated_width() as f64, vbox.allocated_height() as f64);
            let (r, g, b, a) = BORDER_RGBA;
            rounded_rect_path(&cr, 0.5, 0.5, w - 1.0, h - 1.0, CORNER_RADIUS - 0.5);
            cr.set_source_rgba(r, g, b, a);
            cr.set_line_width(1.0);
            if let Err(e) = cr.stroke() {
                log::warn!(target: "app_lib::services::chat_corners", "border stroke failed: {e}");
            }
        }
        Some(false.into())
    });

    let app = window.app_handle().clone();
    let state_vbox = vbox.clone();
    gtk_window.connect_window_state_event(move |_, event| {
        let rounded = rounded_for_state(event.new_window_state());
        if rounded != is_rounded() {
            apply(&app, &state_vbox, rounded);
        }
        Propagation::Proceed
    });

    let initial_state = WidgetExt::window(&gtk_window)
        .map(|w| w.state())
        .unwrap_or_else(WindowState::empty);
    apply(window.app_handle(), &vbox, rounded_for_state(initial_state));
}

#[cfg(target_os = "linux")]
fn apply(app: &tauri::AppHandle, vbox: &gtk::Box, rounded: bool) {
    use gtk::prelude::WidgetExt;
    use tauri::Emitter;

    ROUNDED.store(rounded, Ordering::Relaxed);
    vbox.queue_draw();
    log::debug!(target: "app_lib::services::chat_corners", "rounded={rounded}");
    if let Err(e) = app.emit_to(
        super::dialog::SHELL_TOOLBAR_LABEL,
        "shell:corners",
        serde_json::json!({ "rounded": rounded }),
    ) {
        log::warn!(
            target: "app_lib::services::chat_corners",
            "emit_to({}) shell:corners failed: {e}",
            super::dialog::SHELL_TOOLBAR_LABEL,
        );
    }
}

#[cfg(target_os = "linux")]
fn rounded_rect_path(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::{FRAC_PI_2, PI};

    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, PI + FRAC_PI_2);
    cr.close_path();
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
}
