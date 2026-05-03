use crate::services::dialog::{self, DialogConfig};
use crate::Error;

#[tauri::command]
pub async fn open_history_dialog(app: tauri::AppHandle) -> crate::Result<()> {
    let config = DialogConfig {
        label: "history-dialog".into(),
        url: "history-dialog.html".into(),
        title: "History".into(),
        default_width: 600.0,
        default_height: 500.0,
        geometry_key: "history-dialog".into(),
    };

    dialog::open_or_focus(&app, &config)
        .await
        .map_err(Error::Other)?;
    Ok(())
}
