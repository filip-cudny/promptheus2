use std::sync::Arc;

use crate::commands::settings::{emit_changed, rebuild_ai};
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::dialog::{self, DialogConfig, SETTINGS_DIALOG_LABEL};
use crate::services::settings_agent::panel;
use crate::services::secrets;
use crate::services::settings_sync::SettingsSync;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

const DEFAULT_WIDTH: f64 = 960.0;

#[tauri::command]
pub async fn open_settings_window(
    app: tauri::AppHandle,
    section: Option<String>,
) -> crate::Result<()> {
    let initial_panel = if app.get_webview_window(SETTINGS_DIALOG_LABEL).is_none() {
        Some(panel::initial_for_new_window(&app, SETTINGS_DIALOG_LABEL, DEFAULT_WIDTH).await)
    } else {
        None
    };
    let config = DialogConfig {
        label: SETTINGS_DIALOG_LABEL.into(),
        url: "settings-dialog.html".into(),
        title: "Settings".into(),
        default_width: DEFAULT_WIDTH,
        default_height: 640.0,
        geometry_key: "settings-dialog".into(),
        added_width: match initial_panel {
            Some(panel::InitialPanel::Widened(width)) => width,
            _ => 0.0,
        },
    };

    let (win, created) = dialog::open_or_focus(&app, &config).await?;

    if created {
        if let Err(e) = win.set_min_size(Some(tauri::LogicalSize::new(800.0, 560.0))) {
            log::warn!("settings-dialog: failed to set min size: {e}");
        }
    }

    if let (true, Some(initial_panel)) = (created, &initial_panel) {
        if let panel::InitialPanel::Widened(width) = initial_panel {
            panel::adopt_widened(&app, *width);
        }
        if let Some(webview) = app.get_webview(&config.label) {
            let mode = initial_panel.mode();
            if let Err(e) = webview.eval(format!(
                "window.__settingsAgentPanel = \"{mode}\"; window.__onSettingsAgentPanel?.(\"{mode}\");"
            )) {
                log::warn!("settings-dialog: failed to pass the agent panel mode to the page: {e}");
            }
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
