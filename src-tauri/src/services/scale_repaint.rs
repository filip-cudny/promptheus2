use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib::{self, ObjectExt};
use gtk::prelude::*;

const DEBOUNCE: Duration = Duration::from_millis(300);
const RESTORE_DELAY: Duration = Duration::from_millis(100);
const SETTLE_DELAY: Duration = Duration::from_millis(100);

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("scale-repaint")
        .on_window_ready(|window| match window.gtk_window() {
            Ok(gtk_window) => watch_scale_factor(&gtk_window),
            Err(error) => log::warn!(
                "scale repaint not installed for window '{}': {error}",
                window.label()
            ),
        })
        .build()
}

fn watch_scale_factor(window: &gtk::ApplicationWindow) {
    let generation = Rc::new(Cell::new(0u64));
    let nudging = Rc::new(Cell::new(false));
    window.connect_scale_factor_notify(move |window| {
        if nudging.get() {
            return;
        }
        generation.set(generation.get() + 1);
        let scheduled = generation.get();
        let weak = window.downgrade();
        let generation = generation.clone();
        let nudging = nudging.clone();
        glib::timeout_add_local_once(DEBOUNCE, move || {
            if generation.get() != scheduled {
                return;
            }
            if let Some(window) = weak.upgrade() {
                nudge(&window, &nudging);
            }
        });
    });
}

fn nudge(window: &gtk::ApplicationWindow, nudging: &Rc<Cell<bool>>) {
    let fullscreen = window
        .window()
        .is_some_and(|gdk| gdk.state().contains(gtk::gdk::WindowState::FULLSCREEN));
    if window.is_maximized() || fullscreen {
        return;
    }
    let (width, height) = window.size();
    nudging.set(true);
    log::debug!(
        "scale factor changed to {}, nudging window of {width}x{height}",
        window.scale_factor()
    );
    window.resize(width + 1, height);
    let weak = window.downgrade();
    let nudging = nudging.clone();
    glib::timeout_add_local_once(RESTORE_DELAY, move || {
        if let Some(window) = weak.upgrade() {
            restore(&window, (width, height));
        }
        glib::timeout_add_local_once(SETTLE_DELAY, move || nudging.set(false));
    });
}

fn restore(window: &gtk::ApplicationWindow, nudged_from: (i32, i32)) {
    match restore_size(nudged_from, window.size()) {
        Some((width, height)) => window.resize(width, height),
        None => log::debug!("window resized by the app during the nudge, restore skipped"),
    }
}

fn restore_size(nudged_from: (i32, i32), current: (i32, i32)) -> Option<(i32, i32)> {
    (current.0 == nudged_from.0 + 1).then_some((nudged_from.0, current.1))
}

#[cfg(test)]
mod tests {
    use super::restore_size;

    #[test]
    fn restores_width_when_nudge_applied() {
        assert_eq!(restore_size((400, 300), (401, 300)), Some((400, 300)));
    }

    #[test]
    fn keeps_height_changed_by_app() {
        assert_eq!(restore_size((400, 300), (401, 350)), Some((400, 350)));
    }

    #[test]
    fn skips_when_width_unchanged() {
        assert_eq!(restore_size((400, 300), (400, 300)), None);
    }

    #[test]
    fn skips_when_width_changed_by_app() {
        assert_eq!(restore_size((400, 300), (405, 300)), None);
    }
}
