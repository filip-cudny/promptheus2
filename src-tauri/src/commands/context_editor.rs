use crate::services::dialog::{self, DialogConfig};

#[tauri::command]
pub async fn open_context_editor(app: tauri::AppHandle) -> Result<(), String> {
    let config = DialogConfig {
        label: "context-editor",
        url: "context-editor.html".into(),
        title: "Edit Context",
        default_width: 500.0,
        default_height: 400.0,
        geometry_key: "context-editor",
    };

    dialog::open_or_focus(&app, &config).await?;
    Ok(())
}
