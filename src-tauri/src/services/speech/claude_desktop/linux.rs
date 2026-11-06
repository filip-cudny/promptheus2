use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use gtk::gio::prelude::*;
use gtk::{gio, glib};
use tokio::sync::oneshot;

use super::{Outcome, LOG_TARGET};
use crate::services::gnome_shell;

const CLAUDE_WM_CLASS: &str = "com.anthropic.Claude";
const TITLE_WATCH_MS: u32 = 15_000;
const RESTORE_TIMEOUT_MS: u32 = 2000;

// Starting value from the 2026-11-06 measurement (handler start +577-882 ms after the link), to be
// corrected by repeated live runs. Stops being correct when a handler that handed over to the
// running Claude regularly needs longer than this: delivery then ends in Timeout.
const HANDOFF_TIMEOUT: Duration = Duration::from_millis(5000);
// Starting value from the 2026-11-06 measurement (handler start +577-882 ms after the link), to be
// corrected by repeated live runs. Stops being correct when Claude opens /new later than this
// after the handler exited: Return then lands before the new chat.
const HANDOFF_SETTLE: Duration = Duration::from_millis(400);

pub async fn watch_titles() {
    if !gnome_shell::is_gnome_wayland() {
        return;
    }
    let result = async {
        let proxy = gnome_shell::proxy().await?;
        proxy.watch_app_titles(CLAUDE_WM_CLASS, TITLE_WATCH_MS).await
    }
    .await;
    if let Err(e) = result {
        log::debug!(target: LOG_TARGET, "title watch not started: {e}");
    }
}

pub async fn paste_into_claude(files: &[PathBuf]) -> Outcome {
    if !gnome_shell::is_gnome_wayland() {
        return Outcome::Failed("not a GNOME Wayland session".into());
    }
    let uris: Vec<String> = files
        .iter()
        .filter_map(|path| tauri::Url::from_file_path(path).ok())
        .map(String::from)
        .collect();
    let uri_refs: Vec<&str> = uris.iter().map(String::as_str).collect();
    let reply = async {
        let proxy = gnome_shell::proxy().await?;
        proxy.paste_into_app(CLAUDE_WM_CLASS, &uri_refs).await
    }
    .await
    .map_err(|e| match &e {
        zbus::Error::MethodError(name, _, _) => name.to_string(),
        other => other.to_string(),
    });
    Outcome::from_reply(reply)
}

fn launch_handler(url: &str) -> Result<Option<glib::Pid>, Outcome> {
    let info = gio::AppInfo::default_for_uri_scheme("claude")
        .and_then(|info| info.downcast::<gio::DesktopAppInfo>().ok())
        .ok_or_else(|| Outcome::Failed("no claude:// handler".into()))?;
    let mut pid = None;
    info.launch_uris_as_manager(
        &[url],
        None::<&gio::AppLaunchContext>,
        glib::SpawnFlags::SEARCH_PATH
            | glib::SpawnFlags::DO_NOT_REAP_CHILD
            | glib::SpawnFlags::STDOUT_TO_DEV_NULL
            | glib::SpawnFlags::STDERR_TO_DEV_NULL,
        None,
        Some(&mut |_, spawned| pid = Some(spawned)),
    )
    .map_err(|e| Outcome::Failed(format!("spawning the claude:// handler failed: {e}")))?;
    Ok(pid)
}

fn watch_exit(pid: glib::Pid) -> oneshot::Receiver<ExitStatus> {
    let (tx, rx) = oneshot::channel();
    let mut tx = Some(tx);
    glib::child_watch_add(pid, move |_, status| {
        if let Some(tx) = tx.take() {
            let _ = tx.send(ExitStatus::from_raw(status));
        }
    });
    rx
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowState {
    Absent,
    Shown,
    Restored,
    RestoreTimeout,
}

impl WindowState {
    fn from_reply(reply: &str) -> Option<Self> {
        match reply {
            "absent" => Some(Self::Absent),
            "shown" => Some(Self::Shown),
            "restored" => Some(Self::Restored),
            "restore_timeout" => Some(Self::RestoreTimeout),
            _ => None,
        }
    }
}

async fn restore_claude_window() -> WindowState {
    if !gnome_shell::is_gnome_wayland() {
        return WindowState::Absent;
    }
    let result = async {
        let proxy = gnome_shell::proxy().await?;
        proxy.restore_app_window(CLAUDE_WM_CLASS).await
    }
    .await;
    match result {
        Ok(reply) => WindowState::from_reply(&reply).map_or_else(
            || {
                log::debug!(target: LOG_TARGET, "unknown window state reply, treated as absent");
                WindowState::Absent
            },
            |state| {
                log::debug!(target: LOG_TARGET, "Claude window: {state:?}");
                state
            },
        ),
        Err(e) => {
            log::debug!(target: LOG_TARGET, "window check failed, treated as absent: {e}");
            WindowState::Absent
        }
    }
}

/// `Ok(false)`: no Claude window (or no way to tell), the caller opens the link itself.
pub async fn open_link(url: String) -> Result<bool, Outcome> {
    match restore_claude_window().await {
        WindowState::Absent => return Ok(false),
        WindowState::RestoreTimeout => {
            log::info!(
                target: LOG_TARGET,
                "Claude window not restored within {} ms",
                RESTORE_TIMEOUT_MS,
            );
            return Err(Outcome::Timeout);
        }
        WindowState::Shown | WindowState::Restored => {}
    }
    let pid = tauri::async_runtime::spawn_blocking(move || launch_handler(&url))
        .await
        .map_err(|e| Outcome::Failed(e.to_string()))??;
    let Some(pid) = pid else {
        log::info!(target: LOG_TARGET, "handler launched without a process to wait for");
        return Err(Outcome::Timeout);
    };
    let exited = watch_exit(pid);
    let spawned = Instant::now();
    log::debug!(target: LOG_TARGET, "handler spawned");
    match tokio::time::timeout(HANDOFF_TIMEOUT, exited).await {
        Ok(Ok(status)) => {
            let code = status.code().map_or_else(|| "signal".into(), |c| c.to_string());
            log::debug!(
                target: LOG_TARGET,
                "handler exited in {} ms (status {code})",
                spawned.elapsed().as_millis(),
            );
            tokio::time::sleep(HANDOFF_SETTLE).await;
            log::debug!(target: LOG_TARGET, "settle {} ms done", HANDOFF_SETTLE.as_millis());
            Ok(true)
        }
        Ok(Err(e)) => Err(Outcome::Failed(format!("waiting for the handler failed: {e}"))),
        Err(_) => {
            log::info!(
                target: LOG_TARGET,
                "handler did not exit within {} ms",
                HANDOFF_TIMEOUT.as_millis(),
            );
            Err(Outcome::Timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_state_from_reply_maps_the_extension_states() {
        assert_eq!(WindowState::from_reply("absent"), Some(WindowState::Absent));
        assert_eq!(WindowState::from_reply("shown"), Some(WindowState::Shown));
        assert_eq!(WindowState::from_reply("restored"), Some(WindowState::Restored));
        assert_eq!(WindowState::from_reply("restore_timeout"), Some(WindowState::RestoreTimeout));
    }

    #[test]
    fn window_state_from_reply_rejects_an_unknown_string() {
        assert_eq!(WindowState::from_reply("minimized"), None);
    }
}
