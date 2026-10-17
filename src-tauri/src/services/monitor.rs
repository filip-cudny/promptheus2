use serde::Serialize;
use tauri::Manager;

#[derive(Serialize, Clone)]
pub struct CursorWorkArea {
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub work_x: f64,
    pub work_y: f64,
    pub work_width: f64,
    pub work_height: f64,
}

pub fn cursor_work_area(win: &tauri::WebviewWindow) -> crate::Result<CursorWorkArea> {
    let cursor = win.cursor_position()?;
    let monitor = find_monitor_at(win.app_handle(), cursor.x as i32, cursor.y as i32)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    Ok(CursorWorkArea {
        cursor_x: cursor.x / scale,
        cursor_y: cursor.y / scale,
        work_x: work.position.x as f64 / scale,
        work_y: work.position.y as f64 / scale,
        work_width: work.size.width as f64 / scale,
        work_height: work.size.height as f64 / scale,
    })
}

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

pub fn clamp_origin(
    anchor: (f64, f64),
    size: (f64, f64),
    work_area: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (work_x, work_y, work_width, work_height) = work_area;
    let (mut x, mut y) = anchor;
    if x + size.0 > work_x + work_width {
        x = work_x + work_width - size.0;
    }
    if y + size.1 > work_y + work_height {
        y = work_y + work_height - size.1;
    }
    (x.max(work_x), y.max(work_y))
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
    fn clamp_keeps_an_origin_that_fits() {
        assert_eq!(clamp_origin((100.0, 100.0), (240.0, 80.0), (0.0, 0.0, 1920.0, 1040.0)), (100.0, 100.0));
    }

    #[test]
    fn clamp_pulls_a_bottom_right_anchor_inside() {
        assert_eq!(clamp_origin((1900.0, 1030.0), (240.0, 80.0), (0.0, 0.0, 1920.0, 1040.0)), (1680.0, 960.0));
    }

    #[test]
    fn clamp_pushes_a_negative_anchor_to_the_work_origin() {
        assert_eq!(clamp_origin((-10.0, -10.0), (240.0, 80.0), (0.0, 0.0, 1920.0, 1040.0)), (0.0, 0.0));
    }

    #[test]
    fn clamp_respects_a_work_area_offset() {
        assert_eq!(clamp_origin((3800.0, 50.0), (240.0, 80.0), (1920.0, 0.0, 1920.0, 1080.0)), (3600.0, 50.0));
    }

    #[test]
    fn clamp_favours_the_top_left_when_larger_than_the_work_area() {
        assert_eq!(clamp_origin((10.0, 10.0), (2000.0, 1200.0), (0.0, 0.0, 1920.0, 1040.0)), (0.0, 0.0));
    }

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
