use std::collections::HashMap;
use std::sync::RwLock;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::commands;
use crate::commands::settings::AppState;
use crate::models::settings::Settings;
use crate::services::frontmost_app;
use crate::services::notification::NotificationLevel;

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

#[cfg(desktop)]
pub fn reload_shortcuts(app: &tauri::AppHandle, settings: &Settings) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

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
            {
                let state = app.state::<Mutex<AppState>>();
                state.lock().await.push_active_app(frontmost);
            }
            if let Err(e) = commands::menu::show_context_menu_window(app.clone()).await {
                log::error!("open_context_menu failed: {e}");
            }
        }
        "speech_to_text_toggle" => {
            let state = app.state::<Mutex<AppState>>();
            if let Err(e) =
                commands::speech::toggle_speech_recording(app.clone(), state, None).await
            {
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
    let state = app.state::<Mutex<AppState>>();
    match action {
        "set_context_value" => {
            let notification_settings = {
                let mut s = state.lock().await;
                let result: std::result::Result<(), String> = if s.clipboard.has_image() {
                    s.clipboard
                        .get_image_base64()
                        .map(|(data, mt)| s.context.set_context_image(data, mt))
                        .map_err(|e| e.to_string())
                } else {
                    s.clipboard
                        .get_text()
                        .map(|text| s.context.set_context(text))
                        .map_err(|e| e.to_string())
                };
                if let Err(e) = result {
                    log::error!("set_context_value hotkey failed: {}", e);
                    return;
                }
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
                "context_set",
                NotificationLevel::Success,
                "Context set",
                None::<String>,
                &notification_settings,
            );
        }
        "append_context_value" => {
            let notification_settings = {
                let mut s = state.lock().await;
                let result: std::result::Result<(), String> = if s.clipboard.has_image() {
                    s.clipboard
                        .get_image_base64()
                        .map(|(data, mt)| s.context.append_context_image(data, mt))
                        .map_err(|e| e.to_string())
                } else {
                    s.clipboard
                        .get_text()
                        .map(|text| s.context.append_context(text))
                        .map_err(|e| e.to_string())
                };
                if let Err(e) = result {
                    log::error!("append_context_value hotkey failed: {}", e);
                    return;
                }
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
                "context_append",
                NotificationLevel::Success,
                "Context appended",
                None::<String>,
                &notification_settings,
            );
        }
        "clear_context" => {
            let notification_settings = {
                let mut s = state.lock().await;
                s.context.clear();
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
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
