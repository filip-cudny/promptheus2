use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

mod commands;
mod error;
mod models;
mod providers;
mod services;
mod setup;
mod traits;

pub use error::{Error, Result};

use services::config::ConfigService;
use services::frontmost_app;
use services::recent_apps::RecentAppsState;

#[cfg(desktop)]
pub use services::hotkeys::reload_shortcuts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            log::info!(target: "app_lib", "second instance suppressed");
        }))
        .plugin(setup::log::plugin())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("platform")
                .js_init_script(format!(
                    "document.documentElement.dataset.platform = '{}'",
                    std::env::consts::OS
                ))
                .build(),
        )
        .setup(|app| setup::init::run(app))
        .invoke_handler(crate::handlers!())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let detected = frontmost_app::detect();
                    let max = app
                        .state::<Arc<Mutex<ConfigService>>>()
                        .lock()
                        .await
                        .settings()
                        .recent_apps_count;
                    app.state::<Arc<RecentAppsState>>().push(detected, max).await;
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
