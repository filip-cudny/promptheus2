use crate::services::dialog::{self, DialogConfig};

#[tauri::command]
pub async fn open_history_dialog(app: tauri::AppHandle) -> crate::Result<()> {
    let config = DialogConfig {
        label: "history-dialog".into(),
        url: "history-dialog.html".into(),
        title: "History".into(),
        default_width: 600.0,
        default_height: 500.0,
        geometry_key: "history-dialog".into(),
        added_width: 0.0,
    };

    dialog::open_or_focus(&app, &config).await?;
    Ok(())
}
