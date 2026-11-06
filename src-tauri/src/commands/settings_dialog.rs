use std::sync::Arc;

use crate::commands::settings::{emit_changed, rebuild_ai};
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::dialog::{self, DialogConfig};
use crate::services::secrets;
use crate::services::settings_sync::SettingsSync;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

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

    let (win, created) = dialog::open_or_focus(&app, &config).await?;

    if created {
        if let Err(e) = win.set_min_size(Some(tauri::LogicalSize::new(800.0, 560.0))) {
            log::warn!("settings-dialog: failed to set min size: {e}");
        }
    }

    if let (Some(section), Some(webview)) = (section, app.get_webview(&config.label)) {
        let _ = webview.eval(format!(
            "window.__settingsInitialSection = {};",
            serde_json::to_string(&section).unwrap_or_else(|_| "null".into())
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn save_secret(app: tauri::AppHandle, name: String, value: String) -> crate::Result<()> {
    let config_dir = app.path().app_config_dir()?;
    let host = app.state::<Arc<SettingsSync>>().host().await;
    let saved_name = name.clone();
    tokio::task::spawn_blocking(move || secrets::save_secret(&config_dir, &host, &name, &value))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    log::info!("secret '{saved_name}' saved from the settings dialog");
    {
        let config = app.state::<Arc<Mutex<ConfigService>>>();
        let ai = app.state::<Arc<Mutex<AiService>>>();
        let guard = config.lock().await;
        rebuild_ai(&guard, &mut *ai.lock().await);
    }
    emit_changed(&app)?;
    app.emit("change-log-changed", ())?;
    Ok(())
}

#[tauri::command]
pub async fn get_secret_value(app: tauri::AppHandle, name: String) -> crate::Result<Option<String>> {
    if !secrets::validate_name(&name) {
        return Ok(None);
    }
    let config_dir = app.path().app_config_dir()?;
    let value = tokio::task::spawn_blocking(move || secrets::current_value(&config_dir, &name))
        .await
        .map_err(|e| e.to_string())?;
    Ok(value)
}
