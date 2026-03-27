use tauri::Manager;
use tokio::sync::Mutex;

mod commands;
mod models;
mod services;

use commands::settings::AppState;
use services::config::ConfigService;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            services::config::load_env(&config_dir);
            let config_service = ConfigService::load(&config_dir)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            app.manage(Mutex::new(AppState {
                config: config_service,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
