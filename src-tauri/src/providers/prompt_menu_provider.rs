use crate::models::menu::{MenuItem, MenuItemType};
use crate::models::settings::PromptData;
use crate::traits::MenuItemProvider;

pub struct PromptMenuProvider {
    prompts: Vec<PromptData>,
}

impl PromptMenuProvider {
    pub fn new(prompts: Vec<PromptData>) -> Self {
        Self { prompts }
    }

    pub fn update_prompts(&mut self, prompts: Vec<PromptData>) {
        self.prompts = prompts;
    }
}

impl MenuItemProvider for PromptMenuProvider {
    fn provider_name(&self) -> &str {
        "PromptProvider"
    }

    fn get_menu_items(&self) -> Vec<MenuItem> {
        self.prompts
            .iter()
            .map(|prompt| MenuItem {
                id: prompt.id.clone(),
                label: prompt.name.clone(),
                item_type: MenuItemType::Prompt,
                data: Some(serde_json::json!({
                    "prompt_id": prompt.id,
                    "prompt_name": prompt.name,
                })),
                enabled: true,
                separator_after: false,
                style: None,
                tooltip: prompt.description.clone(),
                submenu_items: None,
                icon: None,
                section_id: None,
            })
            .collect()
    }

    fn refresh(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prompt(id: &str, name: &str) -> PromptData {
        PromptData {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            messages: Vec::new(),
        }
    }

    #[test]
    fn test_get_menu_items_maps_prompts() {
        let provider = PromptMenuProvider::new(vec![
            make_prompt("p1", "Summarize"),
            make_prompt("p2", "Translate"),
        ]);

        let items = provider.get_menu_items();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "p1");
        assert_eq!(items[0].label, "Summarize");
        assert_eq!(items[0].item_type, MenuItemType::Prompt);
        assert!(items[0].enabled);

        let data = items[0].data.as_ref().unwrap();
        assert_eq!(data["prompt_id"], "p1");
        assert_eq!(data["prompt_name"], "Summarize");
    }

    #[test]
    fn test_empty_prompts() {
        let provider = PromptMenuProvider::new(Vec::new());
        assert!(provider.get_menu_items().is_empty());
    }

    #[test]
    fn test_update_prompts() {
        let mut provider = PromptMenuProvider::new(vec![make_prompt("p1", "Old")]);
        assert_eq!(provider.get_menu_items().len(), 1);

        provider.update_prompts(vec![
            make_prompt("p2", "New1"),
            make_prompt("p3", "New2"),
        ]);
        let items = provider.get_menu_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "p2");
    }

    #[test]
    fn test_provider_name() {
        let provider = PromptMenuProvider::new(Vec::new());
        assert_eq!(provider.provider_name(), "PromptProvider");
    }
}
