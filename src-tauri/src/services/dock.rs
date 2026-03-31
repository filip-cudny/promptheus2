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
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        }
    }

    pub fn has_open_dialogs(&self) -> bool {
        self.dialog_count.load(Ordering::SeqCst) > 0
    }

    pub fn dialog_closed(&self, #[allow(unused)] app: &tauri::AppHandle) {
        let prev = self.dialog_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
}
