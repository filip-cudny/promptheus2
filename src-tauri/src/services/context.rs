use crate::models::context::ContextItem;
use crate::models::menu::{MenuItem, MenuItemType};
use crate::traits::MenuItemProvider;

pub struct ContextMenuProvider {
    items: Vec<ContextItem>,
}

impl ContextMenuProvider {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn update_items(&mut self, items: Vec<ContextItem>) {
        self.items = items;
    }
}

impl MenuItemProvider for ContextMenuProvider {
    fn provider_name(&self) -> &str {
        "ContextMenuProvider"
    }

    fn get_menu_items(&self) -> Vec<MenuItem> {
        vec![MenuItem {
            id: "environment_section".to_string(),
            label: "Context".to_string(),
            item_type: MenuItemType::Context,
            data: Some(serde_json::json!({ "items": self.items })),
            enabled: true,
            separator_after: false,
            tooltip: None,
            icon: None,
            section_id: None,
        }]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn pasted_text_block(number: impl std::fmt::Display, content: &str) -> String {
    format!("<pasted-text name=\"Text #{number}\">\n{content}\n</pasted-text>")
}

pub struct ContextManagerService {
    items: Vec<ContextItem>,
    next_text_number: u32,
    next_image_number: u32,
}

impl ContextManagerService {
    pub fn new() -> Self {
        Self { items: Vec::new(), next_text_number: 1, next_image_number: 1 }
    }

    pub fn set_context(&mut self, value: String) {
        self.clear();
        self.append_context(value);
    }

    pub fn append_context(&mut self, value: String) {
        let number = self.next_text_number;
        self.next_text_number += 1;
        self.items.push(ContextItem::Text { number, content: value });
    }

    pub fn get_context(&self) -> Option<String> {
        let text_items: Vec<&str> = self
            .items
            .iter()
            .filter_map(|item| match item {
                ContextItem::Text { content, .. } if !content.is_empty() => Some(content.as_str()),
                _ => None,
            })
            .collect();

        if text_items.is_empty() {
            None
        } else {
            Some(text_items.join("\n"))
        }
    }

    pub fn labelled_context(&self) -> Option<String> {
        let blocks: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| match item {
                ContextItem::Text { number, content } => Some(pasted_text_block(*number, content)),
                _ => None,
            })
            .collect();

        if blocks.is_empty() {
            None
        } else {
            Some(blocks.join("\n\n"))
        }
    }

    pub fn has_context(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn set_context_image(&mut self, data: String, media_type: String) {
        self.clear();
        self.append_context_image(data, media_type);
    }

    pub fn append_context_image(&mut self, data: String, media_type: String) {
        let number = self.next_image_number;
        self.next_image_number += 1;
        self.items.push(ContextItem::Image { number, data, media_type });
    }

    pub fn has_images(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, ContextItem::Image { .. }))
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.next_text_number = 1;
        self.next_image_number = 1;
    }

    pub fn get_items(&self) -> Vec<ContextItem> {
        self.items.clone()
    }

    pub fn remove_item(&mut self, index: usize) -> bool {
        if index < self.items.len() {
            self.items.remove(index);
            true
        } else {
            false
        }
    }

}

#[cfg(test)]
impl ContextManagerService {
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> ContextManagerService {
        ContextManagerService::new()
    }

    #[test]
    fn set_context_replaces_all_items() {
        let mut svc = service();
        svc.append_context("first".into());
        svc.append_context_image("img".into(), "image/png".into());
        svc.set_context("replaced".into());

        assert_eq!(svc.item_count(), 1);
        assert_eq!(svc.get_context(), Some("replaced".into()));
        assert!(!svc.has_images());
    }

    #[test]
    fn append_context_preserves_order() {
        let mut svc = service();
        svc.append_context("first".into());
        svc.append_context("second".into());
        svc.append_context("third".into());

        assert_eq!(svc.get_context(), Some("first\nsecond\nthird".into()));
    }

    #[test]
    fn get_context_concatenates_with_newline_and_returns_none_when_empty() {
        let svc = service();
        assert_eq!(svc.get_context(), None);

        let mut svc = service();
        svc.append_context("a".into());
        svc.append_context("b".into());
        assert_eq!(svc.get_context(), Some("a\nb".into()));
    }

    #[test]
    fn get_context_skips_empty_content_text_items() {
        let mut svc = service();
        svc.append_context("".into());
        svc.append_context("visible".into());
        svc.append_context("".into());

        assert_eq!(svc.get_context(), Some("visible".into()));
    }

    #[test]
    fn get_context_returns_none_when_all_text_empty() {
        let mut svc = service();
        svc.append_context("".into());
        svc.append_context("".into());

        assert_eq!(svc.get_context(), None);
    }

    #[test]
    fn has_context_returns_true_when_only_images() {
        let mut svc = service();
        svc.append_context_image("img".into(), "image/png".into());

        assert!(svc.has_context());
        assert!(svc.has_images());
    }

    #[test]
    fn has_images_returns_false_when_only_text() {
        let mut svc = service();
        svc.append_context("text".into());

        assert!(!svc.has_images());
        assert!(svc.has_context());
    }

    #[test]
    fn mixed_items_preserve_insertion_order() {
        let mut svc = service();
        svc.append_context("text1".into());
        svc.append_context_image("img1".into(), "image/png".into());
        svc.append_context("text2".into());
        svc.append_context_image("img2".into(), "image/jpeg".into());

        let items = svc.get_items();
        assert_eq!(items.len(), 4);
        assert!(matches!(&items[0], ContextItem::Text { content, .. } if content == "text1"));
        assert!(matches!(&items[1], ContextItem::Image { data, .. } if data == "img1"));
        assert!(matches!(&items[2], ContextItem::Text { content, .. } if content == "text2"));
        assert!(matches!(&items[3], ContextItem::Image { data, .. } if data == "img2"));
    }

