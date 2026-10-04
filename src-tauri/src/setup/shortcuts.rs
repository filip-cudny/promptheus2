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

    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        log::info!("GNOME Wayland session: shortcuts are grabbed by the shell extension");
        tauri::async_runtime::spawn(run_shell_shortcuts(
            app.handle().clone(),
            settings.clone(),
        ));
        return Ok(());
    }

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

#[cfg(target_os = "linux")]
async fn run_shell_shortcuts(app: tauri::AppHandle, initial_settings: Settings) {
    use std::sync::Arc;

    use futures::StreamExt;
    use tokio::sync::Mutex;

    use crate::services::config::ConfigService;
    use crate::services::hotkeys::sync_shell_shortcuts;

    let proxy = match crate::services::gnome_shell::proxy().await {
        Ok(proxy) => proxy,
        Err(e) => {
            log::warn!("failed to connect to the shell extension, shortcuts are disabled: {e}");
            return;
        }
    };
    let (mut ready, mut activated) = match (
        proxy.receive_ready().await,
        proxy.receive_shortcut_activated().await,
    ) {
        (Ok(ready), Ok(activated)) => (ready, activated),
        (Err(e), _) | (_, Err(e)) => {
            log::warn!("failed to subscribe to shell extension signals, shortcuts are disabled: {e}");
            return;
        }
    };

    let current_settings = || async {
        match app.try_state::<Arc<Mutex<ConfigService>>>() {
            Some(config) => config.lock().await.settings().clone(),
            None => initial_settings.clone(),
        }
    };

    sync_shell_shortcuts(&current_settings().await).await;

    loop {
        tokio::select! {
            Some(_) = ready.next() => {
                log::info!("shell extension ready, registering shortcuts");
                sync_shell_shortcuts(&current_settings().await).await;
            }
            Some(signal) = activated.next() => {
                let action = match signal.args() {
                    Ok(args) => args.action.to_string(),
                    Err(e) => {
                        log::warn!("invalid ShortcutActivated signal: {e}");
                        continue;
                    }
                };
                log::info!(
                    target: "app_lib::hotkey_handler",
                    "hotkey action: {}",
                    action,
                );
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    execute_hotkey_action(&app, &action).await;
                });
            }
            else => break,
        }
    }
    log::warn!("shell extension signal streams ended, shortcuts are disabled");
}

#[cfg(not(desktop))]
pub fn register(
    _app: &tauri::App,
    _settings: &crate::models::settings::Settings,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
