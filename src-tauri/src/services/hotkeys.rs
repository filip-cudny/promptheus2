use std::collections::HashMap;
use std::sync::RwLock;

use crate::models::settings::Settings;

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
