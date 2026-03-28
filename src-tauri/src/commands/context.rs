use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::context::ContextItem;
use crate::services::notification::NotificationLevel;

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
    let notification_settings = {
        let mut state = state.lock().await;
        state.context.set_context(value);
        state.config.settings().notifications.clone()
    };
    emit_context_changed(&app)?;
    let _ = state.lock().await.notifications.notify(
        "context_set",
        NotificationLevel::Success,
        "Context set",
        None::<String>,
        &notification_settings,
    );
    Ok(())
}

#[tauri::command]
pub async fn append_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    value: String,
) -> Result<(), String> {
    let notification_settings = {
        let mut state = state.lock().await;
        state.context.append_context(value);
        state.config.settings().notifications.clone()
    };
    emit_context_changed(&app)?;
    let _ = state.lock().await.notifications.notify(
        "context_append",
        NotificationLevel::Success,
        "Context appended",
        None::<String>,
        &notification_settings,
    );
    Ok(())
}

#[tauri::command]
pub async fn clear_context(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let notification_settings = {
        let mut state = state.lock().await;
        state.context.clear();
        state.config.settings().notifications.clone()
    };
    emit_context_changed(&app)?;
    let _ = state.lock().await.notifications.notify(
        "context_cleared",
        NotificationLevel::Success,
        "Context cleared",
        None::<String>,
        &notification_settings,
    );
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
    let notification_settings = {
        let mut state = state.lock().await;
        state.context.set_context_image(data, media_type);
        state.config.settings().notifications.clone()
    };
    emit_context_changed(&app)?;
    let _ = state.lock().await.notifications.notify(
        "image_added",
        NotificationLevel::Success,
        "Context image set",
        None::<String>,
        &notification_settings,
    );
    Ok(())
}

#[tauri::command]
pub async fn append_context_image(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    data: String,
    media_type: String,
) -> Result<(), String> {
    let notification_settings = {
        let mut state = state.lock().await;
        state.context.append_context_image(data, media_type);
        state.config.settings().notifications.clone()
    };
    emit_context_changed(&app)?;
    let _ = state.lock().await.notifications.notify(
        "image_added",
        NotificationLevel::Success,
        "Context image appended",
        None::<String>,
        &notification_settings,
    );
    Ok(())
}

#[tauri::command]
pub async fn set_context_from_clipboard(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let (is_image, notification_settings) = {
        let mut state = state.lock().await;
        let is_image = state.clipboard.has_image();
        if is_image {
            let (data, media_type) = state
                .clipboard
                .get_image_base64()
                .map_err(|e| e.to_string())?;
            state.context.set_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text().map_err(|e| e.to_string())?;
            state.context.set_context(text);
        }
        (is_image, state.config.settings().notifications.clone())
    };
    emit_context_changed(&app)?;
    let (event_name, title) = if is_image {
        ("context_set", "Context image set")
    } else {
        ("context_set", "Context set")
    };
    let _ = state.lock().await.notifications.notify(
        event_name,
        NotificationLevel::Success,
        title,
        None::<String>,
        &notification_settings,
    );
    Ok(())
}

#[tauri::command]
pub async fn append_context_from_clipboard(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let (is_image, notification_settings) = {
        let mut state = state.lock().await;
        let is_image = state.clipboard.has_image();
        if is_image {
            let (data, media_type) = state
                .clipboard
                .get_image_base64()
                .map_err(|e| e.to_string())?;
            state.context.append_context_image(data, media_type);
        } else {
            let text = state.clipboard.get_text().map_err(|e| e.to_string())?;
            state.context.append_context(text);
        }
        (is_image, state.config.settings().notifications.clone())
    };
    emit_context_changed(&app)?;
    let (event_name, title) = if is_image {
        ("context_append", "Context image appended")
    } else {
        ("context_append", "Context appended")
    };
    let _ = state.lock().await.notifications.notify(
        event_name,
        NotificationLevel::Success,
        title,
        None::<String>,
        &notification_settings,
    );
    Ok(())
}
