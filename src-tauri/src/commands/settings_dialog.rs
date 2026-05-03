use crate::services::dialog::{self, DialogConfig};
use crate::Error;

#[tauri::command]
pub async fn check_env_var(name: String) -> crate::Result<bool> {
    if name.is_empty() {
        return Ok(false);
    }
    Ok(std::env::var(&name).map(|v| !v.is_empty()).unwrap_or(false))
}

#[tauri::command]
pub async fn open_settings_window(
    app: tauri::AppHandle,
    section: Option<String>,
) -> crate::Result<()> {
    let config = DialogConfig {
        label: "settings-dialog".into(),
        url: "settings-dialog.html".into(),
        title: "Settings".into(),
        default_width: 960.0,
        default_height: 640.0,
        geometry_key: "settings-dialog".into(),
    };

    let (win, created) = dialog::open_or_focus(&app, &config)
        .await
        .map_err(Error::Other)?;

    if created {
        if let Err(e) = win.set_min_size(Some(tauri::LogicalSize::new(800.0, 560.0))) {
            log::warn!("settings-dialog: failed to set min size: {e}");
        }
    }

    if let Some(section) = section {
        let _ = win.eval(format!(
            "window.__settingsInitialSection = {};",
            serde_json::to_string(&section).unwrap_or_else(|_| "null".into())
        ));
    }

    Ok(())
}
