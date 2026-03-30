use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::history::HistoryEntryType;
use crate::models::menu::{MenuItem, MenuItemType};
use crate::providers::SpeechMenuProvider;

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[tauri::command]
pub async fn get_context_menu_items(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<MenuItem>, String> {
    let mut state = state.lock().await;
    let context_items = state.context.get_items();
    state.menu_coordinator.update_context_items(context_items);

    let is_recording = state.speech.is_recording();
    let is_executing = state.prompt_execution.is_busy();
    for provider in state.menu_coordinator.providers_mut() {
        if let Some(speech) = provider.as_any_mut().downcast_mut::<SpeechMenuProvider>() {
            speech.set_recording(is_recording);
            speech.set_action_executing(is_executing);
        }
    }

    let last_text = state.history.get_last_item_by_type(HistoryEntryType::Text);
    let last_speech = state.history.get_last_item_by_type(HistoryEntryType::Speech);

    let mut items = state.menu_coordinator.get_menu_items(&state.config);

    for item in &mut items {
        if item.item_type == MenuItemType::LastInteraction {
            item.data = Some(serde_json::json!({
                "input": last_text.as_ref().map(|e| {
                    serde_json::json!({ "content": truncate(&e.input_content, 200) })
                }),
                "output": last_text.as_ref().and_then(|e| {
                    e.output_content.as_ref().map(|c| serde_json::json!({ "content": truncate(c, 200) }))
                }),
                "transcription": last_speech.as_ref().and_then(|e| {
                    e.output_content.as_ref().map(|c| serde_json::json!({ "content": truncate(c, 200) }))
                }),
            }));
        }
    }

    Ok(items)
}

#[tauri::command]
pub async fn execute_menu_item(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    item_id: String,
    shift_pressed: bool,
) -> Result<(), String> {
    log::debug!("execute_menu_item: id={item_id}, shift={shift_pressed}");

    if item_id == "system_speech_to_text" {
        return super::speech::toggle_speech_recording(app, state, None).await;
    }

    if shift_pressed {
        let is_skill = {
            let s = state.lock().await;
            s.skill_service.get_skill(&item_id).is_some()
        };

        if is_skill {
            let mut s = state.lock().await;
            if s.speech.is_recording() {
                drop(s);
                return super::speech::toggle_speech_recording(app, state, None).await;
            }
            s.speech.set_pending_prompt_id(Some(item_id.clone()));
            drop(s);
            return super::speech::toggle_speech_recording(
                app,
                state,
                Some(item_id),
            )
            .await;
        }
    }

    log::warn!("unhandled menu item: {item_id}");
    Ok(())
}

#[tauri::command]
pub async fn show_context_menu_window(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    if let Ok(pos) = win.cursor_position() {
        log::debug!("positioning context menu at ({}, {})", pos.x, pos.y);
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: pos.x as i32,
            y: pos.y as i32,
        }));
    }

    app.emit_to("context-menu", "show-context-menu", ())
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    app.show().map_err(|e| e.to_string())?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn refresh_menu_providers(
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.menu_coordinator.refresh_all();
    Ok(())
}
