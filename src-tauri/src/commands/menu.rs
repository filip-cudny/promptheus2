use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::history::HistoryEntryType;
use crate::models::menu::{MenuItem, MenuItemType};
use crate::providers::SpeechMenuProvider;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::execution::PromptExecutionService;
use crate::services::menu_coordinator::MenuCoordinator;
use crate::services::monitor::find_monitor_at;
use crate::services::skill::{strip_skill_prefix, SkillService};
use crate::services::speech::{AudioClipStore, SpeechService};
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

pub const CONTEXT_MENU_TITLE: &str = "Promptheus Context Menu";

#[derive(Serialize, Clone)]
struct ShowMenuPayload {
    cursor_x: f64,
    cursor_y: f64,
    work_x: f64,
    work_y: f64,
    work_width: f64,
    work_height: f64,
    shell_placement: bool,
}

#[cfg(target_os = "linux")]
fn payload_from_pointer(pointer: (i32, i32, i32, i32, i32, i32)) -> ShowMenuPayload {
    let (cursor_x, cursor_y, work_x, work_y, work_width, work_height) = pointer;
    ShowMenuPayload {
        cursor_x: cursor_x as f64,
        cursor_y: cursor_y as f64,
        work_x: work_x as f64,
        work_y: work_y as f64,
        work_width: work_width as f64,
        work_height: work_height as f64,
        shell_placement: true,
    }
}

