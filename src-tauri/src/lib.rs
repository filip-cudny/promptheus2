use std::sync::Arc;

use tauri::Manager;

mod commands;
mod error;
mod models;
mod services;
mod setup;

pub use error::{Error, Result};

use services::recent_apps::RecentAppsState;

#[cfg(desktop)]
pub use services::hotkeys::reload_shortcuts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    let webkit_renderer_decision = services::webkit_renderer::configure();
    let builder = tauri::Builder::default()
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
        );
    #[cfg(target_os = "linux")]
    let builder = builder.plugin(services::scale_repaint::plugin());
    builder
        .setup(move |app| {
            #[cfg(target_os = "linux")]
            log::info!(target: "app_lib", "{webkit_renderer_decision}");
            setup::init::run(app)
        })
        .invoke_handler(crate::handlers!())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    RecentAppsState::record_frontmost(&app).await;
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                close_mcp_servers(app);
            }
        });
}

fn close_mcp_servers(app: &tauri::AppHandle) {
    let Some(registry) = app.try_state::<Arc<services::mcp::McpRegistry>>() else {
        return;
    };
    log::info!("Closing MCP servers");
    tauri::async_runtime::block_on(registry.close_all(tokio::time::Duration::from_secs(4)));
    log::info!("MCP servers closed");
}
