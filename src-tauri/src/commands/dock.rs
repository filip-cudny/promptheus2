use tauri::Manager;

use crate::services::dock::DockManager;
use crate::Error;

#[tauri::command]
pub async fn hide_dialog_window(app: tauri::AppHandle, label: String) -> crate::Result<()> {
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| Error::Other("window not found".into()))?;
    win.hide()?;
    let dock = app.state::<DockManager>();
    dock.dialog_closed(&app);
    Ok(())
}
