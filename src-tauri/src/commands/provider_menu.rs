use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

use crate::services::dialog::shell_toolbar_label_for;

const MENU_LABEL: &str = "provider-menu";

static MENU_HOST: Mutex<Option<String>> = Mutex::new(None);

fn store_menu_host(host_label: String) {
    *MENU_HOST.lock().unwrap_or_else(|e| e.into_inner()) = Some(host_label);
}

fn take_menu_host() -> Option<String> {
    MENU_HOST.lock().unwrap_or_else(|e| e.into_inner()).take()
}

#[derive(Deserialize)]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
struct ShowPayload {
    providers: Vec<ProviderEntryDto>,
    active_id: String,
}

#[derive(Serialize, Clone)]
struct ProviderEntryDto {
    id: String,
    name: String,
}

#[derive(Serialize, Clone)]
struct SelectPayload {
    provider_id: String,
}

#[tauri::command]
pub async fn show_provider_menu(
    app: AppHandle,
    host_label: String,
    anchor_x: f64,
    anchor_y: f64,
    width: f64,
    height: f64,
    providers: Vec<ProviderEntry>,
    active_id: String,
) -> Result<(), String> {
    log::debug!(
        target: "app_lib::commands::provider_menu",
        "show_provider_menu: host={host_label} anchor=({anchor_x}, {anchor_y}) size=({width}x{height}) active={active_id} count={}",
        providers.len(),
    );

    let win = app
        .get_webview_window(MENU_LABEL)
        .ok_or("provider-menu window not found")?;

    win.set_size(LogicalSize::new(width.max(80.0), height.max(20.0)))
        .map_err(|e| e.to_string())?;
    win.set_position(LogicalPosition::new(anchor_x, anchor_y))
        .map_err(|e| e.to_string())?;

    let payload = ShowPayload {
        providers: providers
            .into_iter()
            .map(|p| ProviderEntryDto { id: p.id, name: p.name })
            .collect(),
        active_id,
    };

    app.emit_to(MENU_LABEL, "provider-menu:show", payload)
        .map_err(|e| e.to_string())?;

    store_menu_host(host_label);

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn hide_provider_menu(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        win.hide().map_err(|e| e.to_string())?;
    }
    if let Some(host) = take_menu_host() {
        let toolbar = shell_toolbar_label_for(&host);
        let _ = app.emit_to(toolbar.as_str(), "provider-menu:closed", ());
    }
    Ok(())
}

#[tauri::command]
pub async fn size_provider_menu(
    app: AppHandle,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        win.set_size(LogicalSize::new(width.max(80.0), height.max(20.0)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn provider_menu_select(
    app: AppHandle,
    provider_id: String,
) -> Result<(), String> {
    log::debug!(
        target: "app_lib::commands::provider_menu",
        "provider_menu_select: {provider_id}",
    );

    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        let _ = win.hide();
    }

    let Some(host) = take_menu_host() else {
        log::warn!(
            target: "app_lib::commands::provider_menu",
            "provider_menu_select: no recorded host for selection {provider_id}",
        );
        return Ok(());
    };
    let toolbar = shell_toolbar_label_for(&host);

    app.emit_to(
        toolbar.as_str(),
        "provider-menu:select",
        SelectPayload { provider_id },
    )
    .map_err(|e| e.to_string())?;
    let _ = app.emit_to(toolbar.as_str(), "provider-menu:closed", ());
    Ok(())
}
