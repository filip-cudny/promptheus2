use tauri::{Emitter, Manager};

use crate::services::dock::DockManager;

#[tauri::command]
pub async fn open_prompt_dialog(
    app: tauri::AppHandle,
    prompt_id: String,
    prompt_name: String,
    history_entry_id: Option<String>,
) -> Result<(), String> {
    let sanitized_id: String = prompt_id
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let label = format!("prompt-dialog-{sanitized_id}");

    if let Some(existing) = app.get_webview_window(&label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        if let Some(entry_id) = history_entry_id {
            app.emit_to(
                &label,
                "restore-history",
                serde_json::json!({ "entry_id": entry_id }),
            )
            .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    let mut url = format!(
        "prompt-dialog.html?promptId={}&promptName={}",
        prompt_id,
        urlencoding::encode(&prompt_name),
    );
    if let Some(entry_id) = history_entry_id {
        url.push_str(&format!("&historyEntryId={}", entry_id));
    }

    let win = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title(format!("Message to: {prompt_name}"))
    .inner_size(700.0, 600.0)
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
