use std::path::PathBuf;

use super::Outcome;
use crate::services::gnome_shell;

const CLAUDE_WM_CLASS: &str = "com.anthropic.Claude";

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
