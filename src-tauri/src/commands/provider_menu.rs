use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

use crate::services::dialog::SHELL_TOOLBAR_LABEL;
use crate::Error;

const MENU_LABEL: &str = "provider-menu";

pub const PROVIDER_MENU_TITLE: &str = "Promptheus Provider Menu";

static MENU_HOST: Mutex<Option<String>> = Mutex::new(None);

fn store_menu_host(host_label: String) {
    *MENU_HOST.lock().unwrap_or_else(|e| e.into_inner()) = Some(host_label);
}

fn take_menu_host() -> Option<String> {
    MENU_HOST.lock().unwrap_or_else(|e| e.into_inner()).take()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MenuItem {
    pub id: String,
    pub kind: String,
    pub icon: String,
    pub title: String,
    pub active: bool,
}

#[derive(Serialize, Clone)]
struct ShowPayload {
    items: Vec<MenuItem>,
}

#[derive(Serialize, Clone)]
struct SelectPayload {
    item_id: String,
}

#[derive(Serialize, Clone)]
struct ProviderMenuClosedEvent {
    via_select: bool,
}

#[tauri::command]
pub async fn show_provider_menu(
    app: AppHandle,
    host_label: String,
    anchor_x: f64,
    anchor_y: f64,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
    items: Vec<MenuItem>,
) -> crate::Result<()> {
    log::debug!(
        target: "app_lib::commands::provider_menu",
        "show_provider_menu: host={host_label} anchor=({anchor_x}, {anchor_y}) offset=({offset_x}, {offset_y}) size=({width}x{height}) count={}",
        items.len(),
    );

    let win = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| Error::Other("provider-menu window not found".into()))?;

    win.set_size(LogicalSize::new(width.max(80.0), height.max(20.0)))?;

    let payload = ShowPayload { items };

    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        let host = app
            .get_window(&host_label)
            .ok_or_else(|| Error::Other(format!("host window {host_label} not found")))?;
        app.emit_to(MENU_LABEL, "provider-menu:show", payload)?;
        store_menu_host(host_label);
        if let Err(e) = crate::services::gnome_shell::set_transient_parent(&win, &host).await {
            log::warn!(
                target: "app_lib::commands::provider_menu",
                "set_transient_parent failed: {e}"
            );
        }
        if let Err(e) = crate::services::gnome_shell::place_window_on_parent(
            PROVIDER_MENU_TITLE,
            offset_x.round() as i32,
            offset_y.round() as i32,
            true,
            Some(&win),
        )
        .await
        {
            log::warn!(
                target: "app_lib::commands::provider_menu",
                "place_window_on_parent failed: {e}"
            );
            win.show()?;
        }
        return Ok(());
    }

    win.set_position(LogicalPosition::new(anchor_x, anchor_y))?;

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
    if take_menu_host().is_some() {
        let _ = app.emit_to(
            SHELL_TOOLBAR_LABEL,
            "provider-menu:closed",
            ProviderMenuClosedEvent { via_select: false },
        );
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
    item_id: String,
) -> crate::Result<()> {
    log::debug!(
        target: "app_lib::commands::provider_menu",
        "provider_menu_select: {item_id}",
    );

    if let Some(win) = app.get_webview_window(MENU_LABEL) {
        let _ = win.hide();
    }

    if take_menu_host().is_none() {
        log::warn!(
            target: "app_lib::commands::provider_menu",
            "provider_menu_select: no recorded host for selection {item_id}",
        );
        return Ok(());
    }

    app.emit_to(
        SHELL_TOOLBAR_LABEL,
        "provider-menu:select",
        SelectPayload { item_id },
    )?;
    let _ = app.emit_to(
        SHELL_TOOLBAR_LABEL,
        "provider-menu:closed",
        ProviderMenuClosedEvent { via_select: true },
    );
    Ok(())
}
