use tauri::Manager;

use crate::services::dialog::window;
use crate::services::dock::DockManager;

#[tauri::command]
pub async fn hide_dialog_window(app: tauri::AppHandle, label: String) -> crate::Result<()> {
    let win = window(&app, &label)?;
    win.hide()?;
    let dock = app.state::<DockManager>();
    dock.dialog_closed(&app);
    Ok(())
}
