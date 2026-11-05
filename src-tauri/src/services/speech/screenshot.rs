use std::path::Path;

pub const DIR_NAME: &str = "pill-screenshots";

pub fn keep_capture(click_generation: u64, current_generation: Option<u64>, target_locked: bool) -> bool {
    current_generation == Some(click_generation) && !target_locked
}

/// Returns the file content when `read` is set; the file is deleted either way.
pub fn read_and_delete(path: &Path, read: bool) -> Option<Vec<u8>> {
    let bytes = if read {
        std::fs::read(path)
            .map_err(|e| log::warn!("reading the screenshot failed: {e}"))
            .ok()
    } else {
        None
    };
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("deleting the screenshot failed: {e}"),
    }
    bytes
}

#[cfg(target_os = "linux")]
pub async fn capture_gnome(path: &str) -> bool {
    let reply = async { crate::services::gnome_shell::proxy().await?.capture_area(path).await }.await;
    match reply {
        Ok(outcome) if outcome == "captured" => true,
        Ok(outcome) => {
            log::debug!("screenshot not captured: {outcome}");
            false
        }
        Err(e) => {
            log::warn!("CaptureArea failed: {e}");
            false
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::Path;

    use tauri::{AppHandle, Manager};

    use crate::services::notification::{NotificationLevel, NotificationService};
    use crate::services::speech::widget::RecordingWidget;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    async fn warn_permission_missing(app: &AppHandle) {
        unsafe { CGRequestScreenCaptureAccess() };
        log::info!("screenshot skipped: Screen Recording permission missing");
        NotificationService::notify_now(
            app,
            "screenshot_permission",
            NotificationLevel::Warning,
            "Screen Recording permission needed",
            Some("Allow Promptheus in System Settings → Privacy & Security → Screen Recording, then restart Promptheus"),
        )
        .await;
    }

    pub async fn capture(app: &AppHandle, generation: u64, path: &Path) -> bool {
        if !unsafe { CGPreflightScreenCaptureAccess() } {
            warn_permission_missing(app).await;
            return false;
        }
        let widget = app.state::<RecordingWidget>();
        if !widget.hide_for_capture(app, generation).await {
            return false;
        }
        let target = path.to_path_buf();
        let status = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("/usr/sbin/screencapture")
                .args(["-i", "-x"])
                .arg(target)
                .status()
        })
        .await;
        widget.restore_after_capture(app, generation).await;
        match status {
            Ok(Ok(status)) => log::debug!("screencapture exited: {status}"),
            Ok(Err(e)) => log::warn!("starting screencapture failed: {e}"),
            Err(e) => log::warn!("screencapture task failed: {e}"),
        }
        let captured = path.exists();
        if !captured {
            log::debug!("screenshot cancelled: no file written");
        }
        captured
    }
}

#[cfg(target_os = "macos")]
pub use macos::capture as capture_macos;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shown_unlocked_pill_of_the_same_generation_keeps_the_capture() {
        assert!(keep_capture(3, Some(3), false));
    }

    #[test]
    fn a_closed_pill_drops_the_capture() {
        assert!(!keep_capture(3, None, false));
    }

    #[test]
    fn another_generation_drops_the_capture() {
        assert!(!keep_capture(3, Some(4), false));
    }

    #[test]
    fn a_locked_target_drops_the_capture() {
        assert!(!keep_capture(3, Some(3), true));
    }
}
