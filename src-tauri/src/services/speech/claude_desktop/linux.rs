use std::path::PathBuf;

use super::{Outcome, LOG_TARGET};
use crate::services::gnome_shell;

const CLAUDE_WM_CLASS: &str = "com.anthropic.Claude";
const TITLE_WATCH_MS: u32 = 15_000;

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
