use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Default)]
struct HostCounting {
    hidden: bool,
}

impl HostCounting {
    fn on_hide(&mut self) -> bool {
        !std::mem::replace(&mut self.hidden, true)
    }

    fn on_surface(&mut self) -> bool {
        std::mem::replace(&mut self.hidden, false)
    }

    fn on_destroy(&mut self) -> bool {
        !std::mem::take(&mut self.hidden)
    }
}

pub struct DockManager {
    dialog_count: AtomicUsize,
    host: Mutex<HostCounting>,
}

impl DockManager {
    pub fn new() -> Self {
        Self {
            dialog_count: AtomicUsize::new(0),
            host: Mutex::new(HostCounting::default()),
        }
    }

    pub fn dialog_opened(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self.dialog_count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            #[cfg(target_os = "macos")]
            apply_dock_visible(app, true);
        }
    }

    pub fn dialog_closed(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self
            .dialog_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
        if prev == Ok(1) {
            #[cfg(target_os = "macos")]
            apply_dock_visible(app, false);
        }
    }

    pub fn host_hidden(&self, app: &tauri::AppHandle) {
        if self.host.lock().unwrap().on_hide() {
            self.dialog_closed(app);
        }
    }

    pub fn host_surfaced(&self, app: &tauri::AppHandle) {
        if self.host.lock().unwrap().on_surface() {
            self.dialog_opened(app);
        }
    }

    pub fn host_destroyed(&self, app: &tauri::AppHandle) {
        if self.host.lock().unwrap().on_destroy() {
            self.dialog_closed(app);
        }
    }
}

#[cfg(target_os = "macos")]
fn apply_dock_visible(app: &tauri::AppHandle, visible: bool) {
    let (policy, policy_label) = if visible {
        (tauri::ActivationPolicy::Regular, "Regular")
    } else {
        (tauri::ActivationPolicy::Accessory, "Accessory")
    };
    if let Err(e) = app.set_activation_policy(policy) {
        log::warn!(
            target: "app_lib::services::dock",
            "set_activation_policy({policy_label}) failed: {e}",
        );
    }
    if let Err(e) = app.set_dock_visibility(visible) {
        log::warn!(
            target: "app_lib::services::dock",
            "set_dock_visibility({visible}) failed: {e}",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hide_decrements_once() {
        let mut host = HostCounting::default();
        assert!(host.on_hide());
        assert!(!host.on_hide());
    }

    #[test]
    fn surface_increments_once_and_only_from_hidden() {
        let mut host = HostCounting::default();
        assert!(!host.on_surface());
        host.on_hide();
        assert!(host.on_surface());
        assert!(!host.on_surface());
    }

    #[test]
    fn destroy_after_hide_does_not_decrement() {
        let mut host = HostCounting::default();
        host.on_hide();
        assert!(!host.on_destroy());
    }

    #[test]
    fn destroy_while_visible_decrements() {
        let mut host = HostCounting::default();
        assert!(host.on_destroy());
    }
}
