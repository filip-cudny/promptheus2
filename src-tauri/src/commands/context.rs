use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::context::ContextItem;

fn emit_context_changed(app: &AppHandle) -> Result<(), String> {
    app.emit("context-changed", ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_context_items(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<ContextItem>, String> {
    let state = state.lock().await;
    Ok(state.context.get_items())
}

#[tauri::command]
pub async fn get_context_text(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    let state = state.lock().await;
    Ok(state.context.get_context())
}

#[tauri::command]
pub async fn has_context(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.lock().await;
    Ok(state.context.has_context())
}

#[tauri::command]
pub async fn has_context_images(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.lock().await;
    Ok(state.context.has_images())
}

#[tauri::command]
pub async fn set_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    value: String,
) -> Result<(), String> {
    state.lock().await.context.set_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    value: String,
) -> Result<(), String> {
    state.lock().await.context.append_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().await.context.clear();
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn remove_context_item(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    index: usize,
) -> Result<bool, String> {
    let removed = {
        let mut state = state.lock().await;
        state.context.remove_item(index)
    };
    if removed {
        emit_context_changed(&app)?;
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_context_image(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    data: String,
    media_type: String,
) -> Result<(), String> {
    state.lock().await.context.set_context_image(data, media_type);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context_image(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    data: String,
    media_type: String,
) -> Result<(), String> {
    state.lock().await.context.append_context_image(data, media_type);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn set_context_from_clipboard(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    {
        let mut state = state.lock().await;
        if state.clipboard.has_image() {
            let (data, media_type) = state
                .clipboard
                .get_image_base64()
                .map_err(|e| e.to_string())?;
            state.context.set_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text().map_err(|e| e.to_string())?;
            state.context.set_context(text);
        }
    }
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context_from_clipboard(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    {
        let mut state = state.lock().await;
        if state.clipboard.has_image() {
            let (data, media_type) = state
                .clipboard
                .get_image_base64()
                .map_err(|e| e.to_string())?;
            state.context.append_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text().map_err(|e| e.to_string())?;
            state.context.append_context(text);
        }
    }
    emit_context_changed(&app)?;
    Ok(())
}
