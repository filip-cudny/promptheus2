use tauri::Emitter;
use tauri::Manager;

pub fn install(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    let app_submenu = Submenu::with_items(
        app,
        "Promptheus",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_submenu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let reload_active = MenuItem::with_id(
        app,
        "reload-active-provider",
        "Reload Active Provider",
        true,
        Some("CmdOrCtrl+R"),
    )?;

    let view_submenu = Submenu::with_items(app, "View", true, &[&reload_active])?;

    let menu = Menu::with_items(app, &[&app_submenu, &edit_submenu, &view_submenu])?;
    app.set_menu(menu)?;

    app.on_menu_event(|handle, event| {
        if event.id().as_ref() != "reload-active-provider" {
            return;
        }
        let Some(window) = handle.get_focused_window() else {
            return;
        };
        let label = window.label().to_string();
        if let Err(e) = handle.emit_to(label.as_str(), "menu:reload-active", ()) {
            log::warn!(
                target: "app_lib::menu",
                "emit menu:reload-active to {label} failed: {e}",
            );
        }
    });

    Ok(())
}
