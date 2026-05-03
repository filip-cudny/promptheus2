#[cfg(desktop)]
use std::collections::HashMap;
#[cfg(desktop)]
use std::sync::RwLock;

#[cfg(desktop)]
use tauri::Manager;

#[cfg(desktop)]
use crate::models::settings::Settings;
#[cfg(desktop)]
use crate::services::hotkeys::{execute_hotkey_action, get_active_bindings, ShortcutActionMap};

#[cfg(desktop)]
pub fn register(
    app: &tauri::App,
    settings: &Settings,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

    let bindings = get_active_bindings(settings);
    let mut action_map = HashMap::new();
    let mut builder = tauri_plugin_global_shortcut::Builder::new();
    for (shortcut_str, action) in &bindings {
        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                let canonical = shortcut.into_string();
                action_map.insert(canonical, action.clone());
                builder = builder.with_shortcut(shortcut_str.as_str())?;
            }
            Err(e) => {
                log::warn!("invalid shortcut {}: {}", shortcut_str, e);
            }
        }
    }
    app.manage(ShortcutActionMap(RwLock::new(action_map)));

    app.handle().plugin(
        builder
            .with_handler(|app, shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let shortcut_str = shortcut.into_string();
                    let action_map = app.state::<ShortcutActionMap>();
                    let map = action_map.0.read().unwrap();
                    if let Some(action) = map.get(&shortcut_str) {
                        let action = action.clone();
                        drop(map);
                        log::info!(
                            target: "app_lib::hotkey_handler",
                            "hotkey action: {} -> {}",
                            shortcut_str, action,
                        );
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            execute_hotkey_action(&app, &action).await;
                        });
                    } else {
                        log::warn!(
                            target: "app_lib::hotkey_handler",
                            "hotkey pressed but no action found: {}",
                            shortcut_str,
                        );
                    }
                }
            })
            .build(),
    )?;

    for (shortcut_str, action) in &bindings {
        log::info!("registered shortcut: {} -> {}", shortcut_str, action);
    }
    log::info!("{} global shortcuts registered", bindings.len());

    Ok(())
}

#[cfg(not(desktop))]
pub fn register(
    _app: &tauri::App,
    _settings: &crate::models::settings::Settings,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
