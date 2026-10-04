use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::commands;
use crate::models::settings::Settings;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::frontmost_app;
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::recent_apps::RecentAppsState;

pub struct ShortcutActionMap(pub RwLock<HashMap<String, String>>);

pub fn translate_shortcut(shortcut: &str, os: &str) -> Option<String> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.len() < 2 {
        return None;
    }

    let translated: Vec<String> = parts
        .iter()
        .map(|part| translate_key_part(part.trim(), os))
        .collect();

    Some(translated.join("+"))
}

fn translate_key_part(part: &str, os: &str) -> String {
    let lower = part.to_lowercase();
    match lower.as_str() {
        "cmd" => match os {
            "macos" => "Command".to_string(),
            _ => "Super".to_string(),
        },
        "ctrl" => "Control".to_string(),
        "shift" => "Shift".to_string(),
        "alt" => "Alt".to_string(),
        "meta" | "super" => "Super".to_string(),
        "space" => "Space".to_string(),
        "tab" => "Tab".to_string(),
        "enter" => "Enter".to_string(),
        "esc" => "Escape".to_string(),
        "delete" => "Delete".to_string(),
        "backspace" => "Backspace".to_string(),
        "up" => "ArrowUp".to_string(),
        "down" => "ArrowDown".to_string(),
        "left" => "ArrowLeft".to_string(),
        "right" => "ArrowRight".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "page_up" => "PageUp".to_string(),
        "page_down" => "PageDown".to_string(),
        s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => {
            format!("F{}", &s[1..])
        }
        s if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() => {
            s.to_uppercase()
        }
        _ => part.to_string(),
    }
}

fn matches_current_os(context: &str, os: &str) -> bool {
    let trimmed = context.trim();
    if let Some(rest) = trimmed.strip_prefix("os ==") {
        rest.trim() == os
    } else if let Some(rest) = trimmed.strip_prefix("os==") {
        rest.trim() == os
    } else {
        false
    }
}

pub fn get_active_bindings(settings: &Settings) -> Vec<(String, String)> {
    get_active_bindings_for_os(settings, std::env::consts::OS)
}

fn get_active_bindings_for_os(settings: &Settings, os: &str) -> Vec<(String, String)> {
    let mut bindings = Vec::new();
    for group in &settings.keymaps {
        if !matches_current_os(&group.context, os) {
            continue;
        }
        for (shortcut, action) in &group.bindings {
            if let Some(translated) = translate_shortcut(shortcut, os) {
                bindings.push((translated, action.clone()));
            }
        }
    }
    bindings
}

/// Human-readable shortcut currently bound to an action, e.g. "Shift+F1",
/// so user-facing hints stay correct after a rebind.
pub fn shortcut_for_action(settings: &Settings, action: &str) -> Option<String> {
    get_active_bindings(settings)
        .into_iter()
        .find(|(_, bound)| bound == action)
        .map(|(shortcut, _)| shortcut)
}

fn gtk_key_name(key: &str) -> Option<String> {
    let name = match key {
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "PageUp" => "Page_Up",
        "PageDown" => "Page_Down",
        "Enter" => "Return",
        "Space" => "space",
        "Backspace" => "BackSpace",
        "Escape" | "Tab" | "Delete" | "Home" | "End" => key,
        _ => {
            let is_function_key = key
                .strip_prefix('F')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=24).contains(&n));
            let is_single_character =
                key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
            return (is_function_key || is_single_character).then(|| key.to_string());
        }
    };
    Some(name.to_string())
}

pub fn to_gtk_accelerator(binding: &str) -> Option<String> {
    let mut parts: Vec<&str> = binding.split('+').collect();
    let key = parts.pop()?;
    let mut accelerator = String::new();
    for modifier in parts {
        match modifier {
            "Control" | "Shift" | "Alt" | "Super" => {
                accelerator.push_str(&format!("<{modifier}>"));
            }
            _ => {
                log::warn!("unsupported modifier '{modifier}' in shortcut {binding}");
                return None;
            }
        }
    }
    match gtk_key_name(key) {
        Some(name) => {
            accelerator.push_str(&name);
            Some(accelerator)
        }
        None => {
            log::warn!("unsupported key '{key}' in shortcut {binding}");
            None
        }
    }
}

pub fn accelerators_by_action(bindings: &[(String, String)]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (binding, action) in bindings {
        if let Some(accelerator) = to_gtk_accelerator(binding) {
            map.entry(action.clone()).or_default().push(accelerator);
        }
    }
    map
}

