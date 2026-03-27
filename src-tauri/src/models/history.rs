use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryEntryType {
    Speech,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedConversationTurn {
    pub turn_number: u32,
    pub message_text: String,
    #[serde(default)]
    pub message_image_paths: Vec<String>,
    pub output_text: Option<String>,
    #[serde(default)]
    pub is_complete: bool,
    #[serde(default)]
    pub output_versions: Vec<String>,
    #[serde(default)]
    pub current_version_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedConversationNode {
    pub node_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub image_paths: Vec<String>,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistoryData {
    pub context_text: String,
    #[serde(default)]
    pub context_image_paths: Vec<String>,
    #[serde(default)]
    pub turns: Vec<SerializedConversationTurn>,
    pub prompt_id: Option<String>,
    pub prompt_name: Option<String>,
    #[serde(default)]
    pub nodes: Vec<SerializedConversationNode>,
    pub root_node_id: Option<String>,
    #[serde(default)]
    pub current_path: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub input_content: String,
    pub entry_type: HistoryEntryType,
    pub output_content: Option<String>,
    pub prompt_id: Option<String>,
    #[serde(default = "default_true")]
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub is_conversation: bool,
    pub prompt_name: Option<String>,
    pub conversation_data: Option<ConversationHistoryData>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_simple_text_entry_round_trip() {
        let entry = HistoryEntry {
            id: "entry-1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            input_content: "Hello world".into(),
            entry_type: HistoryEntryType::Text,
            output_content: Some("Response text".into()),
            prompt_id: Some("prompt-1".into()),
            success: true,
            error: None,
            is_conversation: false,
            prompt_name: Some("Test Prompt".into()),
            conversation_data: None,
            created_at: Some("2026-01-01T00:00:00Z".into()),
            updated_at: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "entry-1");
        assert_eq!(deserialized.entry_type, HistoryEntryType::Text);
        assert!(deserialized.success);
        assert!(!deserialized.is_conversation);
        assert!(deserialized.conversation_data.is_none());
    }

    #[test]
    fn test_entry_with_conversation_data_round_trip() {
        let entry = HistoryEntry {
            id: "entry-2".into(),
            timestamp: "2026-01-01T12:00:00Z".into(),
            input_content: "Conversation start".into(),
            entry_type: HistoryEntryType::Speech,
            output_content: Some("Final output".into()),
            prompt_id: Some("prompt-2".into()),
            success: true,
            error: None,
            is_conversation: true,
            prompt_name: Some("Chat Prompt".into()),
            conversation_data: Some(ConversationHistoryData {
                context_text: "Some context".into(),
                context_image_paths: vec!["/path/to/image.png".into()],
                turns: vec![SerializedConversationTurn {
                    turn_number: 1,
                    message_text: "Hello".into(),
                    message_image_paths: vec![],
                    output_text: Some("Hi there".into()),
                    is_complete: true,
                    output_versions: vec!["Hi there".into(), "Hello!".into()],
                    current_version_index: 0,
                }],
                prompt_id: Some("prompt-2".into()),
                prompt_name: Some("Chat Prompt".into()),
                nodes: vec![
                    SerializedConversationNode {
                        node_id: "node-root".into(),
                        parent_id: None,
                        role: "user".into(),
                        content: "Hello".into(),
                        image_paths: vec![],
                        timestamp: "2026-01-01T12:00:00Z".into(),
                        children: vec!["node-reply".into()],
                    },
                    SerializedConversationNode {
                        node_id: "node-reply".into(),
                        parent_id: Some("node-root".into()),
                        role: "assistant".into(),
                        content: "Hi there".into(),
                        image_paths: vec![],
                        timestamp: "2026-01-01T12:00:01Z".into(),
                        children: vec![],
                    },
                ],
                root_node_id: Some("node-root".into()),
                current_path: vec!["node-root".into(), "node-reply".into()],
            }),
            created_at: Some("2026-01-01T12:00:00Z".into()),
            updated_at: Some("2026-01-01T12:00:01Z".into()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.entry_type, HistoryEntryType::Speech);
        assert!(deserialized.is_conversation);

        let conv = deserialized.conversation_data.unwrap();
        assert_eq!(conv.turns.len(), 1);
        assert_eq!(conv.nodes.len(), 2);
        assert_eq!(conv.root_node_id, Some("node-root".into()));
        assert_eq!(conv.current_path.len(), 2);
        assert_eq!(conv.context_image_paths.len(), 1);
    }

    #[test]
    fn test_minimal_json_defaults() {
        let json = r#"{
            "id": "entry-3",
            "timestamp": "2026-01-01",
            "input_content": "test",
            "entry_type": "text"
        }"#;

        let entry: HistoryEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.id, "entry-3");
        assert_eq!(entry.entry_type, HistoryEntryType::Text);
        assert!(entry.success);
        assert!(!entry.is_conversation);
        assert!(entry.output_content.is_none());
        assert!(entry.prompt_id.is_none());
        assert!(entry.error.is_none());
        assert!(entry.prompt_name.is_none());
        assert!(entry.conversation_data.is_none());
        assert!(entry.created_at.is_none());
        assert!(entry.updated_at.is_none());
    }
}
