use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::message::{ImageData, NodeUpdate};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryEntryType {
    Speech,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedConversationNode {
    pub node_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub updates: Vec<NodeUpdate>,
    #[serde(default)]
    pub prompt_tokens: Option<usize>,
    #[serde(default)]
    pub completion_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistoryData {
    pub context_text: String,
    #[serde(default)]
    pub nodes: Vec<SerializedConversationNode>,
    pub root_node_id: Option<String>,
    #[serde(default)]
    pub current_path: Vec<String>,
    #[serde(default)]
    pub resolved_environment_section: Option<String>,
    #[serde(default)]
    pub node_images: HashMap<String, Vec<ImageData>>,
    #[serde(default)]
    pub context_images: Vec<ImageData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImagePayload {
    pub node_id: Option<String>,
    pub image_index: u32,
    pub data: String,
    pub media_type: String,
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
    pub skill_id: Option<String>,
    #[serde(default = "default_true")]
    pub success: bool,
    pub error: Option<String>,
    #[serde(default, alias = "is_conversation")]
    pub is_multi_turn: bool,
    pub skill_name: Option<String>,
    pub conversation_data: Option<ConversationHistoryData>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub quick_action: bool,

    #[serde(default)]
    pub title: Option<String>,
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
            skill_id: Some("prompt-1".into()),
            success: true,
            error: None,
            is_multi_turn: false,
            skill_name: Some("Test Prompt".into()),
            conversation_data: None,
            created_at: Some("2026-01-01T00:00:00Z".into()),
            updated_at: None,
            quick_action: false,
            title: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "entry-1");
        assert_eq!(deserialized.entry_type, HistoryEntryType::Text);
        assert!(deserialized.success);
        assert!(!deserialized.is_multi_turn);
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
            skill_id: Some("prompt-2".into()),
            success: true,
            error: None,
            is_multi_turn: true,
            skill_name: Some("Chat Prompt".into()),
            conversation_data: Some(ConversationHistoryData {
                context_text: "Some context".into(),
                nodes: vec![
                    SerializedConversationNode {
                        node_id: "node-root".into(),
                        parent_id: None,
                        role: "user".into(),
                        content: "Hello".into(),
                        timestamp: "2026-01-01T12:00:00Z".into(),
                        children: vec!["node-reply".into()],
                        updates: vec![],
                        prompt_tokens: None,
                        completion_tokens: None,
                    },
                    SerializedConversationNode {
                        node_id: "node-reply".into(),
                        parent_id: Some("node-root".into()),
                        role: "assistant".into(),
                        content: "Hi there".into(),
                        timestamp: "2026-01-01T12:00:01Z".into(),
                        children: vec![],
                        updates: vec![],
                        prompt_tokens: None,
                        completion_tokens: None,
                    },
                ],
                root_node_id: Some("node-root".into()),
                current_path: vec!["node-root".into(), "node-reply".into()],
                resolved_environment_section: None,
                node_images: HashMap::new(),
                context_images: vec![],
            }),
            created_at: Some("2026-01-01T12:00:00Z".into()),
            updated_at: Some("2026-01-01T12:00:01Z".into()),
            quick_action: false,
            title: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.entry_type, HistoryEntryType::Speech);
        assert!(deserialized.is_multi_turn);

        let conv = deserialized.conversation_data.unwrap();
        assert_eq!(conv.nodes.len(), 2);
        assert_eq!(conv.root_node_id, Some("node-root".into()));
        assert_eq!(conv.current_path.len(), 2);
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
        assert!(!entry.is_multi_turn);
        assert!(entry.output_content.is_none());
        assert!(entry.skill_id.is_none());
        assert!(entry.error.is_none());
        assert!(entry.skill_name.is_none());
        assert!(entry.conversation_data.is_none());
        assert!(entry.created_at.is_none());
        assert!(entry.updated_at.is_none());
    }
}
