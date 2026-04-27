use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::history::HistoryEntryType;
use crate::models::menu::{MenuItem, MenuItemType};
use crate::providers::SpeechMenuProvider;
use crate::services::monitor::find_monitor_at;

#[derive(Serialize, Clone)]
struct ShowMenuPayload {
    cursor_x: f64,
    cursor_y: f64,
    work_x: f64,
    work_y: f64,
    work_width: f64,
    work_height: f64,
}

fn strip_skill_prefix<'a>(s: &'a str, skill_service: &crate::services::skill::SkillService) -> &'a str {
    if let Some(rest) = s.strip_prefix('/') {
        if let Some(space_idx) = rest.find(' ') {
            let name = &rest[..space_idx];
            if skill_service.get_skill(name).is_some() {
                return &rest[space_idx + 1..];
            }
        }
    }
    s
}

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

    let last_text = state.history.get_last_quick_action(HistoryEntryType::Text);
    let last_speech = state.history.get_last_quick_action(HistoryEntryType::Speech);

    let mut items = state.menu_coordinator.get_menu_items(&state.config);

    for item in &mut items {
        if item.item_type == MenuItemType::LastInteraction {
            item.data = Some(serde_json::json!({
                "input": last_text.as_ref().map(|e| {
                    let raw_input = strip_skill_prefix(&e.input_content, &state.skill_service);
                    serde_json::json!({ "content": raw_input, "preview": truncate(raw_input, 200) })
                }),
                "output": last_text.as_ref().and_then(|e| {
                    e.output_content.as_ref().map(|c| serde_json::json!({ "content": c, "preview": truncate(c, 200) }))
                }),
                "transcription": last_speech.as_ref().and_then(|e| {
                    e.output_content.as_ref().map(|c| serde_json::json!({ "content": c, "preview": truncate(c, 200) }))
                }),
                "last_text_entry": last_text.as_ref().map(|e| {
                    serde_json::json!({
                        "id": e.id,
                        "skill_id": e.skill_id,
                        "skill_name": e.skill_name,
                    })
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
        let display_name = if item_id == "__chat__" {
            Some("Chat".to_string())
        } else {
            let s = state.lock().await;
            s.skill_service.get_skill(&item_id).map(|sk| sk.display_name.clone())
        };

        if let Some(display_name) = display_name {
            let mut s = state.lock().await;
            if s.speech.is_recording() {
                drop(s);
                return super::speech::toggle_speech_recording(app, state, None).await;
            }
            s.speech.set_pending_prompt(Some(item_id.clone()), Some(display_name));
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
    log::debug!(target: "app_lib::commands::menu", "show_context_menu_window: ENTER");
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    log::debug!(target: "app_lib::commands::menu", "show_context_menu_window: calling cursor_position()");
    let t0 = std::time::Instant::now();
    let cursor_pos = win.cursor_position().map_err(|e| e.to_string())?;
    log::debug!(
        target: "app_lib::commands::menu",
        "show_context_menu_window: cursor_position OK in {:?} -> ({}, {})",
        t0.elapsed(), cursor_pos.x, cursor_pos.y,
    );

    let t1 = std::time::Instant::now();
    let monitor = find_monitor_at(&app, cursor_pos.x as i32, cursor_pos.y as i32)?;
    log::debug!(
        target: "app_lib::commands::menu",
        "show_context_menu_window: find_monitor_at OK in {:?}",
        t1.elapsed(),
    );

    let work = monitor.work_area();
    let scale = monitor.scale_factor();

    let payload = ShowMenuPayload {
        cursor_x: cursor_pos.x / scale,
        cursor_y: cursor_pos.y / scale,
        work_x: work.position.x as f64 / scale,
        work_y: work.position.y as f64 / scale,
        work_width: work.size.width as f64 / scale,
        work_height: work.size.height as f64 / scale,
    };

    log::debug!(
        "show context menu: cursor=({}, {}), work_area=({}, {}, {}x{})",
        payload.cursor_x, payload.cursor_y,
        payload.work_x, payload.work_y, payload.work_width, payload.work_height,
    );

    let t2 = std::time::Instant::now();
    app.emit_to("context-menu", "show-context-menu", payload)
        .map_err(|e| e.to_string())?;
    log::debug!(
        target: "app_lib::commands::menu",
        "show_context_menu_window: emit_to OK in {:?}",
        t2.elapsed(),
    );

    Ok(())
}

#[tauri::command]
pub async fn show_context_menu_panel(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let win_clone = win.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(
                crate::services::macos_panel::show_panel_without_activating(&win_clone),
            );
        })
        .map_err(|e| e.to_string())?;
        return rx.await.map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    win.show().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_context_menu_panel(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let win_clone = win.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::services::macos_panel::hide_panel(&win_clone));
        })
        .map_err(|e| e.to_string())?;
        return rx.await.map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    win.hide().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn focus_context_menu(app: tauri::AppHandle) -> Result<(), String> {
    log::debug!(target: "app_lib::commands::menu", "focus_context_menu: ENTER");
    let win = app
        .get_webview_window("context-menu")
        .ok_or("context-menu window not found")?;

    #[cfg(target_os = "linux")]
    {
        use gtk::glib::object::Cast;
        use gtk::prelude::GtkWindowExt;
        use gtk::prelude::WidgetExt;

        log::debug!(target: "app_lib::commands::menu", "focus_context_menu: calling gtk_window()");
        let t0 = std::time::Instant::now();
        if let Ok(gtk_win) = win.gtk_window() {
            log::debug!(
                target: "app_lib::commands::menu",
                "focus_context_menu: gtk_window OK in {:?}",
                t0.elapsed(),
            );
            if let Some(gdk_win) = gtk_win.window() {
                if let Ok(x11_win) = gdk_win.downcast::<gdkx11::X11Window>() {
                    log::debug!(target: "app_lib::commands::menu", "focus_context_menu: calling x11_get_server_time");
                    let t1 = std::time::Instant::now();
                    let timestamp = gdkx11::functions::x11_get_server_time(&x11_win);
                    log::debug!(
                        target: "app_lib::commands::menu",
                        "focus_context_menu: x11_get_server_time={timestamp} in {:?}",
                        t1.elapsed(),
                    );
                    let t2 = std::time::Instant::now();
                    gtk_win.present_with_time(timestamp);
                    log::debug!(
                        target: "app_lib::commands::menu",
                        "focus_context_menu: present_with_time done in {:?}",
                        t2.elapsed(),
                    );
                    return Ok(());
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let win_clone = win.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::services::macos_panel::make_key_without_activating(&win_clone));
        })
        .map_err(|e| e.to_string())?;
        return rx.await.map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    win.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_menu_providers(
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.menu_coordinator.refresh_all();
    Ok(())
}
