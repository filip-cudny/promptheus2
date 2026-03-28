use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

mod commands;
mod models;
mod providers;
mod services;
mod traits;

use commands::settings::AppState;
use services::ai::AiService;
use services::clipboard::ClipboardService;
use services::config::ConfigService;
use services::context::{ContextManagerService, ContextMenuProvider};
use services::history::HistoryService;
use services::image_storage::ImageStorage;
use services::menu_coordinator::MenuCoordinator;
use services::notification::NotificationService;
use services::placeholder::PlaceholderService;
use services::prompt_execution::PromptExecutionService;
use providers::{LastInteractionMenuProvider, PromptMenuProvider};

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
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = commands::menu::show_context_menu_window(app).await;
                });
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
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .level(log::LevelFilter::Info)
                .level_for("app_lib", log::LevelFilter::Debug)
                .build(),
        )
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
            log::info!("config loaded from {}", config_dir.display());

            if config_service.settings().show_tray_icon {
                setup_tray(app)?;
                log::info!("system tray initialized");
            }

            let clipboard_service = ClipboardService::new()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let notification_service = NotificationService::new(app.handle().clone());
            let mut menu_coordinator = MenuCoordinator::new();
            menu_coordinator.add_provider(Box::new(ContextMenuProvider::new()));
            menu_coordinator.add_provider(Box::new(LastInteractionMenuProvider::new()));
            menu_coordinator.add_provider(Box::new(PromptMenuProvider::new(
                config_service.settings().prompts.clone(),
            )));
            let ai_service = AiService::new(&config_service.settings().models);
            let context_service = ContextManagerService::new();
            let placeholder_service = PlaceholderService::new();
            let history_service = HistoryService::new(1000);

            let app_data_dir = app.path().app_data_dir()?;
            let image_storage = ImageStorage::new(&app_data_dir);
            image_storage
                .initialize()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            log::info!("image storage initialized at {}", app_data_dir.display());

            app.manage(Mutex::new(AppState {
                config: config_service,
                clipboard: clipboard_service,
                notifications: notification_service,
                menu_coordinator,
                context: context_service,
                placeholder: placeholder_service,
                ai: ai_service,
                history: history_service,
                image_storage,
                prompt_execution: PromptExecutionService::new(),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ai::complete,
            commands::ai::complete_stream,
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
            commands::context::get_context_items,
            commands::context::get_context_text,
            commands::context::has_context,
            commands::context::has_context_images,
            commands::context::set_context,
            commands::context::append_context,
            commands::context::clear_context,
            commands::context::remove_context_item,
            commands::context::set_context_image,
            commands::context::append_context_image,
            commands::context::set_context_from_clipboard,
            commands::context::append_context_from_clipboard,
            commands::menu::get_context_menu_items,
            commands::menu::execute_menu_item,
            commands::menu::refresh_menu_providers,
            commands::menu::show_context_menu_window,
            commands::history::get_history,
            commands::history::get_history_entry,
            commands::history::add_history_entry,
            commands::history::add_conversation_entry,
            commands::history::update_conversation_entry,
            commands::history::get_last_interaction,
            commands::history::clear_history,
            commands::history::copy_history_content,
            commands::prompt_execution::execute_prompt,
            commands::prompt_execution::execute_conversation_turn,
            commands::prompt_execution::get_execution_state,
            commands::prompt_dialog::open_prompt_dialog,
            commands::notification::update_notification_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