#[cfg(target_os = "linux")]
async fn shell_pointer_payload() -> Option<ShowMenuPayload> {
    let t = std::time::Instant::now();
    let result = async {
        crate::services::gnome_shell::proxy()
            .await?
            .get_pointer()
            .await
    }
    .await;
    match result {
        Ok(pointer) => {
            log::debug!(
                target: "app_lib::commands::menu",
                "show_context_menu_window: GetPointer OK in {:?}",
                t.elapsed(),
            );
            Some(payload_from_pointer(pointer))
        }
        Err(e) => {
            log::warn!(
                target: "app_lib::commands::menu",
                "show_context_menu_window: GetPointer failed, falling back to cursor_position: {e}",
            );
            None
        }
    }
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

/// Chip payloads carry previews only. The clipboard value is resolved on the
/// Rust side by `copy_last_interaction`, so a full response never crosses IPC
/// just to render the menu.
#[tauri::command]
pub async fn get_context_menu_items(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    context: State<'_, Arc<Mutex<ContextManagerService>>>,
    menu_coordinator: State<'_, Arc<Mutex<MenuCoordinator>>>,
    speech: State<'_, Arc<Mutex<SpeechService>>>,
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    clips: State<'_, Arc<AudioClipStore>>,
) -> crate::Result<Vec<MenuItem>> {
    let context_items = context.lock().await.get_items();
    let mut menu_coordinator = menu_coordinator.lock().await;
    menu_coordinator.update_context_items(context_items);

    let (is_recording, retrying_entries) = {
        let s = speech.lock().await;
        (s.is_recording(), s.retrying_entries())
    };
    let is_executing = prompt_execution.lock().await.is_busy();
    for provider in menu_coordinator.providers_mut() {
        if let Some(speech) = provider.as_any_mut().downcast_mut::<SpeechMenuProvider>() {
            speech.set_recording(is_recording);
            speech.set_action_executing(is_executing);
        }
    }

    let (last_text, last_speech, last_speech_clip) = {
        let history = history.lock().await;
        let last_text = history.get_last_quick_action(HistoryEntryType::Text);
        let last_speech = history.get_last_quick_action(HistoryEntryType::Speech);
        let clip = last_speech
            .as_ref()
            .and_then(|e| clips.find_by_entry(history.conn(), &e.id))
            .filter(|clip| clip.path.exists());
        (last_text, last_speech, clip)
    };

    let transcription_payload = last_speech.as_ref().map(|entry| {
        let status = if !entry.success {
            if retrying_entries.contains(&entry.id) {
                "retrying"
            } else {
                "failed"
            }
        } else {
            "ok"
        };
        serde_json::json!({
            "status": status,
            "entry_id": entry.id,
            "preview": entry.output_preview.as_deref().map(|c| truncate(c, 200)),
            "error": entry.error,
            "has_audio": last_speech_clip.is_some(),
            "expires_at": last_speech_clip.as_ref().map(|c| c.expires_at.clone()),
        })
    });

    let config = config.lock().await;
    let skill_service = skill_service.lock().await;

    let mut items = menu_coordinator.get_menu_items(&config);

    for item in &mut items {
        if item.item_type == MenuItemType::LastInteraction {
            item.data = Some(serde_json::json!({
                "input": last_text.as_ref().map(|e| {
                    let raw_input = strip_skill_prefix(&e.input_preview, &skill_service);
                    serde_json::json!({ "preview": truncate(raw_input, 200) })
                }),
                "output": last_text.as_ref().and_then(|e| {
                    e.output_preview.as_ref().map(|c| serde_json::json!({ "preview": truncate(c, 200) }))
                }),
                "transcription": transcription_payload.clone(),
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
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    speech: State<'_, Arc<Mutex<SpeechService>>>,
    item_id: String,
    shift_pressed: bool,
) -> crate::Result<()> {
    log::debug!("execute_menu_item: id={item_id}, shift={shift_pressed}");

    if item_id == "system_speech_to_text" {
        return super::speech::toggle_speech_recording(app, None).await;
    }

    if shift_pressed {
        let display_name = if item_id == "__chat__" {
            Some("Chat".to_string())
        } else {
            skill_service
                .lock()
                .await
                .get_skill(&item_id)
                .map(|sk| sk.display_name.clone())
        };

        if let Some(display_name) = display_name {
            let mut s = speech.lock().await;
            if s.is_recording() {
                drop(s);
                return super::speech::toggle_speech_recording(app, None).await;
            }
            s.set_pending_prompt(Some(item_id.clone()), Some(display_name));
            drop(s);
            return super::speech::toggle_speech_recording(app, Some(item_id)).await;
        }
    }

    log::warn!("unhandled menu item: {item_id}");
    Ok(())
}

#[tauri::command]
pub async fn show_context_menu_window(app: tauri::AppHandle) -> crate::Result<()> {
    log::debug!(target: "app_lib::commands::menu", "show_context_menu_window: ENTER");
    let win = app
        .get_webview_window("context-menu")
        .ok_or_else(|| Error::Other("context-menu window not found".into()))?;

    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        if let Some(payload) = shell_pointer_payload().await {
            app.emit_to("context-menu", "show-context-menu", payload)?;
            return Ok(());
        }
    }

    log::debug!(target: "app_lib::commands::menu", "show_context_menu_window: calling cursor_position()");
    let t0 = std::time::Instant::now();
    let cursor_pos = win.cursor_position()?;
    log::debug!(
        target: "app_lib::commands::menu",
        "show_context_menu_window: cursor_position OK in {:?} -> ({}, {})",
        t0.elapsed(), cursor_pos.x, cursor_pos.y,
    );

    let t1 = std::time::Instant::now();
    let monitor = find_monitor_at(&app, cursor_pos.x as i32, cursor_pos.y as i32)
        .map_err(Error::Other)?;
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
        shell_placement: false,
    };

    log::debug!(
        "show context menu: cursor=({}, {}), work_area=({}, {}, {}x{})",
        payload.cursor_x, payload.cursor_y,
        payload.work_x, payload.work_y, payload.work_width, payload.work_height,
    );

    let t2 = std::time::Instant::now();
    app.emit_to("context-menu", "show-context-menu", payload)?;
    log::debug!(
        target: "app_lib::commands::menu",
        "show_context_menu_window: emit_to OK in {:?}",
        t2.elapsed(),
    );

    Ok(())
}

#[tauri::command]
pub async fn show_context_menu_panel(app: tauri::AppHandle) -> crate::Result<()> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or_else(|| Error::Other("context-menu window not found".into()))?;

    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let win_clone = win.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(
                crate::services::macos_panel::show_panel_without_activating(&win_clone),
            );
        })?;
        rx.await
            .map_err(|e| Error::Other(e.to_string()))?
            .map_err(Error::Other)
    }

    #[cfg(not(target_os = "macos"))]
    {
        win.show()?;
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::WidgetExt;
            if let Ok(gtk_win) = win.gtk_window() {
                gtk_win.set_opacity(0.99);
            } else {
                log::warn!(
                    target: "app_lib::commands::menu",
                    "show_context_menu_panel: gtk_window() failed; rounded corners may render as a gray square",
                );
            }
        }
        Ok(())
    }
}

