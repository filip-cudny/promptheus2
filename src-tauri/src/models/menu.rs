use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MenuItemType {
    Prompt,
    Preset,
    History,
    System,
    Speech,
    Context,
    LastInteraction,
    SettingsSection,
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
