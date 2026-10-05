pub fn find_monitor_at(
    handle: &tauri::AppHandle,
    cx: i32,
    cy: i32,
) -> Result<tauri::Monitor, String> {
    let monitors = handle.available_monitors().map_err(|e| e.to_string())?;
    let at_cursor = monitors.iter().position(|m| {
        let pos = m.position();
        let size = m.size();
        contains(pos.x, pos.y, size.width, size.height, cx, cy)
    });
    if at_cursor.is_none() {
        log::debug!(
            "no monitor contains ({cx}, {cy}); falling back to the primary, then the first of {} monitors",
            monitors.len(),
        );
    }
    let primary = || handle.primary_monitor().ok().flatten();
    select_monitor(monitors, at_cursor, primary).ok_or_else(|| "no monitor found".to_string())
}

fn contains(x: i32, y: i32, width: u32, height: u32, cx: i32, cy: i32) -> bool {
    cx >= x && cx < x + width as i32 && cy >= y && cy < y + height as i32
}

fn select_monitor<M>(
    mut monitors: Vec<M>,
    at_cursor: Option<usize>,
    primary: impl FnOnce() -> Option<M>,
) -> Option<M> {
    if let Some(index) = at_cursor {
        return Some(monitors.swap_remove(index));
    }
    primary().or_else(|| monitors.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_inside_rectangle_is_contained() {
        assert!(contains(1920, 0, 2560, 1440, 2000, 100));
        assert!(!contains(1920, 0, 2560, 1440, 1919, 100));
        assert!(!contains(0, 0, 1920, 1080, 1920, 0));
    }

    #[test]
    fn monitor_under_cursor_wins() {
        assert_eq!(select_monitor(vec!["a", "b"], Some(1), || Some("p")), Some("b"));
    }

    #[test]
    fn primary_is_used_when_cursor_matches_nothing() {
        assert_eq!(select_monitor(vec!["a", "b"], None, || Some("p")), Some("p"));
    }

    #[test]
    fn first_monitor_is_used_without_cursor_match_or_primary() {
        assert_eq!(select_monitor(vec!["a", "b"], None, || None), Some("a"));
    }

    #[test]
    fn no_monitors_and_no_primary_is_none() {
        assert_eq!(select_monitor(Vec::<&str>::new(), None, || None), None);
    }
}
