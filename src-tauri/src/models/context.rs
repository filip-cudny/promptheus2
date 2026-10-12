use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "item_type", rename_all = "lowercase")]
pub enum ContextItem {
    Text { number: u32, content: String },
    Image { number: u32, data: String, media_type: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{self, json};

    #[test]
    fn test_text_serialization() {
        let item = ContextItem::Text {
            number: 1,
            content: "hello".into(),
        };
        let value: serde_json::Value = serde_json::to_value(&item).unwrap();
        assert_eq!(value, json!({"item_type": "text", "number": 1, "content": "hello"}));
    }

    #[test]
    fn test_image_serialization() {
        let item = ContextItem::Image {
            number: 2,
            data: "base64data".into(),
            media_type: "image/png".into(),
        };
        let value: serde_json::Value = serde_json::to_value(&item).unwrap();
        assert_eq!(
            value,
            json!({"item_type": "image", "number": 2, "data": "base64data", "media_type": "image/png"})
        );
    }

    #[test]
    fn test_deserialization_text() {
        let json = r#"{"item_type":"text","number":1,"content":"hello"}"#;
        let item: ContextItem = serde_json::from_str(json).unwrap();
        assert_eq!(item, ContextItem::Text { number: 1, content: "hello".into() });
    }

    #[test]
    fn test_deserialization_image() {
        let json = r#"{"item_type":"image","number":2,"data":"base64data","media_type":"image/png"}"#;
        let item: ContextItem = serde_json::from_str(json).unwrap();
        assert_eq!(
            item,
            ContextItem::Image {
                number: 2,
                data: "base64data".into(),
                media_type: "image/png".into()
            }
        );
    }
}
