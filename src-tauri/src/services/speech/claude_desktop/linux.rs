use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use gtk::gio::prelude::AppInfoExt;

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
// corrected by repeated live runs. Only trades exit-detection latency for wakeups.
const HANDOFF_POLL: Duration = Duration::from_millis(20);
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

fn split_exec(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted && matches!(chars.peek(), Some('"' | '`' | '$' | '\\')) => {
                current.extend(chars.next());
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    tokens.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        tokens.push(current);
    }
    tokens
}

fn expand_field_codes(token: &str, url: &str) -> (Option<String>, bool) {
    let mut out = String::new();
    let mut removed = false;
    let mut used_url = false;
    let mut chars = token.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('u' | 'U') => {
                out.push_str(url);
                used_url = true;
            }
            Some(_) => removed = true,
            None => out.push('%'),
        }
    }
    let dropped = removed && out.is_empty();
    ((!dropped).then_some(out), used_url)
}

fn handler_argv(exec: &str, url: &str) -> Option<Vec<String>> {
    let mut argv = Vec::new();
    let mut has_url = false;
    for token in split_exec(exec) {
        let (expanded, used_url) = expand_field_codes(&token, url);
        has_url |= used_url;
        argv.extend(expanded);
    }
    if argv.first().is_none_or(String::is_empty) {
        return None;
    }
    if !has_url {
        argv.push(url.to_string());
    }
    Some(argv)
}

async fn spawn_handler(url: String) -> Result<Child, Outcome> {
    let argv = tauri::async_runtime::spawn_blocking(move || {
        gtk::gio::AppInfo::default_for_uri_scheme("claude")
            .and_then(|info| info.commandline())
            .and_then(|exec| handler_argv(&exec.to_string_lossy(), &url))
    })
    .await
    .map_err(|e| Outcome::Failed(e.to_string()))?
    .ok_or_else(|| Outcome::Failed("no claude:// handler".into()))?;
    Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Outcome::Failed(format!("spawning the claude:// handler failed: {e}")))
}

fn reap(mut child: Child) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = child.wait();
    });
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
    let mut child = spawn_handler(url).await?;
    let spawned = Instant::now();
    log::debug!(target: LOG_TARGET, "handler spawned");
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().map_or_else(|| "signal".into(), |c| c.to_string());
                log::debug!(
                    target: LOG_TARGET,
                    "handler exited in {} ms (status {code})",
                    spawned.elapsed().as_millis(),
                );
                tokio::time::sleep(HANDOFF_SETTLE).await;
                log::debug!(target: LOG_TARGET, "settle {} ms done", HANDOFF_SETTLE.as_millis());
                return Ok(true);
            }
            Ok(None) if spawned.elapsed() < HANDOFF_TIMEOUT => {
                tokio::time::sleep(HANDOFF_POLL).await;
            }
            Ok(None) => {
                log::info!(
                    target: LOG_TARGET,
                    "handler did not exit within {} ms",
                    HANDOFF_TIMEOUT.as_millis(),
                );
                reap(child);
                return Err(Outcome::Timeout);
            }
            Err(e) => {
                reap(child);
                return Err(Outcome::Failed(format!("waiting for the handler failed: {e}")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "claude://claude.ai/new?surface=chat&q=a%20b&c=1";

    fn argv(exec: &str) -> Option<Vec<String>> {
        handler_argv(exec, URL)
    }

    fn strings(parts: &[&str]) -> Option<Vec<String>> {
        Some(parts.iter().map(|part| part.to_string()).collect())
    }

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

    #[test]
    fn handler_argv_substitutes_the_url_field_code_as_one_argument() {
        assert_eq!(argv("claude-desktop %U"), strings(&["claude-desktop", URL]));
    }

    #[test]
    fn handler_argv_groups_quoted_program_and_keeps_options() {
        assert_eq!(
            argv("\"/opt/Claude App/claude\" --x %u"),
            strings(&["/opt/Claude App/claude", "--x", URL]),
        );
    }

    #[test]
    fn handler_argv_removes_file_codes_and_appends_the_url() {
        assert_eq!(argv("app %f"), strings(&["app", URL]));
        assert_eq!(argv("app %F %i %c %k"), strings(&["app", URL]));
    }

    #[test]
    fn handler_argv_unescapes_percent_and_appends_the_url() {
        assert_eq!(argv("a 100%%"), strings(&["a", "100%", URL]));
    }

    #[test]
    fn handler_argv_unescapes_backslashes_inside_quotes() {
        assert_eq!(argv(r#"app "a \"b\" \$ \\" %u"#), strings(&["app", "a \"b\" $ \\", URL]));
    }

    #[test]
    fn handler_argv_without_a_program_is_none() {
        assert_eq!(argv("\"\""), None);
        assert_eq!(argv("   "), None);
        assert_eq!(argv("%f"), None);
    }
}
