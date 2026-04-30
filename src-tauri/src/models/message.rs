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
pub struct ToolCallPayload {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallPayload>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    pub data: String,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeUpdate {
    Environment {
        value: String,
    },
    Context {
        content: String,
        reason: String,
        #[serde(default)]
        image_refs: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMessage {
    pub tool_call_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedSkill {
    pub name: String,
    pub body_snapshot: String,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationNodeForExecution {
    pub node_id: String,
    pub role: String,
    pub content: String,
    pub images: Vec<ImageData>,
    pub text_attachments: Vec<String>,
    #[serde(default)]
    pub updates: Vec<NodeUpdate>,
    #[serde(default)]
    pub applied_skills: Vec<AppliedSkill>,
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
            tool_calls: None,
            tool_call_id: None,
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
            tool_calls: None,
            tool_call_id: None,
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
    fn assistant_tool_call_message_serialization() {
        let msg = ProcessedMessage {
            role: "assistant".into(),
            content: MessageContent::Text("".into()),
            tool_calls: Some(vec![ToolCallPayload {
                id: "call_123".into(),
                call_type: "function".into(),
                function: ToolCallFunction {
                    name: "get_weather".into(),
                    arguments: r#"{"location":"NYC"}"#.into(),
                },
            }]),
            tool_call_id: None,
        };
        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            value,
            json!({
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"NYC\"}"
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn tool_result_message_serialization() {
        let msg = ProcessedMessage {
            role: "tool".into(),
            content: MessageContent::Text("72°F, partly cloudy".into()),
            tool_calls: None,
            tool_call_id: Some("call_123".into()),
        };
        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            value,
            json!({
                "role": "tool",
                "content": "72°F, partly cloudy",
                "tool_call_id": "call_123"
            })
        );
    }

    #[test]
    fn regular_message_omits_tool_fields() {
        let msg = ProcessedMessage {
            role: "user".into(),
            content: MessageContent::Text("hello".into()),
            tool_calls: None,
            tool_call_id: None,
        };
        let value = serde_json::to_value(&msg).unwrap();
        assert!(value.get("tool_calls").is_none());
        assert!(value.get("tool_call_id").is_none());
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
