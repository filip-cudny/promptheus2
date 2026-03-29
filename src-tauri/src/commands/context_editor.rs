use tauri::Manager;

#[tauri::command]
pub async fn open_context_editor(app: tauri::AppHandle) -> Result<(), String> {
    let label = "context-editor";

    if let Some(existing) = app.get_webview_window(label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App("context-editor.html".into()),
    )
    .title("Edit Context")
    .inner_size(500.0, 400.0)
    .resizable(true)
    .decorations(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}
