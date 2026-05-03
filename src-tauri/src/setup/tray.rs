use tokio::sync::Mutex;

use crate::commands;
use crate::commands::settings::AppState;
use crate::services::frontmost_app;

pub fn install(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::Manager;

    let show_menu_i = MenuItem::with_id(app, "show-menu", "Show Menu", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_menu_i, &sep1, &settings_i, &sep2, &quit_i])?;

    let tray_icon_image =
        tauri::image::Image::from_bytes(include_bytes!("../../icons/tray_icon.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon_image)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Promptheus")
        .on_menu_event(
            |app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
                "show-menu" => {
                    let frontmost = frontmost_app::detect();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        {
                            let state = app.state::<Mutex<AppState>>();
                            state.lock().await.push_active_app(frontmost);
                        }
                        let _ = commands::menu::show_context_menu_window(app).await;
                    });
                }
                "settings" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) =
                            commands::settings_dialog::open_settings_window(app, None).await
                        {
                            log::error!("failed to open settings window: {e}");
                        }
                    });
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            },
        )
        .build(app)?;

    Ok(())
}
