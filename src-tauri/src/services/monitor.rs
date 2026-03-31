pub fn find_monitor_at(
    handle: &tauri::AppHandle,
    cx: i32,
    cy: i32,
) -> Result<tauri::Monitor, String> {
    let monitors = handle.available_monitors().map_err(|e| e.to_string())?;
    monitors
        .into_iter()
        .find(|m| {
            let pos = m.position();
            let size = m.size();
            cx >= pos.x
                && cx < pos.x + size.width as i32
                && cy >= pos.y
                && cy < pos.y + size.height as i32
        })
        .or_else(|| handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "no monitor found".to_string())
}
