use tauri::Manager;

use crate::services::dock::DockManager;

#[tauri::command]
pub async fn open_context_editor(app: tauri::AppHandle) -> Result<(), String> {
    let label = "context-editor";

    if let Some(existing) = app.get_webview_window(label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let win = tauri::WebviewWindowBuilder::new(
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

    let dock = app.state::<DockManager>();
    dock.dialog_opened(&app);

    let app_handle = app.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            let dock = app_handle.state::<DockManager>();
            dock.dialog_closed(&app_handle);
        }
    });

    Ok(())
}
