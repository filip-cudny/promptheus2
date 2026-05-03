use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::context::ContextItem;

fn emit_context_changed(app: &AppHandle) -> crate::Result<()> {
    app.emit("context-changed", ())?;
    Ok(())
}

#[tauri::command]
pub async fn get_context_items(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<Vec<ContextItem>> {
    let state = state.lock().await;
    Ok(state.context.get_items())
}

#[tauri::command]
pub async fn get_context_text(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<Option<String>> {
    let state = state.lock().await;
    Ok(state.context.get_context())
}

#[tauri::command]
pub async fn has_context(state: State<'_, Mutex<AppState>>) -> crate::Result<bool> {
    let state = state.lock().await;
    Ok(state.context.has_context())
}

#[tauri::command]
pub async fn has_context_images(state: State<'_, Mutex<AppState>>) -> crate::Result<bool> {
    let state = state.lock().await;
    Ok(state.context.has_images())
}

#[tauri::command]
pub async fn set_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    value: String,
) -> crate::Result<()> {
    state.lock().await.context.set_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    value: String,
) -> crate::Result<()> {
    state.lock().await.context.append_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<()> {
    state.lock().await.context.clear();
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn remove_context_item(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    index: usize,
) -> crate::Result<bool> {
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
) -> crate::Result<()> {
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
) -> crate::Result<()> {
    state.lock().await.context.append_context_image(data, media_type);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn set_context_from_clipboard(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<()> {
    {
        let mut state = state.lock().await;
        if state.clipboard.has_image() {
            let (data, media_type) = state.clipboard.get_image_base64()?;
            state.context.set_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text()?;
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
) -> crate::Result<()> {
    {
        let mut state = state.lock().await;
        if state.clipboard.has_image() {
            let (data, media_type) = state.clipboard.get_image_base64()?;
            state.context.append_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text()?;
            state.context.append_context(text);
        }
    }
    emit_context_changed(&app)?;
    Ok(())
}
