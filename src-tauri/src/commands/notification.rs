use tauri::Manager;

#[tauri::command]
pub async fn update_notification_window(
    app: tauri::AppHandle,
    count: u32,
    height: u32,
) -> Result<(), String> {
    log::debug!("update_notification_window: count={count}, height={height}");

    let win = app
        .get_webview_window("notification")
        .ok_or_else(|| {
            log::error!("notification window not found");
            "notification window not found".to_string()
        })?;

    if count == 0 {
        log::debug!("hiding notification window (count=0)");
        win.hide().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let new_height = height.max(60);
    log::debug!("resizing notification window to 380x{new_height}");
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 380.0,
        height: new_height as f64,
    }))
    .map_err(|e| {
        log::error!("failed to set notification window size: {e}");
        e.to_string()
    })?;

    reposition_to_bottom_right(&app, &win, new_height)?;

    log::debug!("showing notification window");
    win.show().map_err(|e| {
        log::error!("failed to show notification window: {e}");
        e.to_string()
    })?;
    let _ = win.set_ignore_cursor_events(true);

    Ok(())
}

fn reposition_to_bottom_right(
    handle: &tauri::AppHandle,
    win: &tauri::WebviewWindow,
    window_height_logical: u32,
) -> Result<(), String> {
    let cursor_pos = win.cursor_position().map_err(|e| {
        log::error!("failed to get cursor position: {e}");
        e.to_string()
    })?;
    log::debug!("cursor position: ({}, {})", cursor_pos.x, cursor_pos.y);

    let monitors = handle.available_monitors().map_err(|e| {
        log::error!("failed to get monitors: {e}");
        e.to_string()
    })?;
    log::debug!("available monitors: {}", monitors.len());
    for m in &monitors {
        let pos = m.position();
        let size = m.size();
        log::debug!(
            "  monitor '{}': pos=({},{}), size={}x{}, scale={}",
            m.name().as_deref().unwrap_or(&"?".to_string()),
            pos.x, pos.y, size.width, size.height, m.scale_factor()
        );
    }

    let monitor = monitors
        .iter()
        .find(|m| {
            let pos = m.position();
            let size = m.size();
            let cx = cursor_pos.x as i32;
            let cy = cursor_pos.y as i32;
            cx >= pos.x
                && cx < pos.x + size.width as i32
                && cy >= pos.y
                && cy < pos.y + size.height as i32
        })
        .or_else(|| {
            log::warn!("cursor not on any monitor, falling back to primary");
            handle
                .primary_monitor()
                .ok()
                .flatten()
                .and_then(|m| monitors.iter().find(|mon| mon.name() == m.name()))
        })
        .ok_or_else(|| {
            log::error!("no monitor found for notification positioning");
            "no monitor found".to_string()
        })?;

    let mon_pos = monitor.position();
    let mon_size = monitor.size();
    let scale = monitor.scale_factor();

    let margin = (20.0 * scale) as i32;
    let win_width = (380.0 * scale) as i32;
    let win_height = (window_height_logical as f64 * scale) as i32;

    let x = mon_pos.x + mon_size.width as i32 - win_width - margin;
    let y = mon_pos.y + mon_size.height as i32 - win_height - margin;

    log::debug!("positioning notification window at ({x}, {y}), logical_h={window_height_logical}, physical_h={win_height}");

    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| {
            log::error!("failed to set notification window position: {e}");
            e.to_string()
        })?;

    Ok(())
}
