use crate::models::context::ContextItem;
use crate::models::menu::{MenuItem, MenuItemType};
use crate::services::config::ConfigService;
use crate::services::context::ContextMenuProvider;
use crate::traits::MenuItemProvider;

const DYNAMIC_PROVIDERS: &[&str] = &[
    "LastInteractionMenuProvider",
    "ContextMenuProvider",
    "SpeechMenuProvider",
];

fn resolve_provider_name(section_id: &str) -> &str {
    match section_id {
        "speech" => "SpeechMenuProvider",
        "lastInteraction" => "LastInteractionMenuProvider",
        "context" => "ContextMenuProvider",
        other => other,
    }
}

pub struct MenuCoordinator {
    providers: Vec<Box<dyn MenuItemProvider>>,
}

impl MenuCoordinator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn MenuItemProvider>) {
        self.providers.push(provider);
    }

    pub fn get_menu_items(&self, config: &ConfigService) -> Vec<MenuItem> {
        let section_order = &config.settings().menu_section_order;
        let total_sections = section_order.len();
        let mut all_items: Vec<MenuItem> = Vec::new();

        for (section_index, section_id) in section_order.iter().enumerate() {
            let is_last_section = section_index == total_sections - 1;

            let mut section_items = match section_id.as_str() {
                "chat" => self.build_chat_items(),
                "skills" | "prompts" => self.build_skill_items(),
                "models" => self.build_models_items(config),
                "settings" => self.build_settings_items(config),
                _ => self.build_provider_items(section_id),
            };

            if section_items.is_empty() {
                continue;
            }

            for item in &mut section_items {
                item.section_id = Some(section_id.clone());
            }

            all_items.extend(section_items);

            if !is_last_section {
                if let Some(last) = all_items.last_mut() {
                    last.separator_after = true;
                }
            }
        }

        all_items
    }

    pub fn update_context_items(&mut self, items: Vec<ContextItem>) {
        for provider in &mut self.providers {
            if provider.provider_name() == "ContextMenuProvider" {
                if let Some(ctx) = provider.as_any_mut().downcast_mut::<ContextMenuProvider>() {
                    ctx.update_items(items);
                    return;
                }
            }
        }
    }

    pub fn providers_mut(&mut self) -> &mut Vec<Box<dyn MenuItemProvider>> {
        &mut self.providers
    }

    pub fn refresh_all(&mut self) {
        for provider in &mut self.providers {
            provider.refresh();
        }
    }

    fn build_provider_items(&self, section_id: &str) -> Vec<MenuItem> {
        let provider_name = resolve_provider_name(section_id);
        for provider in &self.providers {
            if provider.provider_name() == provider_name {
                let items = provider.get_menu_items();
                if !items.is_empty() {
                    return items;
                }
            }
        }
        Vec::new()
    }

    fn build_chat_items(&self) -> Vec<MenuItem> {
        vec![MenuItem {
            id: "__chat__".to_string(),
            label: "Chat".to_string(),
            item_type: MenuItemType::Chat,
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

    fn build_skill_items(&self) -> Vec<MenuItem> {
        let mut skill_items = Vec::new();
        for provider in &self.providers {
            if DYNAMIC_PROVIDERS.contains(&provider.provider_name()) {
                continue;
            }
            let items = provider.get_menu_items();
            skill_items.extend(items);
        }
        skill_items
    }

    fn build_models_items(&self, config: &ConfigService) -> Vec<MenuItem> {
        let settings = config.settings();
        if settings.models.is_empty() {
            return Vec::new();
        }

        let default_model_id = settings
            .surfaces
            .quick_actions
            .generation
            .model_id
            .clone()
            .or_else(|| settings.surfaces.chat.generation.model_id.clone());

        let default_reasoning_effort = settings
            .surfaces
            .quick_actions
            .generation
            .parameters
            .reasoning_effort
            .clone();

        let models: Vec<serde_json::Value> = settings
            .models
            .iter()
            .filter(|m| m.is_text())
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "display_name": m.display_name,
                    "model": m.model,
                    "provider": m.provider,
                    "group": m.group,
                })
            })
            .collect();

        let stt_models: Vec<serde_json::Value> = settings
            .models
            .iter()
            .filter(|m| m.is_stt())
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "display_name": m.display_name,
                    "model": m.model,
                    "provider": m.provider,
                    "group": m.group,
                })
            })
            .collect();

        vec![MenuItem {
            id: "__models__".to_string(),
            label: "Models".to_string(),
            item_type: MenuItemType::Models,
            data: Some(serde_json::json!({
                "models": models,
                "default_model_id": default_model_id,
                "default_reasoning_effort": default_reasoning_effort,
                "stt_models": stt_models,
                "speech_to_text_model_id": settings.surfaces.speech_to_text.model_id,
            })),
            enabled: true,
            separator_after: false,
            style: None,
            tooltip: None,
            submenu_items: None,
            icon: None,
            section_id: None,
        }]
    }

    fn build_settings_items(&self, config: &ConfigService) -> Vec<MenuItem> {
        let settings = config.settings();
        let chat_default_id = settings.surfaces.chat.generation.model_id.clone();

        let default_model_display = chat_default_id
            .as_ref()
            .and_then(|default_id| {
                settings
                    .models
                    .iter()
                    .find(|m| &m.id == default_id)
                    .map(|m| m.display_name.clone())
            })
            .or_else(|| chat_default_id.clone())
            .unwrap_or_default();

        let model_options: Vec<serde_json::Value> = settings
            .models
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "display_name": m.display_name,
                    "is_default": chat_default_id.as_ref() == Some(&m.id),
                })
            })
            .collect();

        vec![MenuItem {
            id: "settings_section".to_string(),
            label: "Settings".to_string(),
            item_type: MenuItemType::SettingsSection,
            data: Some(serde_json::json!({
                "model_options": model_options,
                "current_model": default_model_display,
            })),
            enabled: true,
            separator_after: false,
            style: None,
            tooltip: None,
            submenu_items: None,
            icon: None,
            section_id: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider {
        name: String,
        items: Vec<MenuItem>,
    }

    impl TestProvider {
        fn new(name: &str, items: Vec<MenuItem>) -> Self {
            Self {
                name: name.to_string(),
                items,
            }
        }
    }

    impl MenuItemProvider for TestProvider {
        fn provider_name(&self) -> &str {
            &self.name
        }

        fn get_menu_items(&self) -> Vec<MenuItem> {
            self.items.clone()
        }

        fn refresh(&mut self) {}

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    fn make_item(id: &str, label: &str, item_type: MenuItemType) -> MenuItem {
        MenuItem {
            id: id.to_string(),
            label: label.to_string(),
            item_type,
            data: None,
            enabled: true,
            separator_after: false,
            style: None,
            tooltip: None,
            submenu_items: None,
            icon: None,
            section_id: None,
        }
    }

    fn make_config_service() -> ConfigService {
        let dir = tempfile::TempDir::new().unwrap();
        let default_json = include_str!("../../resources/default_settings.json");
        std::fs::write(dir.path().join("settings.json"), default_json).unwrap();
        let svc = ConfigService::load(dir.path(), None).unwrap();
        std::mem::forget(dir);
        svc
    }

    #[test]
    fn test_section_ordering() {
        let mut coordinator = MenuCoordinator::new();

        coordinator.add_provider(Box::new(TestProvider::new(
            "ContextMenuProvider",
            vec![make_item("ctx-1", "Context", MenuItemType::Context)],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "SkillProvider",
            vec![make_item("p-1", "My Prompt", MenuItemType::Skill)],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        let sections: Vec<Option<&str>> = items
            .iter()
            .map(|i| i.section_id.as_deref())
            .collect();

        assert!(sections.contains(&Some("ContextMenuProvider")));
        assert!(sections.contains(&Some("prompts")) || sections.contains(&Some("skills")));
        assert!(sections.contains(&Some("settings")));
    }

    #[test]
    fn test_separator_between_sections() {
        let mut coordinator = MenuCoordinator::new();

        coordinator.add_provider(Box::new(TestProvider::new(
            "ContextMenuProvider",
            vec![make_item("ctx-1", "Context", MenuItemType::Context)],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "SkillProvider",
            vec![make_item("p-1", "Prompt", MenuItemType::Skill)],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        let ctx_items: Vec<&MenuItem> = items
            .iter()
            .filter(|i| i.section_id.as_deref() == Some("ContextMenuProvider"))
            .collect();
        assert!(!ctx_items.is_empty());
        assert!(
            ctx_items.last().unwrap().separator_after,
            "Last item in non-final section should have separator_after"
        );

        let settings_items: Vec<&MenuItem> = items
            .iter()
            .filter(|i| i.section_id.as_deref() == Some("settings"))
            .collect();
        assert!(!settings_items.is_empty());
        assert!(
            !settings_items.last().unwrap().separator_after,
            "Last section should not have separator_after"
        );
    }

    #[test]
    fn test_empty_provider_skipped() {
        let mut coordinator = MenuCoordinator::new();

        coordinator.add_provider(Box::new(TestProvider::new(
            "LastInteractionMenuProvider",
            vec![],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "SkillProvider",
            vec![make_item("p-1", "Prompt", MenuItemType::Skill)],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        let has_last_interaction = items
            .iter()
            .any(|i| i.section_id.as_deref() == Some("LastInteractionMenuProvider"));
        assert!(!has_last_interaction, "Empty provider section should be skipped");
    }

    #[test]
    fn test_skills_virtual_section_excludes_dynamic() {
        let mut coordinator = MenuCoordinator::new();

        coordinator.add_provider(Box::new(TestProvider::new(
            "ContextMenuProvider",
            vec![make_item("ctx-1", "Context", MenuItemType::Context)],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "LastInteractionMenuProvider",
            vec![make_item("li-1", "Last", MenuItemType::LastInteraction)],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "SpeechMenuProvider",
            vec![make_item("sp-1", "Speech", MenuItemType::Speech)],
        )));
        coordinator.add_provider(Box::new(TestProvider::new(
            "SkillProvider",
            vec![make_item("p-1", "My Prompt", MenuItemType::Skill)],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        let skill_section: Vec<&MenuItem> = items
            .iter()
            .filter(|i| matches!(i.section_id.as_deref(), Some("skills" | "prompts")))
            .collect();
        assert_eq!(skill_section.len(), 1);
        assert_eq!(skill_section[0].id, "p-1");
    }

    #[test]
    fn test_settings_virtual_section() {
        let coordinator = MenuCoordinator::new();
        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        let settings_items: Vec<&MenuItem> = items
            .iter()
            .filter(|i| i.section_id.as_deref() == Some("settings"))
            .collect();
        assert_eq!(settings_items.len(), 1);
        assert_eq!(settings_items[0].item_type, MenuItemType::SettingsSection);

        let data = settings_items[0].data.as_ref().unwrap();
        assert!(data.get("model_options").is_some());
        assert!(data.get("current_model").is_some());
    }

    #[test]
    fn test_section_id_set_on_all_items() {
        let mut coordinator = MenuCoordinator::new();
        coordinator.add_provider(Box::new(TestProvider::new(
            "ContextMenuProvider",
            vec![
                make_item("ctx-1", "A", MenuItemType::Context),
                make_item("ctx-2", "B", MenuItemType::Context),
            ],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        for item in &items {
            assert!(
                item.section_id.is_some(),
                "Every item must have a section_id"
            );
        }
    }

    #[test]
    fn test_no_providers_returns_settings_only() {
        let coordinator = MenuCoordinator::new();
        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, MenuItemType::SettingsSection);
    }

    #[test]
    fn test_empty_separator_when_section_skipped() {
        let mut coordinator = MenuCoordinator::new();

        coordinator.add_provider(Box::new(TestProvider::new(
            "LastInteractionMenuProvider",
            vec![],
        )));

        let config = make_config_service();
        let items = coordinator.get_menu_items(&config);

        assert_eq!(items.len(), 1);
        assert!(!items[0].separator_after);
    }
}
