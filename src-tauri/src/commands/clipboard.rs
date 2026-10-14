use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

use crate::services::clipboard::{self, ClipboardFingerprint, ClipboardService};
use crate::Error;

struct PendingCopy {
    token: String,
    fingerprint: ClipboardFingerprint,
}

static PENDING_COPY: Mutex<Option<PendingCopy>> = Mutex::new(None);

pub fn store_pending_copy(fingerprint: ClipboardFingerprint) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    *PENDING_COPY.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingCopy {
        token: token.clone(),
        fingerprint,
    });
    token
}

fn take_copy_if_unchanged(
    slot: &mut Option<PendingCopy>,
    token: &str,
    current: &ClipboardFingerprint,
) -> bool {
    if slot.as_ref().is_none_or(|pending| pending.token != token) {
        return false;
    }
    let pending = slot.take().expect("slot checked above");
    pending.fingerprint == *current && *current != ClipboardFingerprint::Unreadable
}

#[tauri::command(async)]
pub fn get_clipboard_text() -> crate::Result<String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| Error::Other(e.to_string()))?;
    let text = clipboard
        .get_text()
        .map_err(|e| Error::Other(e.to_string()))?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err(Error::Other("clipboard is empty".into()));
    }
    Ok(trimmed)
}

#[tauri::command(async)]
pub fn get_clipboard_image(
    clipboard: State<'_, ClipboardService>,
) -> crate::Result<(String, String)> {
    Ok(clipboard.get_image_base64()?)
}

#[tauri::command]
pub async fn copy_chat_answer_if_unchanged(
    app: AppHandle,
    token: String,
    text: String,
) -> crate::Result<bool> {
    let service = (*app.state::<ClipboardService>()).clone();
    let (token_matched, unchanged) = tokio::task::spawn_blocking(move || {
        let current = service.fingerprint();
        let mut slot = PENDING_COPY.lock().unwrap_or_else(|e| e.into_inner());
        let token_matched = slot.as_ref().is_some_and(|pending| pending.token == token);
        (token_matched, take_copy_if_unchanged(&mut slot, &token, &current))
    })
    .await
    .map_err(|e| Error::Other(e.to_string()))?;

    let copied = unchanged && {
        let app = app.clone();
        let written = tokio::task::spawn_blocking(move || clipboard::write_text(&app, &text))
            .await
            .map_err(|e| Error::Other(e.to_string()))?;
        if let Err(e) = &written {
            log::warn!("chat answer copy failed: {e}");
        }
        written.is_ok()
    };
    log::debug!("chat answer copy: token_matched={token_matched} unchanged={unchanged} copied={copied}");
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(token: &str, fingerprint: ClipboardFingerprint) -> Option<PendingCopy> {
        Some(PendingCopy { token: token.into(), fingerprint })
    }

    #[test]
    fn chat_copy_writes_on_pending_token_and_equal_fingerprint() {
        let mut s = slot("t", ClipboardFingerprint::Text(1));
        assert!(take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Text(1)));
        assert!(s.is_none());
    }

    #[test]
    fn chat_copy_skips_changed_fingerprint() {
        let mut s = slot("t", ClipboardFingerprint::Text(1));
        assert!(!take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Text(2)));
        assert!(s.is_none());
    }

    #[test]
    fn chat_copy_skips_unknown_or_replaced_token() {
        let mut s = slot("new", ClipboardFingerprint::Empty);
        assert!(!take_copy_if_unchanged(&mut s, "old", &ClipboardFingerprint::Empty));
        assert!(s.is_some());
        let mut empty = None;
        assert!(!take_copy_if_unchanged(&mut empty, "old", &ClipboardFingerprint::Empty));
    }

    #[test]
    fn chat_copy_second_call_with_same_token_skips() {
        let mut s = slot("t", ClipboardFingerprint::Empty);
        assert!(take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Empty));
        assert!(!take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Empty));
    }

    #[test]
    fn chat_copy_skips_unreadable_at_either_end() {
        let mut s = slot("t", ClipboardFingerprint::Unreadable);
        assert!(!take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Empty));
        let mut s = slot("t", ClipboardFingerprint::Empty);
        assert!(!take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Unreadable));
        let mut s = slot("t", ClipboardFingerprint::Unreadable);
        assert!(!take_copy_if_unchanged(&mut s, "t", &ClipboardFingerprint::Unreadable));
    }
}
