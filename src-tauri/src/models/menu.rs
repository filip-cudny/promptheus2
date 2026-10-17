use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MenuItemType {
    Skill,
    Speech,
    Context,
    LastInteraction,
    SettingsSection,
    Chat,
    Models,
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub item_type: MenuItemType,
    pub data: Option<serde_json::Value>,
    pub enabled: bool,
    pub separator_after: bool,
    pub tooltip: Option<String>,
    pub icon: Option<String>,
    pub section_id: Option<String>,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>, item_type: MenuItemType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            item_type,
            data: None,
            enabled: true,
            separator_after: false,
            tooltip: None,
            icon: None,
            section_id: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_tooltip(mut self, tooltip: Option<String>) -> Self {
        self.tooltip = tooltip;
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_menu_item_type_serialization() {
        assert_eq!(serde_json::to_string(&MenuItemType::Skill).unwrap(), "\"skill\"");
        assert_eq!(
            serde_json::to_string(&MenuItemType::LastInteraction).unwrap(),
            "\"last_interaction\""
        );
        assert_eq!(
            serde_json::to_string(&MenuItemType::SettingsSection).unwrap(),
            "\"settings_section\""
        );
    }
}
