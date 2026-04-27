use std::sync::atomic::{AtomicUsize, Ordering};

pub struct DockManager {
    dialog_count: AtomicUsize,
}

impl DockManager {
    pub fn new() -> Self {
        Self {
            dialog_count: AtomicUsize::new(0),
        }
    }

    pub fn dialog_opened(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self.dialog_count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            #[cfg(target_os = "macos")]
            apply_dock_visible(app, true);
        }
    }

    pub fn has_open_dialogs(&self) -> bool {
        self.dialog_count.load(Ordering::SeqCst) > 0
    }

    pub fn dialog_closed(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self.dialog_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            #[cfg(target_os = "macos")]
            apply_dock_visible(app, false);
        }
    }

    pub fn rollback_open(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self.dialog_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            #[cfg(target_os = "macos")]
            apply_dock_visible(app, false);
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