#[cfg(target_os = "linux")]
pub async fn sync_shell_shortcuts(settings: &Settings) {
    let accelerators = accelerators_by_action(&get_active_bindings(settings));
    let count: usize = accelerators.values().map(Vec::len).sum();
    let result = match crate::services::gnome_shell::proxy().await {
        Ok(proxy) => proxy.set_shortcuts(accelerators).await,
        Err(e) => Err(e),
    };
    match result {
        Ok(ungrabbed) => {
            for accelerator in &ungrabbed {
                log::warn!("shell extension could not grab shortcut {accelerator}");
            }
            log::info!(
                target: "app_lib::hotkey_handler",
                "{} shortcuts grabbed by the shell extension",
                count.saturating_sub(ungrabbed.len())
            );
        }
        Err(zbus::Error::MethodError(name, _, _))
            if matches!(
                name.as_str(),
                "org.freedesktop.DBus.Error.UnknownObject"
                    | "org.freedesktop.DBus.Error.UnknownMethod"
                    | "org.freedesktop.DBus.Error.ServiceUnknown"
            ) =>
        {
            log::info!("waiting for the shell extension");
        }
        Err(e) => log::warn!("failed to set shortcuts on the shell extension: {e}"),
    }
}

#[cfg(desktop)]
pub fn reload_shortcuts(app: &tauri::AppHandle, settings: &Settings) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    #[cfg(target_os = "linux")]
    if crate::services::gnome_shell::is_gnome_wayland() {
        let settings = settings.clone();
        tauri::async_runtime::spawn(async move {
            sync_shell_shortcuts(&settings).await;
        });
        return;
    }

    let global_shortcut = app.global_shortcut();

    log::debug!(target: "app_lib::hotkey_handler", "reload_shortcuts: unregistering all");
    if let Err(e) = global_shortcut.unregister_all() {
        log::error!("failed to unregister shortcuts: {}", e);
        return;
    }

    let bindings = get_active_bindings(settings);
    let mut new_action_map = HashMap::new();

    for (shortcut_str, action) in &bindings {
        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                let canonical = shortcut.into_string();
                new_action_map.insert(canonical.clone(), action.clone());
                if let Err(e) = global_shortcut.register(shortcut) {
                    log::warn!(
                        "failed to register shortcut {} ({}): {}",
                        shortcut_str, canonical, e
                    );
                }
            }
            Err(e) => {
                log::warn!("invalid shortcut {}: {}", shortcut_str, e);
            }
        }
    }

    let action_map_state = app.state::<ShortcutActionMap>();
    let mut map = action_map_state.0.write().unwrap();
    *map = new_action_map;

    log::info!(
        target: "app_lib::hotkey_handler",
        "reloaded {} global shortcuts",
        bindings.len()
    );
}

pub async fn execute_hotkey_action(app: &tauri::AppHandle, action: &str) {
    match action {
        "set_context_value" | "append_context_value" | "clear_context" => {
            execute_context_action(app, action).await;
        }
        "open_context_menu" => {
            let frontmost = frontmost_app::detect();
            let max = app
                .state::<Arc<Mutex<ConfigService>>>()
                .lock()
                .await
                .settings()
                .recent_apps_count;
            app.state::<Arc<RecentAppsState>>()
                .push(frontmost, max)
                .await;
            if let Err(e) = commands::menu::show_context_menu_window(app.clone()).await {
                log::error!("open_context_menu failed: {e}");
            }
        }
        "speech_to_text_toggle" => {
            if let Err(e) = commands::speech::toggle_speech_recording(app.clone(), None).await {
                log::error!("speech_to_text_toggle failed: {e}");
            }
        }
        "execute_active_prompt" => {
            log::warn!("hotkey action '{}' is not yet implemented", action);
        }
        _ => {
            log::warn!("unknown hotkey action: {}", action);
        }
    }
}

