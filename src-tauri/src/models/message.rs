use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrlData {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrlData },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedMessage {
    pub role: String,
    pub content: MessageContent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{self, json};

    #[test]
    fn text_only_message_serialization() {
        let msg = ProcessedMessage {
            role: "user".into(),
            content: MessageContent::Text("hello".into()),
        };
        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(value, json!({"role": "user", "content": "hello"}));
    }

    #[test]
    fn multi_part_message_serialization() {
        let msg = ProcessedMessage {
            role: "user".into(),
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "hello".into(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrlData {
                        url: "data:image/png;base64,abc".into(),
                    },
                },
            ]),
        };
        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            value,
            json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
                ]
            })
        );
    }

    #[test]
    fn text_content_serializes_as_plain_string() {
        let content = MessageContent::Text("hello".into());
        let value = serde_json::to_value(&content).unwrap();
        assert_eq!(value, json!("hello"));
    }

    #[test]
    fn parts_content_serializes_as_array() {
        let content = MessageContent::Parts(vec![ContentPart::Text {
            text: "hello".into(),
        }]);
        let value = serde_json::to_value(&content).unwrap();
        assert_eq!(value, json!([{"type": "text", "text": "hello"}]));
    }
}
