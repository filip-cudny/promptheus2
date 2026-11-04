#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::services::ai_webview::{fallback, read_context};
use crate::services::notification::{NotificationLevel, NotificationService};

const LOG_TARGET: &str = "app_lib::services::speech::claude_desktop";
const LINK_TEXT_LIMIT: usize = 14_336;
const RUN_DIR_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_SUBDIR: &str = "claude-desktop";
const CONTEXT_FILE: &str = "context.md";

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Sent,
    NoWindow,
    FocusLost,
    NoRead,
    NoPermission,
    Failed(String),
}

#[derive(Debug, PartialEq)]
enum Action {
    None,
    Fallback(&'static str),
    PasteToast,
}

impl Outcome {
    #[cfg(any(target_os = "linux", test))]
    pub fn from_reply(reply: Result<String, String>) -> Self {
        match reply.as_deref() {
            Ok("sent") => Outcome::Sent,
            Ok("no_window") => Outcome::NoWindow,
            Ok("focus_lost") => Outcome::FocusLost,
            Ok("no_read") => Outcome::NoRead,
            Ok(_) => Outcome::Failed("unknown outcome".into()),
            Err(e) => Outcome::Failed(e.into()),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Outcome::Sent => "Sent",
            Outcome::NoWindow => "NoWindow",
            Outcome::FocusLost => "FocusLost",
            Outcome::NoRead => "NoRead",
            Outcome::NoPermission => "NoPermission",
            Outcome::Failed(_) => "Failed",
        }
    }

    fn action(&self) -> Action {
        match self {
            Outcome::Sent => Action::None,
            Outcome::NoWindow => Action::Fallback("Claude Desktop did not open a window"),
            Outcome::NoPermission => Action::Fallback("Accessibility permission missing"),
            Outcome::Failed(_) => Action::Fallback("could not hand over to Claude Desktop"),
            Outcome::FocusLost | Outcome::NoRead => Action::PasteToast,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Placement {
    q: String,
    context_file: Option<String>,
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn full_text(context: Option<&str>, transcript: &str) -> String {
    match context {
        Some(context) => format!("{context}\n\n{transcript}"),
        None => transcript.to_string(),
    }
}

fn place(context: Option<&str>, transcript: &str) -> Placement {
    let full = full_text(context, transcript);
    if utf16_len(&full) <= LINK_TEXT_LIMIT {
        return Placement { q: full, context_file: None };
    }
    match context {
        Some(context) if utf16_len(transcript) <= LINK_TEXT_LIMIT => Placement {
            q: transcript.to_string(),
            context_file: Some(context.to_string()),
        },
        _ => Placement { q: String::new(), context_file: Some(full) },
    }
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn link(q: &str) -> String {
    format!("claude://claude.ai/new?surface=chat&q={}", percent_encode(q))
}

fn image_extension(media_type: &str) -> &'static str {
    match media_type {
        "image/jpeg" => "jpeg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn cache_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join(CACHE_SUBDIR))
        .map_err(|e| e.to_string())
}

fn write_files(
    dir: PathBuf,
    images: Vec<(String, String)>,
    context_file: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut paths = Vec::new();
    for (index, (media_type, data)) in images.iter().enumerate() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| e.to_string())?;
        let path = dir.join(format!("image-{}.{}", index + 1, image_extension(media_type)));
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        paths.push(path);
    }
    if let Some(context) = context_file {
        let path = dir.join(CONTEXT_FILE);
        std::fs::write(&path, context).map_err(|e| e.to_string())?;
        paths.push(path);
    }
    Ok(paths)
}

async fn prepare_files(
    app: &AppHandle,
    run: u64,
    images: Vec<(String, String)>,
    context_file: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    if images.is_empty() && context_file.is_none() {
        return Ok(Vec::new());
    }
    let dir = cache_root(app)?.join(run.to_string());
    tauri::async_runtime::spawn_blocking(move || write_files(dir, images, context_file))
        .await
        .map_err(|e| e.to_string())?
}

async fn paste(app: &AppHandle, files: &[PathBuf], warm: bool) -> Outcome {
    #[cfg(target_os = "linux")]
    {
        let _ = (app, warm);
        linux::paste_into_claude(files).await
    }
    #[cfg(target_os = "macos")]
    {
        macos::paste_into_claude(app, files, warm).await
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (app, files, warm);
        Outcome::Failed("unsupported platform".into())
    }
}

fn is_warm() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        Some(macos::claude_running())
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn accessibility_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::accessibility_trusted_or_prompt()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub async fn deliver(app: AppHandle, transcript: String) {
    let run = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    let (context, images) = read_context(&app).await;
    let full = full_text(context.as_deref(), &transcript);
    let Placement { q, context_file } = place(context.as_deref(), &transcript);

    let warm = is_warm();
    let (outcome, file_count) = if !accessibility_trusted() {
        (Outcome::NoPermission, 0)
    } else {
        match prepare_files(&app, run, images, context_file).await {
            Err(e) => {
                log::warn!(target: LOG_TARGET, "run={run}: writing the run files failed: {e}");
                (Outcome::Failed("writing files failed".into()), 0)
            }
            Ok(files) => {
                let outcome = match app.opener().open_url(link(&q), None::<&str>) {
                    Err(e) => {
                        log::warn!(target: LOG_TARGET, "run={run}: opening the link failed: {e}");
                        Outcome::Failed("opening the link failed".into())
                    }
                    Ok(()) => paste(&app, &files, warm.unwrap_or(false)).await,
                };
                (outcome, files.len())
            }
        }
    };

    let temperature = match warm {
        Some(true) => " warm",
        Some(false) => " cold",
        None => "",
    };
    log::info!(
        target: LOG_TARGET,
        "run={run} outcome={} files={file_count}{temperature}",
        outcome.name(),
    );
    if let Outcome::Failed(reason) = &outcome {
        log::warn!(target: LOG_TARGET, "run={run} failed: {reason}");
    }

    act(&app, run, full, &outcome).await;
    schedule_run_dir_removal(&app, run);
}

async fn act(app: &AppHandle, run: u64, full: String, outcome: &Outcome) {
    match outcome.action() {
        Action::None => {}
        Action::Fallback(reason) => {
            fallback(app, "claude", "Claude Desktop", Some(run), full, reason).await;
        }
        Action::PasteToast => {
            let key = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
            NotificationService::notify_now(
                app,
                "webview_send_failed",
                NotificationLevel::Error,
                "Not sent to Claude Desktop",
                Some(format!("paste with {key}+V and press Enter")),
            )
            .await;
        }
    }
}

fn schedule_run_dir_removal(app: &AppHandle, run: u64) {
    let Ok(dir) = cache_root(app).map(|root| root.join(run.to_string())) else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(RUN_DIR_TTL).await;
        remove_dir(dir).await;
    });
}

async fn remove_dir(dir: PathBuf) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::fs::remove_dir_all(dir)).await;
}

