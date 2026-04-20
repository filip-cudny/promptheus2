use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::models::settings::Settings;

pub fn apply(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    match result {
        Ok(()) => {
            if enabled {
                log::info!("autostart: enabled");
            } else {
                log::info!("autostart: disabled");
            }
            Ok(())
        }
        Err(e) => {
            log::error!("autostart apply failed: {e}");
            Err(e.to_string())
        }
    }
}

pub fn reconcile(app: &AppHandle, settings: &Settings) {
    let manager = app.autolaunch();
    let os_state = match manager.is_enabled() {
        Ok(state) => state,
        Err(e) => {
            log::warn!("autostart is_enabled check failed: {e}");
            return;
        }
    };

    let desired = settings.launch_at_startup;
    if os_state == desired {
        log::debug!("autostart state in sync");
        return;
    }

    log::info!(
        "autostart state mismatch: os={os_state}, settings={desired}, applying settings value"
    );
    if let Err(e) = apply(app, desired) {
        log::warn!("autostart reconcile failed: {e}");
    }
}