#[tauri::command]
pub async fn place_context_menu(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    first: bool,
) -> crate::Result<()> {
    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        return place_context_menu_through_shell(&app, x.round() as i32, y.round() as i32, first)
            .await;
    }
    let _ = (app, x, y, first);
    Ok(())
}

#[cfg(target_os = "linux")]
async fn place_context_menu_through_shell(
    app: &tauri::AppHandle,
    x: i32,
    y: i32,
    first: bool,
) -> crate::Result<()> {
    use std::future::{poll_fn, Future};
    use std::task::Poll;

    let win = app
        .get_webview_window("context-menu")
        .ok_or_else(|| Error::Other("context-menu window not found".into()))?;
    let proxy = crate::services::gnome_shell::proxy()
        .await
        .map_err(|e| Error::Other(e.to_string()))?;

    let t = std::time::Instant::now();
    let mut call = std::pin::pin!(proxy.place_window(CONTEXT_MENU_TITLE, x, y, first));
    if first {
        poll_fn(|cx| {
            let _ = call.as_mut().poll(cx);
            Poll::Ready(())
        })
        .await;
        win.show()?;
    }
    let placed = call.await.map_err(|e| Error::Other(e.to_string()))?;
    log::debug!(
        target: "app_lib::commands::menu",
        "place_context_menu: PlaceWindow(({x}, {y}), activate={first}) -> {placed} in {:?}",
        t.elapsed(),
    );
    if !placed {
        log::warn!(
            target: "app_lib::commands::menu",
            "place_context_menu: extension found no window titled {CONTEXT_MENU_TITLE:?}",
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_context_menu_panel(app: tauri::AppHandle) -> crate::Result<()> {
    let win = app
        .get_webview_window("context-menu")
        .ok_or_else(|| Error::Other("context-menu window not found".into()))?;

    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let win_clone = win.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::services::macos_panel::hide_panel(&win_clone));
        })?;
        rx.await
            .map_err(|e| Error::Other(e.to_string()))?
            .map_err(Error::Other)
    }

    #[cfg(not(target_os = "macos"))]
    {
        win.hide()?;
        Ok(())
    }
}

#[tauri::command]
pub async fn focus_context_menu(app: tauri::AppHandle) -> crate::Result<()> {
    log::debug!(target: "app_lib::commands::menu", "focus_context_menu: ENTER");
    let win = app
        .get_webview_window("context-menu")
        .ok_or_else(|| Error::Other("context-menu window not found".into()))?;

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
        })?;
        rx.await
            .map_err(|e| Error::Other(e.to_string()))?
            .map_err(Error::Other)
    }

    #[cfg(not(target_os = "macos"))]
    {
        win.set_focus()?;
        Ok(())
    }
}

#[tauri::command]
pub async fn refresh_menu_providers(
    menu_coordinator: State<'_, Arc<Mutex<MenuCoordinator>>>,
) -> crate::Result<()> {
    menu_coordinator.lock().await.refresh_all();
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn pointer_tuple_maps_to_shell_placement_payload() {
        let payload = payload_from_pointer((100, 200, 0, 32, 1920, 1048));
        assert_eq!((payload.cursor_x, payload.cursor_y), (100.0, 200.0));
        assert_eq!(
            (payload.work_x, payload.work_y, payload.work_width, payload.work_height),
            (0.0, 32.0, 1920.0, 1048.0),
        );
        assert!(payload.shell_placement);
    }
}
