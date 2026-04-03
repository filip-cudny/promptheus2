use crate::models::menu::{MenuItem, MenuItemType};
use crate::models::skill::SkillSummary;
use crate::traits::MenuItemProvider;

pub struct SkillMenuProvider {
    skills: Vec<SkillSummary>,
}

impl SkillMenuProvider {
    pub fn new(skills: Vec<SkillSummary>) -> Self {
        Self { skills }
    }

    pub fn update_skills(&mut self, skills: Vec<SkillSummary>) {
        self.skills = skills;
    }
}

impl MenuItemProvider for SkillMenuProvider {
    fn provider_name(&self) -> &str {
        "SkillProvider"
    }

    fn get_menu_items(&self) -> Vec<MenuItem> {
        self.skills
            .iter()
            .map(|skill| MenuItem {
                id: skill.name.clone(),
                label: skill.display_name.clone(),
                item_type: MenuItemType::Skill,
                data: Some(serde_json::json!({
                    "skill_id": skill.name,
                    "skill_name": skill.display_name,
                })),
                enabled: true,
                separator_after: false,
                style: None,
                tooltip: skill.description.clone(),
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

    fn make_skill(name: &str, display_name: &str) -> SkillSummary {
        SkillSummary {
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: None,
        }
    }

    #[test]
    fn get_menu_items_maps_skills() {
        let provider = SkillMenuProvider::new(vec![
            make_skill("summarize", "Summarize"),
            make_skill("translate", "Translate"),
        ]);

        let items = provider.get_menu_items();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "summarize");
        assert_eq!(items[0].label, "Summarize");
        assert_eq!(items[0].item_type, MenuItemType::Skill);
        assert!(items[0].enabled);

        let data = items[0].data.as_ref().unwrap();
        assert_eq!(data["skill_id"], "summarize");
        assert_eq!(data["skill_name"], "Summarize");
    }

    #[test]
    fn empty_skills() {
        let provider = SkillMenuProvider::new(Vec::new());
        assert!(provider.get_menu_items().is_empty());
    }

    #[test]
    fn update_skills() {
        let mut provider = SkillMenuProvider::new(vec![make_skill("p1", "Old")]);
        assert_eq!(provider.get_menu_items().len(), 1);

        provider.update_skills(vec![
            make_skill("p2", "New1"),
            make_skill("p3", "New2"),
        ]);
        let items = provider.get_menu_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "p2");
    }

    #[test]
    fn provider_name() {
        let provider = SkillMenuProvider::new(Vec::new());
        assert_eq!(provider.provider_name(), "SkillProvider");
    }
}
