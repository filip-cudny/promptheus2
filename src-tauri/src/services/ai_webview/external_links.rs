use tauri_plugin_opener::OpenerExt;

const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];

fn validated_url(raw_url: &str) -> Option<tauri::Url> {
    let parsed = match tauri::Url::parse(raw_url) {
        Ok(u) => u,
        Err(e) => {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "open_external_url: invalid url={raw_url} err={e}"
            );
            return None;
        }
    };

    let scheme = parsed.scheme();
    if !ALLOWED_SCHEMES.contains(&scheme) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "open_external_url: blocked scheme={scheme} url={raw_url}"
        );
        return None;
    }

    Some(parsed)
}

pub(super) fn open_external_url(app: &tauri::AppHandle, raw_url: &str) {
    let Some(parsed) = validated_url(raw_url) else {
        return;
    };

    log::info!(
        target: "app_lib::services::ai_webview",
        "open_external_url: opening url={parsed}"
    );

    if let Err(e) = app.opener().open_url(parsed.as_str(), None::<&str>) {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "open_external_url: opener.open_url failed url={raw_url} err={e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_passes() {
        assert!(validated_url("https://example.com/path?q=1").is_some());
    }

    #[test]
    fn http_passes() {
        assert!(validated_url("http://example.com/").is_some());
    }

    #[test]
    fn mailto_passes() {
        assert!(validated_url("mailto:user@example.com").is_some());
    }

    #[test]
    fn tel_passes() {
        assert!(validated_url("tel:+1234567890").is_some());
    }

    #[test]
    fn file_scheme_blocked() {
        assert!(validated_url("file:///etc/passwd").is_none());
    }

    #[test]
    fn javascript_scheme_blocked() {
        assert!(validated_url("javascript:alert(1)").is_none());
    }

    #[test]
    fn vscode_scheme_blocked() {
        assert!(validated_url("vscode://file/etc/passwd").is_none());
    }

    #[test]
    fn smb_scheme_blocked() {
        assert!(validated_url("smb://attacker/share").is_none());
    }

    #[test]
    fn empty_url_rejected() {
        assert!(validated_url("").is_none());
    }

    #[test]
    fn unparseable_url_rejected() {
        assert!(validated_url("not a url at all").is_none());
    }
}
