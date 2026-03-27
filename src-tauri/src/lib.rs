use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

mod commands;
mod models;
mod services;
mod traits;

use commands::settings::AppState;
use services::clipboard::ClipboardService;
use services::config::ConfigService;
use services::menu_coordinator::MenuCoordinator;
use services::notification::NotificationService;

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_menu_i = MenuItem::with_id(app, "show-menu", "Show Menu", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_menu_i, &sep1, &settings_i, &sep2, &quit_i])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Promptheus")
        .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
            "show-menu" => {
                let _ = app.emit("show-context-menu", ());
            }
            "settings" => {
                let _ = app.emit("open-settings", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let resource_dir =
                app.path()
                    .resolve("", tauri::path::BaseDirectory::Resource)?;
            services::config::load_env(&config_dir);
            let config_service =
                ConfigService::load(&config_dir, Some(&resource_dir))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            if config_service.settings().show_tray_icon {
                setup_tray(app)?;
            }

            let clipboard_service = ClipboardService::new()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let notification_service = NotificationService::new(app.handle().clone());
            let menu_coordinator = MenuCoordinator::new();
            app.manage(Mutex::new(AppState {
                config: config_service,
                clipboard: clipboard_service,
                notifications: notification_service,
                menu_coordinator,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::clipboard::get_clipboard_text,
            commands::clipboard::set_clipboard_text,
            commands::clipboard::clipboard_is_empty,
            commands::clipboard::clipboard_has_image,
            commands::clipboard::get_clipboard_image,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::settings::add_model,
            commands::settings::update_model,
            commands::settings::delete_model,
            commands::settings::add_prompt,
            commands::settings::update_prompt,
            commands::settings::delete_prompt,
            commands::settings::reorder_prompts,
            commands::settings::update_notifications,
            commands::settings::update_speech_model,
            commands::settings::update_keymaps,
            commands::settings::update_menu_section_order,
            commands::settings::reload_settings,
            commands::menu::get_context_menu_items,
            commands::menu::execute_menu_item,
            commands::menu::refresh_menu_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
