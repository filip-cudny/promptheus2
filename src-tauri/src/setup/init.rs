use tauri::Manager;

use crate::services::{self, config::ConfigService};
use crate::setup;

pub fn run(app: &mut tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    services::capability_audit::assert_no_ai_webview_ipc_grant();

    #[cfg(target_os = "linux")]
    services::webkit_pressure::install();

    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        setup::menu::install(app)?;
    }

    let config_dir = app.path().app_config_dir()?;
    let resource_dir = app.path().resolve("", tauri::path::BaseDirectory::Resource)?;
    services::config::load_env(&config_dir);
    let config_service = ConfigService::load(&config_dir, Some(&resource_dir))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    log::info!("config loaded from {}", config_dir.display());

    if config_service.settings().show_tray_icon {
        setup::tray::install(app)?;
        log::info!("system tray initialized");
    }

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::MacosLauncher;
        app.handle().plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))?;
        services::autostart::reconcile(app.handle(), config_service.settings());
    }

    setup::windows::create(app)?;
    setup::shortcuts::register(app, config_service.settings())?;
    setup::state::manage(app, &config_dir, &resource_dir, config_service)?;
    setup::background::spawn_heartbeat(app.handle().clone());
    setup::background::spawn_ai_webview_cold_suspend(app.handle().clone());

    Ok(())
}
