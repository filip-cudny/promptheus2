use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::context::ContextItem;
use crate::services::clipboard::ClipboardService;
use crate::services::context::ContextManagerService;

fn emit_context_changed(app: &AppHandle) -> crate::Result<()> {
    app.emit("context-changed", ())?;
    Ok(())
}

#[tauri::command]
pub async fn get_context_items(
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<Vec<ContextItem>> {
    Ok(context.lock().await.get_items())
}

#[tauri::command]
pub async fn get_context_text(
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<Option<String>> {
    Ok(context.lock().await.get_context())
}

#[tauri::command]
pub async fn has_context(
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<bool> {
    Ok(context.lock().await.has_context())
}

#[tauri::command]
pub async fn has_context_images(
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<bool> {
    Ok(context.lock().await.has_images())
}

#[tauri::command]
pub async fn set_context(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    value: String,
) -> crate::Result<()> {
    context.lock().await.set_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    value: String,
) -> crate::Result<()> {
    context.lock().await.append_context(value);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_context(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<()> {
    context.lock().await.clear();
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn remove_context_item(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    index: usize,
) -> crate::Result<bool> {
    let removed = context.lock().await.remove_item(index);
    if removed {
        emit_context_changed(&app)?;
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_context_image(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    data: String,
    media_type: String,
) -> crate::Result<()> {
    context.lock().await.set_context_image(data, media_type);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context_image(
    app: AppHandle,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    data: String,
    media_type: String,
) -> crate::Result<()> {
    context
        .lock()
        .await
        .append_context_image(data, media_type);
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn set_context_from_clipboard(
    app: AppHandle,
    clipboard: State<'_, ClipboardService>,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<()> {
    if clipboard.has_image() {
        let (data, media_type) = clipboard.get_image_base64()?;
        context.lock().await.set_context_image(data, media_type);
    } else {
        let text = clipboard.get_text()?;
        context.lock().await.set_context(text);
    }
    emit_context_changed(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn append_context_from_clipboard(
    app: AppHandle,
    clipboard: State<'_, ClipboardService>,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<()> {
    if clipboard.has_image() {
        let (data, media_type) = clipboard.get_image_base64()?;
        context
            .lock()
            .await
            .append_context_image(data, media_type);
    } else {
        let text = clipboard.get_text()?;
        context.lock().await.append_context(text);
    }
    emit_context_changed(&app)?;
    Ok(())
}