async fn execute_context_action(app: &tauri::AppHandle, action: &str) {
    let clipboard = app.state::<ClipboardService>();
    let context = app.state::<Arc<Mutex<ContextManagerService>>>();
    let config = app.state::<Arc<Mutex<ConfigService>>>();
    let notifications = app.state::<NotificationService>();
    match action {
        "set_context_value" => {
            let result: std::result::Result<(), String> = if clipboard.has_image() {
                match clipboard.get_image_base64() {
                    Ok((data, mt)) => {
                        context.lock().await.set_context_image(data, mt);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            } else {
                match clipboard.get_text() {
                    Ok(text) => {
                        context.lock().await.set_context(text);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            };
            if let Err(e) = result {
                log::error!("set_context_value hotkey failed: {}", e);
                return;
            }
            let notification_settings = config.lock().await.settings().notifications.clone();
            let _ = app.emit("context-changed", ());
            let _ = notifications.notify(
                "context_set",
                NotificationLevel::Success,
                "Context set",
                None::<String>,
                &notification_settings,
            );
        }
        "append_context_value" => {
            let result: std::result::Result<(), String> = if clipboard.has_image() {
                match clipboard.get_image_base64() {
                    Ok((data, mt)) => {
                        context.lock().await.append_context_image(data, mt);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            } else {
                match clipboard.get_text() {
                    Ok(text) => {
                        context.lock().await.append_context(text);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            };
            if let Err(e) = result {
                log::error!("append_context_value hotkey failed: {}", e);
                return;
            }
            let notification_settings = config.lock().await.settings().notifications.clone();
            let _ = app.emit("context-changed", ());
            let _ = notifications.notify(
                "context_append",
                NotificationLevel::Success,
                "Context appended",
                None::<String>,
                &notification_settings,
            );
        }
        "clear_context" => {
            context.lock().await.clear();
            let notification_settings = config.lock().await.settings().notifications.clone();
            let _ = app.emit("context-changed", ());
            let _ = notifications.notify(
                "context_cleared",
                NotificationLevel::Success,
                "Context cleared",
                None::<String>,
                &notification_settings,
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::KeymapGroup;
    use std::collections::HashMap;

    #[test]
    fn to_gtk_accelerator_translates_bindings() {
        let cases = [
            ("Control+F1", Some("<Control>F1")),
            ("Shift+F1", Some("<Shift>F1")),
            ("Super+Space", Some("<Super>space")),
            ("Alt+Enter", Some("<Alt>Return")),
            ("Control+Shift+A", Some("<Control><Shift>A")),
            ("Control+ArrowUp", Some("<Control>Up")),
            ("Control+PageDown", Some("<Control>Page_Down")),
            ("Control+Foo", None),
        ];
        for (binding, expected) in cases {
            assert_eq!(to_gtk_accelerator(binding).as_deref(), expected, "{binding}");
        }
    }

    #[test]
    fn accelerators_by_action_groups_bindings_and_skips_unknown_keys() {
        let bindings = vec![
            ("Control+F1".to_string(), "open_context_menu".to_string()),
            ("Shift+F1".to_string(), "open_context_menu".to_string()),
            ("Control+Foo".to_string(), "clear_context".to_string()),
        ];
        let map = accelerators_by_action(&bindings);
        assert_eq!(map.len(), 1);
        assert_eq!(
            map["open_context_menu"],
            vec!["<Control>F1".to_string(), "<Shift>F1".to_string()]
        );
    }

    #[test]
    fn test_translate_cmd_macos() {
        assert_eq!(
            translate_shortcut("cmd+f1", "macos"),
            Some("Command+F1".to_string())
        );
    }

    #[test]
    fn test_translate_cmd_linux() {
        assert_eq!(
            translate_shortcut("cmd+f1", "linux"),
            Some("Super+F1".to_string())
        );
    }

    #[test]
    fn test_translate_ctrl() {
        assert_eq!(
            translate_shortcut("ctrl+f1", "linux"),
            Some("Control+F1".to_string())
        );
    }

    #[test]
    fn test_translate_shift() {
        assert_eq!(
            translate_shortcut("shift+f3", "linux"),
            Some("Shift+F3".to_string())
        );
    }

    #[test]
    fn test_translate_complex_combo() {
        assert_eq!(
            translate_shortcut("ctrl+shift+a", "linux"),
            Some("Control+Shift+A".to_string())
        );
    }

    #[test]
    fn test_translate_special_keys() {
        assert_eq!(
            translate_shortcut("ctrl+space", "linux"),
            Some("Control+Space".to_string())
        );
        assert_eq!(
            translate_shortcut("alt+enter", "linux"),
            Some("Alt+Enter".to_string())
        );
        assert_eq!(
            translate_shortcut("ctrl+esc", "linux"),
            Some("Control+Escape".to_string())
        );
    }

    #[test]
    fn test_translate_arrow_keys() {
        assert_eq!(
            translate_shortcut("ctrl+up", "linux"),
            Some("Control+ArrowUp".to_string())
        );
        assert_eq!(
            translate_shortcut("ctrl+down", "linux"),
            Some("Control+ArrowDown".to_string())
        );
    }

    #[test]
    fn test_translate_page_keys() {
        assert_eq!(
            translate_shortcut("ctrl+page_up", "linux"),
            Some("Control+PageUp".to_string())
        );
        assert_eq!(
            translate_shortcut("ctrl+page_down", "linux"),
            Some("Control+PageDown".to_string())
        );
    }

    #[test]
    fn test_translate_meta_super() {
        assert_eq!(
            translate_shortcut("meta+a", "linux"),
            Some("Super+A".to_string())
        );
        assert_eq!(
            translate_shortcut("super+a", "linux"),
            Some("Super+A".to_string())
        );
    }

    #[test]
    fn test_translate_function_keys() {
        for i in 1..=20 {
            let input = format!("ctrl+f{}", i);
            let expected = format!("Control+F{}", i);
            assert_eq!(translate_shortcut(&input, "linux"), Some(expected));
        }
    }

    #[test]
    fn test_invalid_single_key() {
        assert_eq!(translate_shortcut("f1", "linux"), None);
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(translate_shortcut("", "linux"), None);
    }

    #[test]
    fn test_os_filtering_macos() {
        let settings = Settings {
            keymaps: vec![
                KeymapGroup {
                    context: "os == macos".to_string(),
                    bindings: HashMap::from([
                        ("cmd+f1".to_string(), "open_context_menu".to_string()),
                    ]),
                },
                KeymapGroup {
                    context: "os == linux".to_string(),
                    bindings: HashMap::from([
                        ("ctrl+f1".to_string(), "open_context_menu".to_string()),
                    ]),
                },
            ],
            ..Default::default()
        };

        let bindings = get_active_bindings_for_os(&settings, "macos");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "Command+F1");
        assert_eq!(bindings[0].1, "open_context_menu");
    }

    #[test]
    fn test_os_filtering_linux() {
        let settings = Settings {
            keymaps: vec![
                KeymapGroup {
                    context: "os == macos".to_string(),
                    bindings: HashMap::from([
                        ("cmd+f1".to_string(), "open_context_menu".to_string()),
                    ]),
                },
                KeymapGroup {
                    context: "os == linux".to_string(),
                    bindings: HashMap::from([
                        ("ctrl+f1".to_string(), "open_context_menu".to_string()),
                    ]),
                },
            ],
            ..Default::default()
        };

        let bindings = get_active_bindings_for_os(&settings, "linux");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "Control+F1");
        assert_eq!(bindings[0].1, "open_context_menu");
    }

    #[test]
    fn test_no_matching_os() {
        let settings = Settings {
            keymaps: vec![KeymapGroup {
                context: "os == windows".to_string(),
                bindings: HashMap::from([
                    ("ctrl+f1".to_string(), "open_context_menu".to_string()),
                ]),
            }],
            ..Default::default()
        };

        let bindings = get_active_bindings_for_os(&settings, "linux");
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_multiple_bindings_same_group() {
        let settings = Settings {
            keymaps: vec![KeymapGroup {
                context: "os == linux".to_string(),
                bindings: HashMap::from([
                    ("ctrl+f1".to_string(), "open_context_menu".to_string()),
                    ("ctrl+f2".to_string(), "execute_active_prompt".to_string()),
                    ("shift+f3".to_string(), "append_context_value".to_string()),
                ]),
            }],
            ..Default::default()
        };

        let bindings = get_active_bindings_for_os(&settings, "linux");
        assert_eq!(bindings.len(), 3);
    }

    #[test]
    fn shortcut_for_action_returns_bound_shortcut() {
        let settings = Settings {
            keymaps: vec![KeymapGroup {
                context: format!("os == {}", std::env::consts::OS),
                bindings: HashMap::from([(
                    "shift+f1".to_string(),
                    "speech_to_text_toggle".to_string(),
                )]),
            }],
            ..Default::default()
        };

        assert_eq!(
            shortcut_for_action(&settings, "speech_to_text_toggle"),
            Some("Shift+F1".to_string())
        );
        assert_eq!(shortcut_for_action(&settings, "clear_context"), None);
    }

    #[test]
    fn test_empty_keymaps() {
        let settings = Settings::default();
        let bindings = get_active_bindings_for_os(&settings, "linux");
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_invalid_context_format() {
        let settings = Settings {
            keymaps: vec![KeymapGroup {
                context: "invalid context".to_string(),
                bindings: HashMap::from([
                    ("ctrl+f1".to_string(), "open_context_menu".to_string()),
                ]),
            }],
            ..Default::default()
        };

        let bindings = get_active_bindings_for_os(&settings, "linux");
        assert!(bindings.is_empty());
    }
}
