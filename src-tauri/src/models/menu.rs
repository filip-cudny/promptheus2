use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MenuItemType {
    Skill,
    Preset,
    History,
    System,
    Speech,
    Context,
    LastInteraction,
    SettingsSection,
    Chat,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub item_type: MenuItemType,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub separator_after: bool,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub submenu_items: Option<Vec<MenuItem>>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
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

    #[test]
    fn test_menu_item_from_minimal_json() {
        let json = r#"{"id":"item-1","label":"Test","item_type":"skill"}"#;
        let item: MenuItem = serde_json::from_str(json).unwrap();

        assert_eq!(item.id, "item-1");
        assert_eq!(item.label, "Test");
        assert_eq!(item.item_type, MenuItemType::Skill);
        assert!(item.enabled);
        assert!(!item.separator_after);
        assert!(item.data.is_none());
        assert!(item.style.is_none());
        assert!(item.tooltip.is_none());
        assert!(item.submenu_items.is_none());
        assert!(item.icon.is_none());
        assert!(item.section_id.is_none());
    }
}