    #[test]
    fn set_context_image_clears_everything() {
        let mut svc = service();
        svc.append_context("text".into());
        svc.append_context_image("img1".into(), "image/png".into());
        svc.set_context_image("new_img".into(), "image/jpeg".into());

        assert_eq!(svc.item_count(), 1);
        assert!(svc.has_context());
        assert!(svc.has_images());
        assert!(
            matches!(&svc.get_items()[0], ContextItem::Image { data, media_type, .. } if data == "new_img" && media_type == "image/jpeg")
        );
    }

    #[test]
    fn remove_item_valid_and_invalid_indices() {
        let mut svc = service();
        svc.append_context("a".into());
        svc.append_context("b".into());
        svc.append_context("c".into());

        assert!(!svc.remove_item(5));
        assert_eq!(svc.item_count(), 3);

        assert!(svc.remove_item(1));
        assert_eq!(svc.item_count(), 2);
        assert_eq!(svc.get_context(), Some("a\nc".into()));
    }

    #[test]
    fn clear_removes_everything() {
        let mut svc = service();
        svc.append_context("text".into());
        svc.append_context_image("img".into(), "image/png".into());
        svc.clear();

        assert_eq!(svc.item_count(), 0);
        assert!(!svc.has_context());
        assert!(!svc.has_images());
    }

    #[test]
    fn item_count_follows_appends() {
        let mut svc = service();
        assert_eq!(svc.item_count(), 0);

        svc.append_context("text".into());
        assert_eq!(svc.item_count(), 1);

        svc.append_context_image("img".into(), "image/png".into());
        assert_eq!(svc.item_count(), 2);
    }

    fn numbers(svc: &ContextManagerService) -> Vec<(&'static str, u32)> {
        svc.get_items()
            .iter()
            .map(|item| match item {
                ContextItem::Text { number, .. } => ("text", *number),
                ContextItem::Image { number, .. } => ("image", *number),
            })
            .collect()
    }

    #[test]
    fn numbers_are_per_kind_and_survive_removal() {
        let mut svc = service();
        svc.append_context("a".into());
        svc.append_context_image("i".into(), "image/png".into());
        svc.append_context("b".into());
        assert_eq!(numbers(&svc), [("text", 1), ("image", 1), ("text", 2)]);

        svc.remove_item(0);
        assert_eq!(numbers(&svc), [("image", 1), ("text", 2)]);

        svc.append_context("c".into());
        assert_eq!(numbers(&svc), [("image", 1), ("text", 2), ("text", 3)]);
    }

    #[test]
    fn clear_resets_numbers() {
        let mut svc = service();
        svc.append_context("a".into());
        svc.append_context("b".into());
        svc.clear();
        svc.append_context("c".into());
        assert_eq!(numbers(&svc), [("text", 1)]);
    }

    #[test]
    fn set_context_numbers_the_item_one() {
        let mut svc = service();
        svc.append_context("a".into());
        svc.append_context("b".into());
        svc.set_context("c".into());
        assert_eq!(numbers(&svc), [("text", 1)]);
    }

    #[test]
    fn set_context_image_resets_text_numbers() {
        let mut svc = service();
        svc.append_context("a".into());
        svc.set_context_image("i".into(), "image/png".into());
        assert_eq!(numbers(&svc), [("image", 1)]);

        svc.append_context("b".into());
        assert_eq!(numbers(&svc), [("image", 1), ("text", 1)]);
    }

    #[test]
    fn labelled_context_names_each_text_by_its_number() {
        let mut svc = service();
        svc.append_context("one".into());
        svc.append_context("two".into());
        svc.append_context("".into());
        svc.remove_item(1);

        assert_eq!(
            svc.labelled_context(),
            Some(
                "<pasted-text name=\"Text #1\">\none\n</pasted-text>\n\n<pasted-text name=\"Text #3\">\n\n</pasted-text>"
                    .into()
            )
        );
    }

    #[test]
    fn labelled_context_is_none_without_text() {
        let mut svc = service();
        assert_eq!(svc.labelled_context(), None);

        svc.append_context_image("img".into(), "image/png".into());
        assert_eq!(svc.labelled_context(), None);
    }

    #[test]
    fn provider_name_matches_dynamic_providers() {
        let provider = ContextMenuProvider::new();
        assert_eq!(provider.provider_name(), "ContextMenuProvider");
    }

    #[test]
    fn provider_returns_environment_section_when_no_items() {
        let provider = ContextMenuProvider::new();
        let items = provider.get_menu_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, MenuItemType::Context);

        let data = items[0].data.as_ref().unwrap();
        let context_items = data["items"].as_array().unwrap();
        assert!(context_items.is_empty());
    }

    #[test]
    fn provider_returns_menu_item_with_serialized_context() {
        use crate::models::menu::MenuItemType;
        use crate::traits::MenuItemProvider;

        let mut provider = ContextMenuProvider::new();
        provider.update_items(vec![
            ContextItem::Text {
                number: 1,
                content: "hello".into(),
            },
            ContextItem::Image {
                number: 1,
                data: "img_data".into(),
                media_type: "image/png".into(),
            },
        ]);

        let items = provider.get_menu_items();
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert_eq!(item.id, "environment_section");
        assert_eq!(item.label, "Context");
        assert_eq!(item.item_type, MenuItemType::Context);
        assert!(item.enabled);

        let data = item.data.as_ref().unwrap();
        let context_items = data["items"].as_array().unwrap();
        assert_eq!(context_items.len(), 2);
    }
}
