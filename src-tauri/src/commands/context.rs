use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::context::ContextItem;
use crate::services::clipboard::{paste_kind, ClipboardError, ClipboardService, PasteKind};
use crate::services::context::ContextManagerService;
use crate::services::speech::widget::RecordingWidget;

static CONTEXT_VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Clone)]
struct ContextChangedEvent {
    version: u64,
}

pub fn current_context_version() -> u64 {
    CONTEXT_VERSION.load(Ordering::Relaxed)
}

pub(crate) fn emit_context_changed(app: &AppHandle) -> crate::Result<()> {
    let version = CONTEXT_VERSION.fetch_add(1, Ordering::Relaxed) + 1;
    app.emit("context-changed", ContextChangedEvent { version })?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        app.state::<RecordingWidget>().refresh_model(&app).await;
    });
    Ok(())
}

fn remove_item_at_version(
    context: &mut ContextManagerService,
    index: usize,
    version: u64,
    current_version: u64,
) -> bool {
    version == current_version && context.remove_item(index)
}

pub async fn remove_item_for_pill(app: &AppHandle, index: usize, version: u64) -> crate::Result<()> {
    let removed = {
        let context = app.state::<Arc<Mutex<ContextManagerService>>>();
        let mut context = context.lock().await;
        remove_item_at_version(&mut context, index, version, current_context_version())
    };
    if removed {
        emit_context_changed(app)?;
    } else {
        log::debug!("pill item removal ignored: index={index} version={version} (stale or out of range)");
    }
    Ok(())
}

enum PastedItem {
    Text(String),
    Image(String, String),
}

fn read_pasted_item(clipboard: &ClipboardService) -> Result<Option<PastedItem>, ClipboardError> {
    match paste_kind(clipboard.has_file_list(), clipboard.has_image()) {
        PasteKind::Nothing => Ok(None),
        PasteKind::Image => clipboard
            .get_image_base64()
            .map(|(data, media_type)| Some(PastedItem::Image(data, media_type))),
        PasteKind::Text => clipboard.get_text().map(|text| Some(PastedItem::Text(text))),
    }
}

pub async fn paste_item_for_pill(app: &AppHandle) -> crate::Result<()> {
    let clipboard = app.state::<ClipboardService>().inner().clone();
    let read = tokio::task::spawn_blocking(move || read_pasted_item(&clipboard))
        .await
        .map_err(|e| crate::Error::Other(e.to_string()))?;
    let item = match read {
        Ok(Some(item)) => item,
        Ok(None) => {
            log::debug!("pill paste ignored: the clipboard holds files");
            return Ok(());
        }
        Err(ClipboardError::Unavailable(_)) => {
            log::debug!("pill paste ignored: nothing pasteable on the clipboard");
            return Ok(());
        }
        Err(e) => {
            log::warn!("pill paste failed reading the clipboard: {e}");
            return Ok(());
        }
    };
    {
        let context = app.state::<Arc<Mutex<ContextManagerService>>>();
        let mut context = context.lock().await;
        match item {
            PastedItem::Text(text) => context.append_context(text),
            PastedItem::Image(data, media_type) => context.append_context_image(data, media_type),
        }
    }
    emit_context_changed(app)
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
pub async fn get_labelled_context(
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
) -> crate::Result<Option<String>> {
    Ok(context.lock().await.labelled_context())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn service_with_two_texts() -> ContextManagerService {
        let mut service = ContextManagerService::new();
        service.append_context("a".into());
        service.append_context("b".into());
        service
    }

    #[test]
    fn removal_with_a_stale_version_removes_nothing() {
        let mut service = service_with_two_texts();
        assert!(!remove_item_at_version(&mut service, 0, 3, 4));
        assert_eq!(service.item_count(), 2);
    }

    #[test]
    fn removal_with_the_current_version_removes_the_item() {
        let mut service = service_with_two_texts();
        assert!(remove_item_at_version(&mut service, 0, 4, 4));
        assert_eq!(service.get_context().as_deref(), Some("b"));
    }
}
