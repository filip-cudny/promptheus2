use tauri::Manager;

use super::AiWebviewState;

pub(super) const CONVERSATION_DIALOG_TITLE: &str = "Promptheus — chat";

pub fn host_window_for_webview(app: &tauri::AppHandle, webview_label: &str) -> Option<String> {
    app.get_webview(webview_label)
        .map(|w| w.window().label().to_string())
}

pub fn cleanup_host_state(app: &tauri::AppHandle, host_label: &str) {
    if let Some(state) = app.try_state::<AiWebviewState>() {
        state.drop_host(host_label);
    }
}

pub(super) fn enable_media_for_webview(webview: &tauri::Webview) {
    #[cfg(target_os = "linux")]
    {
        let label = webview.label().to_string();
        let app = webview.app_handle().clone();
        let label_for_install = label.clone();
        if let Err(e) = webview.with_webview(move |pv| {
            install_media_permissions(pv, app, label_for_install);
        }) {
            log::warn!(
                target: "app_lib::services::ai_webview",
                "install media permissions failed for webview {label}: {e}",
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = webview;
    }
}

#[cfg(target_os = "linux")]
fn install_media_permissions(
    pv: tauri::webview::PlatformWebview,
    app: tauri::AppHandle,
    webview_label: String,
) {
    use webkit2gtk::glib;
    use webkit2gtk::glib::object::ObjectExt;
    use webkit2gtk::{
        DeviceInfoPermissionRequest, PermissionRequestExt, SettingsExt,
        UserMediaPermissionRequest, WebViewExt,
    };

    let wk = pv.inner();
    if let Some(settings) = WebViewExt::settings(&wk) {
        settings.set_enable_media_stream(true);
        settings.set_enable_mediasource(true);
        settings.set_enable_media_capabilities(true);
        settings.set_enable_webrtc(true);
    }

    wk.connect_permission_request(move |_, req| {
        if req.is::<UserMediaPermissionRequest>() || req.is::<DeviceInfoPermissionRequest>() {
            req.allow();
            true
        } else {
            false
        }
    });

    let label_for_term = webview_label.clone();
    wk.connect_web_process_terminated(move |_, reason| {
        log::warn!(
            target: "app_lib::services::ai_webview",
            "web process terminated: webview={label_for_term} reason={reason:?}",
        );
    });

    let label_for_watch = webview_label;
    let app_for_watch = app;
    wk.connect_is_web_process_responsive_notify(move |wv| {
        let responsive = wv.is_web_process_responsive();
        log::warn!(
            target: "app_lib::services::ai_webview",
            "responsiveness changed: webview={label_for_watch} responsive={responsive}",
        );
        if responsive {
            return;
        }
        let label = label_for_watch.clone();
        let app = app_for_watch.clone();
        let wv_weak = wv.downgrade();
        glib::timeout_add_seconds_local_once(
            super::unload::UNRESPONSIVE_GRACE_SECONDS,
            move || {
                let Some(wv) = wv_weak.upgrade() else { return };
                if wv.is_web_process_responsive() {
                    log::info!(
                        target: "app_lib::services::ai_webview",
                        "responsiveness recovered: webview={label}",
                    );
                    return;
                }
                log::error!(
                    target: "app_lib::services::ai_webview",
                    "web process unresponsive >{}s, restarting: webview={label}",
                    super::unload::UNRESPONSIVE_GRACE_SECONDS,
                );
                wv.terminate_web_process();
                wv.reload();
                let provider_name = super::unload::provider_display_name(&app, &label);
                super::unload::emit_provider_lifecycle_toast(
                    &app,
                    "provider_restarted",
                    crate::services::notification::NotificationLevel::Warning,
                    format!("{provider_name} was unresponsive"),
                    Some(format!(
                        "Restarted to recover (was unresponsive {}s+)",
                        super::unload::UNRESPONSIVE_GRACE_SECONDS,
                    )),
                );
            },
        );
    });
}
