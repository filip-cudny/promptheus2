use tauri::Manager;

use crate::services::dock::DockManager;

#[tauri::command]
pub async fn hide_dialog_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    let win = app
        .get_webview_window(&label)
        .ok_or("window not found")?;
    win.hide().map_err(|e| e.to_string())?;
    let dock = app.state::<DockManager>();
    dock.dialog_closed(&app);
    Ok(())
}
