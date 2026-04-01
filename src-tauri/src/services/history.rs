use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::history::{
    ConversationHistoryData, HistoryEntry, HistoryEntryType, SerializedConversationTurn,
};

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
}

pub struct HistoryService {
    max_entries: usize,
    entries: Vec<HistoryEntry>,
}

impl HistoryService {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Vec::new(),
        }
    }

    pub fn add_entry(
        &mut self,
        input_content: String,
        entry_type: HistoryEntryType,
        output_content: Option<String>,
        prompt_id: Option<String>,
        success: bool,
        error: Option<String>,
        is_multi_turn: bool,
        prompt_name: Option<String>,
        quick_action: bool,
    ) {
        let now = Self::now_timestamp();
        let entry = HistoryEntry {
            id: Self::generate_id(),
            timestamp: now.clone(),
            input_content,
            entry_type,
            output_content,
            prompt_id,
            success,
            error,
            is_multi_turn,
            prompt_name,
            conversation_data: None,
            created_at: Some(now),
            updated_at: None,
            quick_action,
        };
        self.entries.push(entry);
        self.enforce_max_entries();
    }

    pub fn add_conversation_entry(
        &mut self,
        turns: &[SerializedConversationTurn],
        context_text: String,
        context_image_paths: Vec<String>,
        prompt_id: Option<String>,
        prompt_name: Option<String>,
        success: bool,
        error: Option<String>,
        nodes: Vec<crate::models::history::SerializedConversationNode>,
        root_node_id: Option<String>,
        current_path: Vec<String>,
        quick_action: bool,
    ) -> String {
        let now = Self::now_timestamp();
        let id = Self::generate_id();

        let input_summary = Self::build_input_summary(turns);
        let output_summary = Self::build_output_summary(turns);

        let conv_data = ConversationHistoryData {
            context_text,
            context_image_paths,
            turns: turns.to_vec(),
            prompt_id: prompt_id.clone(),
            prompt_name: prompt_name.clone(),
            nodes,
            root_node_id,
            current_path,
        };

        let entry = HistoryEntry {
            id: id.clone(),
            timestamp: now.clone(),
            input_content: input_summary,
            entry_type: HistoryEntryType::Text,
            output_content: Some(output_summary),
            prompt_id,
            success,
            error,
            is_multi_turn: true,
            prompt_name,
            conversation_data: Some(conv_data),
            created_at: Some(now),
            updated_at: None,
            quick_action,
        };
        self.entries.push(entry);
        self.enforce_max_entries();
        id
    }

    pub fn update_conversation_entry(
        &mut self,
        entry_id: &str,
        turns: &[SerializedConversationTurn],
        context_text: String,
        context_image_paths: Vec<String>,
        nodes: Vec<crate::models::history::SerializedConversationNode>,
        root_node_id: Option<String>,
        current_path: Vec<String>,
    ) -> Result<(), HistoryError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.id == entry_id)
            .ok_or_else(|| HistoryError::EntryNotFound(entry_id.to_string()))?;

        let now = Self::now_timestamp();

        if let Some(ref mut conv_data) = entry.conversation_data {
            conv_data.context_text = context_text;
            conv_data.context_image_paths = context_image_paths;
            conv_data.turns = turns.to_vec();
            conv_data.nodes = nodes;
            conv_data.root_node_id = root_node_id;
            conv_data.current_path = current_path;
        }

        entry.input_content = Self::build_input_summary(turns);
        entry.output_content = Some(Self::build_output_summary(turns));
        entry.timestamp = now.clone();
        entry.updated_at = Some(now);

        Ok(())
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| {
            let a_key = a
                .updated_at
                .as_deref()
                .or(a.created_at.as_deref())
                .unwrap_or(&a.timestamp);
            let b_key = b
                .updated_at
                .as_deref()
                .or(b.created_at.as_deref())
                .unwrap_or(&b.timestamp);
            b_key.cmp(a_key)
        });
        entries
    }

    pub fn get_entry_by_id(&self, id: &str) -> Option<HistoryEntry> {
        self.entries.iter().find(|e| e.id == id).cloned()
    }

    pub fn get_last_item_by_type(&self, entry_type: HistoryEntryType) -> Option<HistoryEntry> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.entry_type == entry_type)
            .cloned()
    }

    pub fn get_last_quick_action(&self, entry_type: HistoryEntryType) -> Option<HistoryEntry> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.entry_type == entry_type && e.quick_action)
            .cloned()
    }

    pub fn get_conversation_data(&self, entry_id: &str) -> Option<ConversationHistoryData> {
        self.entries
            .iter()
            .find(|e| e.id == entry_id)
            .and_then(|e| e.conversation_data.clone())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn generate_id() -> String {
        format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        )
    }

    fn now_timestamp() -> String {
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    fn enforce_max_entries(&mut self) {
        if self.entries.len() > self.max_entries {
            self.entries.sort_by(|a, b| {
                let a_key = a
                    .updated_at
                    .as_deref()
                    .or(a.created_at.as_deref())
                    .unwrap_or(&a.timestamp);
                let b_key = b
                    .updated_at
                    .as_deref()
                    .or(b.created_at.as_deref())
                    .unwrap_or(&b.timestamp);
                b_key.cmp(a_key)
            });
            self.entries.truncate(self.max_entries);
        }
    }

    fn build_input_summary(turns: &[SerializedConversationTurn]) -> String {
        if turns.is_empty() {
            return "(no input)".to_string();
        }
        let last_turn = &turns[turns.len() - 1];
        let last_msg = if last_turn.message_text.is_empty() {
            if !last_turn.message_image_paths.is_empty() {
                "(image)".to_string()
            } else {
                "(empty)".to_string()
            }
        } else {
            last_turn.message_text.clone()
        };

        if turns.len() > 1 {
            let truncated: String = last_msg.chars().take(100).collect();
            format!("{}... (+{} more)", truncated, turns.len() - 1)
        } else if last_msg.chars().count() > 200 {
            last_msg.chars().take(200).collect()
        } else {
            last_msg
        }
    }

    fn build_output_summary(turns: &[SerializedConversationTurn]) -> String {
        let complete_turns: Vec<&SerializedConversationTurn> = turns
            .iter()
            .filter(|t| t.is_complete && t.output_text.is_some())
            .collect();

        if complete_turns.is_empty() {
            return "(no output yet)".to_string();
        }

        let last_output = complete_turns
            .last()
            .unwrap()
            .output_text
            .as_deref()
            .unwrap_or("");

        if complete_turns.len() > 1 {
            let truncated: String = last_output.chars().take(100).collect();
            format!("{}... (+{} more)", truncated, complete_turns.len() - 1)
        } else if last_output.chars().count() > 200 {
            last_output.chars().take(200).collect()
        } else {
            last_output.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::SerializedConversationNode;

    fn make_turn(
        number: u32,
        text: &str,
        output: Option<&str>,
        complete: bool,
    ) -> SerializedConversationTurn {
        SerializedConversationTurn {
            turn_number: number,
            message_text: text.to_string(),
            message_image_paths: vec![],
            output_text: output.map(|s| s.to_string()),
            is_complete: complete,
            output_versions: vec![],
            current_version_index: 0,
        }
    }

    #[test]
    fn add_entries_and_get_history_sorted() {
        let mut svc = HistoryService::new(10);

        svc.add_entry(
            "first".into(),
            HistoryEntryType::Text,
            Some("out1".into()),
            None,
            true,
            None,
            false,
            None,
            false,
        );
        svc.entries.last_mut().unwrap().created_at = Some("2026-01-01 00:00:01".into());

        svc.add_entry(
            "second".into(),
            HistoryEntryType::Text,
            Some("out2".into()),
            None,
            true,
            None,
            false,
            None,
            false,
        );
        svc.entries.last_mut().unwrap().created_at = Some("2026-01-01 00:00:02".into());

        let history = svc.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].input_content, "second");
        assert_eq!(history[1].input_content, "first");
    }

    #[test]
    fn max_entries_enforcement() {
        let mut svc = HistoryService::new(3);

        for i in 0..5 {
            let ts = format!("2026-01-01 00:00:{:02}", i);
            svc.entries.push(HistoryEntry {
                id: format!("id-{}", i),
                timestamp: ts.clone(),
                input_content: format!("entry-{}", i),
                entry_type: HistoryEntryType::Text,
                output_content: None,
                prompt_id: None,
                success: true,
                error: None,
                is_multi_turn: false,
                prompt_name: None,
                conversation_data: None,
                created_at: Some(ts),
                updated_at: None,
                quick_action: false,
            });
            svc.enforce_max_entries();
        }

        assert_eq!(svc.entry_count(), 3);
        let history = svc.get_history();
        assert_eq!(history[0].input_content, "entry-4");
        assert_eq!(history[1].input_content, "entry-3");
        assert_eq!(history[2].input_content, "entry-2");
    }

    #[test]
    fn get_entry_by_id_found_and_not_found() {
        let mut svc = HistoryService::new(10);
        svc.add_entry(
            "test".into(),
            HistoryEntryType::Text,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );

        let entry = svc.get_history().into_iter().next().unwrap();
        assert!(svc.get_entry_by_id(&entry.id).is_some());
        assert!(svc.get_entry_by_id("nonexistent").is_none());
    }

    #[test]
    fn get_last_item_by_type_with_mixed_types() {
        let mut svc = HistoryService::new(10);

        svc.add_entry(
            "text1".into(),
            HistoryEntryType::Text,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );
        svc.add_entry(
            "speech1".into(),
            HistoryEntryType::Speech,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );
        svc.add_entry(
            "text2".into(),
            HistoryEntryType::Text,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );

        let last_text = svc.get_last_item_by_type(HistoryEntryType::Text).unwrap();
        assert_eq!(last_text.input_content, "text2");

        let last_speech = svc
            .get_last_item_by_type(HistoryEntryType::Speech)
            .unwrap();
        assert_eq!(last_speech.input_content, "speech1");
    }

    #[test]
    fn update_conversation_entry_success_and_failure() {
        let mut svc = HistoryService::new(10);

        let turns = vec![make_turn(1, "hello", Some("hi"), true)];
        let id = svc.add_conversation_entry(
            &turns,
            "context".into(),
            vec![],
            Some("p1".into()),
            Some("Prompt".into()),
            true,
            None,
            vec![],
            None,
            vec![],
            false,
        );

        let updated_turns = vec![
            make_turn(1, "hello", Some("hi"), true),
            make_turn(2, "how are you?", Some("good"), true),
        ];
        let result = svc.update_conversation_entry(
            &id,
            &updated_turns,
            "new context".into(),
            vec![],
            vec![],
            None,
            vec![],
        );
        assert!(result.is_ok());

        let entry = svc.get_entry_by_id(&id).unwrap();
        assert!(entry.updated_at.is_some());
        let conv = entry.conversation_data.unwrap();
        assert_eq!(conv.turns.len(), 2);
        assert_eq!(conv.context_text, "new context");

        let fail_result = svc.update_conversation_entry(
            "nonexistent",
            &updated_turns,
            "ctx".into(),
            vec![],
            vec![],
            None,
            vec![],
        );
        assert!(fail_result.is_err());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut svc = HistoryService::new(10);
        svc.add_entry(
            "test".into(),
            HistoryEntryType::Text,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );
        assert_eq!(svc.entry_count(), 1);

        svc.clear();
        assert_eq!(svc.entry_count(), 0);
        assert!(svc.get_history().is_empty());
    }

    #[test]
    fn conversation_entry_with_summary_building() {
        let mut svc = HistoryService::new(10);

        let turns = vec![
            make_turn(1, "first message", Some("first reply"), true),
            make_turn(2, "second message", Some("second reply"), true),
        ];
        let id = svc.add_conversation_entry(
            &turns,
            "ctx".into(),
            vec![],
            None,
            None,
            true,
            None,
            vec![],
            None,
            vec![],
            false,
        );

        let entry = svc.get_entry_by_id(&id).unwrap();
        assert!(entry.input_content.contains("second message"));
        assert!(entry.input_content.contains("+1 more"));
        assert!(entry
            .output_content
            .as_ref()
            .unwrap()
            .contains("second reply"));
        assert!(entry
            .output_content
            .as_ref()
            .unwrap()
            .contains("+1 more"));
    }

    #[test]
    fn get_conversation_data_returns_data() {
        let mut svc = HistoryService::new(10);

        let nodes = vec![SerializedConversationNode {
            node_id: "root".into(),
            parent_id: None,
            role: "user".into(),
            content: "hello".into(),
            image_paths: vec![],
            timestamp: "2026-01-01".into(),
            children: vec![],
        }];

        let id = svc.add_conversation_entry(
            &[make_turn(1, "hello", Some("hi"), true)],
            "ctx".into(),
            vec!["img.png".into()],
            Some("p1".into()),
            Some("Test".into()),
            true,
            None,
            nodes,
            Some("root".into()),
            vec!["root".into()],
            false,
        );

        let conv = svc.get_conversation_data(&id).unwrap();
        assert_eq!(conv.context_text, "ctx");
        assert_eq!(conv.context_image_paths, vec!["img.png"]);
        assert_eq!(conv.nodes.len(), 1);
        assert_eq!(conv.root_node_id, Some("root".into()));
        assert_eq!(conv.current_path, vec!["root"]);

        assert!(svc.get_conversation_data("nonexistent").is_none());
    }

    #[test]
    fn build_input_summary_edge_cases() {
        assert_eq!(HistoryService::build_input_summary(&[]), "(no input)");

        let image_turn = SerializedConversationTurn {
            turn_number: 1,
            message_text: "".into(),
            message_image_paths: vec!["img.png".into()],
            output_text: None,
            is_complete: false,
            output_versions: vec![],
            current_version_index: 0,
        };
        assert_eq!(
            HistoryService::build_input_summary(&[image_turn]),
            "(image)"
        );

        let empty_turn = make_turn(1, "", None, false);
        assert_eq!(
            HistoryService::build_input_summary(&[empty_turn]),
            "(empty)"
        );

        let long_text: String = "a".repeat(250);
        let long_turn = make_turn(1, &long_text, None, false);
        let summary = HistoryService::build_input_summary(&[long_turn]);
        assert_eq!(summary.chars().count(), 200);
    }

    #[test]
    fn build_output_summary_edge_cases() {
        assert_eq!(
            HistoryService::build_output_summary(&[]),
            "(no output yet)"
        );

        let incomplete = make_turn(1, "msg", None, false);
        assert_eq!(
            HistoryService::build_output_summary(&[incomplete]),
            "(no output yet)"
        );

        let long_output: String = "b".repeat(250);
        let turn = make_turn(1, "msg", Some(&long_output), true);
        let summary = HistoryService::build_output_summary(&[turn]);
        assert_eq!(summary.chars().count(), 200);
    }
}
