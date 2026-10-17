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
