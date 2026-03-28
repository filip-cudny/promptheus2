use crate::models::menu::{MenuItem, MenuItemType};
use crate::traits::MenuItemProvider;

pub struct LastInteractionMenuProvider;

impl LastInteractionMenuProvider {
    pub fn new() -> Self {
        Self
    }
}

impl MenuItemProvider for LastInteractionMenuProvider {
    fn provider_name(&self) -> &str {
        "LastInteractionMenuProvider"
    }

    fn get_menu_items(&self) -> Vec<MenuItem> {
        vec![MenuItem {
            id: "last_interaction_section".to_string(),
            label: "Last interaction".to_string(),
            item_type: MenuItemType::LastInteraction,
            data: None,
            enabled: true,
            separator_after: false,
            style: None,
            tooltip: None,
            submenu_items: None,
            icon: None,
            section_id: None,
        }]
    }

    fn refresh(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
