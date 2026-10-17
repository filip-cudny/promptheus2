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

#[derive(Debug, PartialEq)]
enum AutostartAction {
    Enable,
    Disable,
    None,
}

fn reconcile_action(desired: bool, os_state: bool, rewrite_enabled: bool) -> AutostartAction {
    if rewrite_enabled && desired {
        AutostartAction::Enable
    } else if desired != os_state {
        if desired {
            AutostartAction::Enable
        } else {
            AutostartAction::Disable
        }
    } else {
        AutostartAction::None
    }
}

pub fn reconcile(app: &AppHandle, settings: &Settings, rewrite_enabled: bool) {
    let manager = app.autolaunch();
    let os_state = match manager.is_enabled() {
        Ok(state) => state,
        Err(e) => {
            log::warn!("autostart is_enabled check failed: {e}");
            return;
        }
    };

    let desired = settings.launch_at_startup;
    let action = reconcile_action(desired, os_state, rewrite_enabled);
    if action == AutostartAction::None {
        log::debug!("autostart state in sync");
        return;
    }

    if os_state == desired {
        log::info!("autostart entry rewritten for the current binary");
    } else {
        log::info!(
            "autostart state mismatch: os={os_state}, settings={desired}, applying settings value"
        );
    }
    if let Err(e) = apply(app, action == AutostartAction::Enable) {
        log::warn!("autostart reconcile failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_startup_rewrites_enabled_entry() {
        assert_eq!(reconcile_action(true, true, true), AutostartAction::Enable);
    }

    #[test]
    fn debug_in_sync_does_nothing() {
        assert_eq!(reconcile_action(true, true, false), AutostartAction::None);
    }

    #[test]
    fn disabled_in_sync_does_nothing_even_with_rewrite() {
        assert_eq!(reconcile_action(false, false, true), AutostartAction::None);
    }

    #[test]
    fn disabled_setting_disables_enabled_entry() {
        assert_eq!(
            reconcile_action(false, true, true),
            AutostartAction::Disable
        );
    }

    #[test]
    fn mismatch_enables_missing_entry() {
        assert_eq!(
            reconcile_action(true, false, false),
            AutostartAction::Enable
        );
    }
}