pub async fn clear_cache(app: &AppHandle) {
    if let Ok(root) = cache_root(app) {
        remove_dir(root).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(n: usize) -> String {
        "a".repeat(n)
    }

    #[test]
    fn claude_desktop_link_starts_with_the_new_chat_url() {
        assert!(link("").starts_with("claude://claude.ai/new?surface=chat&q="));
    }

    #[test]
    fn claude_desktop_link_percent_encodes_like_python_quote() {
        assert_eq!(percent_encode("a+b&c\nd 😀"), "a%2Bb%26c%0Ad%20%F0%9F%98%80");
    }

    #[test]
    fn claude_desktop_placement_transcript_at_and_over_the_limit() {
        let fits = place(None, &units(14_336));
        assert_eq!(fits, Placement { q: units(14_336), context_file: None });
        let over = place(None, &units(14_337));
        assert_eq!(over, Placement { q: String::new(), context_file: Some(units(14_337)) });
    }

    #[test]
    fn claude_desktop_placement_context_and_transcript_at_and_over_the_limit() {
        let context = units(100);
        let transcript = units(14_336 - 102);
        let inline = place(Some(&context), &transcript);
        assert_eq!(inline.context_file, None);
        assert_eq!(inline.q, format!("{context}\n\n{transcript}"));

        let transcript = units(14_336 - 101);
        let split = place(Some(&context), &transcript);
        assert_eq!(split, Placement { q: transcript, context_file: Some(context) });
    }

    #[test]
    fn claude_desktop_placement_counts_utf16_units() {
        assert_eq!(place(None, &"😀".repeat(7_168)).context_file, None);
        assert!(place(None, &"😀".repeat(7_169)).context_file.is_some());
    }

    #[test]
    fn claude_desktop_image_extensions() {
        assert_eq!(image_extension("image/png"), "png");
        assert_eq!(image_extension("image/jpeg"), "jpeg");
        assert_eq!(image_extension("image/gif"), "gif");
        assert_eq!(image_extension("image/webp"), "webp");
        assert_eq!(image_extension("image/bmp"), "png");
    }

    #[test]
    fn claude_desktop_outcome_parses_replies() {
        assert_eq!(Outcome::from_reply(Ok("sent".into())), Outcome::Sent);
        assert_eq!(Outcome::from_reply(Ok("no_window".into())), Outcome::NoWindow);
        assert_eq!(Outcome::from_reply(Ok("focus_lost".into())), Outcome::FocusLost);
        assert_eq!(Outcome::from_reply(Ok("no_read".into())), Outcome::NoRead);
        assert!(matches!(Outcome::from_reply(Ok("bogus".into())), Outcome::Failed(_)));
        let unknown = Outcome::from_reply(Err("org.freedesktop.DBus.Error.UnknownMethod".into()));
        assert!(matches!(unknown, Outcome::Failed(_)));
        assert!(matches!(unknown.action(), Action::Fallback(_)));
    }

    #[test]
    fn claude_desktop_outcome_actions() {
        assert!(matches!(Outcome::NoWindow.action(), Action::Fallback(_)));
        assert!(matches!(Outcome::NoPermission.action(), Action::Fallback(_)));
        assert_eq!(Outcome::FocusLost.action(), Action::PasteToast);
        assert_eq!(Outcome::NoRead.action(), Action::PasteToast);
        assert_eq!(Outcome::Sent.action(), Action::None);
    }
}
