use crate::commands;
use crate::services::recent_apps::RecentAppsState;

#[cfg(target_os = "macos")]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-macos.png");
#[cfg(not(target_os = "macos"))]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-linux.png");

pub fn install(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_menu_i = MenuItem::with_id(app, "show-menu", "Show Menu", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_menu_i, &sep1, &settings_i, &sep2, &quit_i])?;

    let tray_icon_image = tauri::image::Image::from_bytes(TRAY_ICON_PNG)?;

    let tray = TrayIconBuilder::with_id("main-tray").icon(tray_icon_image);
    #[cfg(target_os = "macos")]
    let tray = tray.icon_as_template(true);

    tray
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Promptheus")
        .on_menu_event(
            |app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
                "show-menu" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        RecentAppsState::record_frontmost(&app).await;
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

#[cfg(test)]
mod tests {
    #[test]
    fn tray_pngs_decode_with_expected_sizes() {
        let macos =
            tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-macos.png")).unwrap();
        assert_eq!((macos.width(), macos.height()), (36, 36));
        let linux =
            tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-linux.png")).unwrap();
        assert_eq!((linux.width(), linux.height()), (32, 32));
    }
}
