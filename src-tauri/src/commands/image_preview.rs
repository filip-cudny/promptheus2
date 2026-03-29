use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn open_image_preview(
    app: tauri::AppHandle,
    data: String,
    media_type: String,
) -> Result<(), String> {
    let win = app
        .get_webview_window("image-preview")
        .ok_or("image-preview window not found")?;

    if let Ok(pos) = win.cursor_position() {
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: pos.x as i32,
            y: pos.y as i32,
        }));
    }

    let payload = serde_json::json!({ "data": data, "media_type": media_type });
    app.emit_to("image-preview", "image-data", payload)
        .map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}
