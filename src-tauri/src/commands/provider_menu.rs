use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

use crate::services::dialog::shell_toolbar_label_for;
use crate::Error;

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
    #[serde(default)]
    pub url: Option<String>,
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
    url: Option<String>,
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
) -> crate::Result<()> {
    log::debug!(
        target: "app_lib::commands::provider_menu",
        "show_provider_menu: host={host_label} anchor=({anchor_x}, {anchor_y}) size=({width}x{height}) active={active_id} count={}",
        providers.len(),
    );

    let win = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| Error::Other("provider-menu window not found".into()))?;

    win.set_size(LogicalSize::new(width.max(80.0), height.max(20.0)))?;
    win.set_position(LogicalPosition::new(anchor_x, anchor_y))?;

    let payload = ShowPayload {
        providers: providers
            .into_iter()
            .map(|p| ProviderEntryDto {
                id: p.id,
                name: p.name,
                url: p.url,
            })
            .collect(),
        active_id,
    };

    app.emit_to(MENU_LABEL, "provider-menu:show", payload)?;

    store_menu_host(host_label);

    win.show()?;
    win.set_focus()?;

    Ok(())
}

#[tauri::command]
pub async fn hide_provider_menu(app: AppHandle) -> crate::Result<()> {
    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        win.hide()?;
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
) -> crate::Result<()> {
    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        win.set_size(LogicalSize::new(width.max(80.0), height.max(20.0)))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn provider_menu_select(
    app: AppHandle,
    provider_id: String,
) -> crate::Result<()> {
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
    )?;
    let _ = app.emit_to(toolbar.as_str(), "provider-menu:closed", ());
    Ok(())
}
