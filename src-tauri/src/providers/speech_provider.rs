use crate::models::menu::{MenuItem, MenuItemType};
use crate::traits::MenuItemProvider;

pub struct SpeechMenuProvider {
    is_recording: bool,
    is_action_executing: bool,
}

impl SpeechMenuProvider {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            is_action_executing: false,
        }
    }

    pub fn set_recording(&mut self, recording: bool) {
        self.is_recording = recording;
    }

    pub fn set_action_executing(&mut self, executing: bool) {
        self.is_action_executing = executing;
    }
}

impl MenuItemProvider for SpeechMenuProvider {
    fn provider_name(&self) -> &str {
        "SpeechMenuProvider"
    }

    fn get_menu_items(&self) -> Vec<MenuItem> {
        let label = if self.is_recording {
            "Stop Recording"
        } else {
            "Speech to text"
        };

        let enabled = self.is_recording || !self.is_action_executing;

        vec![MenuItem {
            id: "system_speech_to_text".to_string(),
            label: label.to_string(),
            item_type: MenuItemType::Speech,
            data: Some(serde_json::json!({"type": "speech_to_text"})),
            enabled,
            separator_after: false,
            style: None,
            tooltip: None,
            submenu_items: None,
            icon: Some("mic".to_string()),
            section_id: None,
        }]
    }

    fn refresh(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name_matches_dynamic_providers() {
        let provider = SpeechMenuProvider::new();
        assert_eq!(provider.provider_name(), "SpeechMenuProvider");
    }

    #[test]
    fn default_state_shows_speech_to_text() {
        let provider = SpeechMenuProvider::new();
        let items = provider.get_menu_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "system_speech_to_text");
        assert_eq!(items[0].label, "Speech to text");
        assert_eq!(items[0].item_type, MenuItemType::Speech);
        assert_eq!(items[0].icon.as_deref(), Some("mic"));
        assert!(items[0].enabled);
    }

    #[test]
    fn recording_state_shows_stop_recording() {
        let mut provider = SpeechMenuProvider::new();
        provider.set_recording(true);
        let items = provider.get_menu_items();
        assert_eq!(items[0].label, "Stop Recording");
        assert!(items[0].enabled);
    }

    #[test]
    fn disabled_when_action_executing() {
        let mut provider = SpeechMenuProvider::new();
        provider.set_action_executing(true);
        let items = provider.get_menu_items();
        assert!(!items[0].enabled);
    }

    #[test]
    fn recording_overrides_executing_disable() {
        let mut provider = SpeechMenuProvider::new();
        provider.set_recording(true);
        provider.set_action_executing(true);
        let items = provider.get_menu_items();
        assert!(items[0].enabled);
        assert_eq!(items[0].label, "Stop Recording");
    }

    #[test]
    fn menu_item_has_correct_data() {
        let provider = SpeechMenuProvider::new();
        let items = provider.get_menu_items();
        let data = items[0].data.as_ref().unwrap();
        assert_eq!(data["type"], "speech_to_text");
    }
}
