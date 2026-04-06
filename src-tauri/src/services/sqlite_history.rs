use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::models::history::{
    ConversationHistoryData, HistoryEntry, HistoryEntryType, ImagePayload,
    SerializedConversationNode,
};
use crate::models::message::ImageData;
use crate::services::database::Database;

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TreeJson {
    nodes: Vec<SerializedConversationNode>,
    root_node_id: Option<String>,
    current_path: Vec<String>,
    resolved_environment_section: Option<String>,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    reasoning_effort: Option<String>,
}

pub struct SqliteHistoryService {
    db: Database,
    max_entries: usize,
}

impl SqliteHistoryService {
    pub fn new(db: Database, max_entries: usize) -> Self {
        Self { db, max_entries }
    }

    pub fn add_entry(
        &self,
        input_content: String,
        entry_type: HistoryEntryType,
        output_content: Option<String>,
        skill_id: Option<String>,
        success: bool,
        error: Option<String>,
        is_multi_turn: bool,
        skill_name: Option<String>,
        quick_action: bool,
    ) {
        let id = Self::generate_id();
        let now = Self::now_timestamp();
        let entry_type_str = match entry_type {
            HistoryEntryType::Text => "text",
            HistoryEntryType::Speech => "speech",
        };

        let result = self.db.conn().execute(
            "INSERT INTO conversations (id, entry_type, input_content, output_content, skill_id, skill_name, success, error, is_multi_turn, quick_action, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id,
                entry_type_str,
                input_content,
                output_content,
                skill_id,
                skill_name,
                success,
                error,
                is_multi_turn,
                quick_action,
                now,
            ],
        );

        if let Err(e) = result {
            log::error!("failed to add history entry: {}", e);
            return;
        }

        self.enforce_max_entries();
    }

    pub fn add_conversation_entry(
        &self,
        context_text: String,
        skill_id: Option<String>,
        skill_name: Option<String>,
        success: bool,
        error: Option<String>,
        nodes: Vec<SerializedConversationNode>,
        root_node_id: Option<String>,
        current_path: Vec<String>,
        quick_action: bool,
        resolved_environment_section: Option<String>,
        images: Vec<ImagePayload>,
        model_id: Option<String>,
        reasoning_effort: Option<String>,
    ) -> String {
        let id = Self::generate_id();
        let now = Self::now_timestamp();

        let input_summary = Self::build_input_summary(&nodes);
        let output_summary = Self::build_output_summary(&nodes);

        let tree_json = serde_json::to_string(&TreeJson {
            nodes,
            root_node_id: root_node_id.clone(),
            current_path: current_path.clone(),
            resolved_environment_section,
            model_id,
            reasoning_effort,
        })
        .unwrap_or_default();

        let conn = self.db.conn();
        let tx = conn.unchecked_transaction().unwrap();

        tx.execute(
            "INSERT INTO conversations (id, entry_type, input_content, output_content, skill_id, skill_name, success, error, is_multi_turn, quick_action, context_text, tree_json, created_at)
             VALUES (?1, 'text', ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id,
                input_summary,
                output_summary,
                skill_id,
                skill_name,
                success,
                error,
                quick_action,
                context_text,
                tree_json,
                now,
            ],
        )
        .unwrap();

        self.insert_images(&tx, &id, &images);

        tx.commit().unwrap();
        self.enforce_max_entries();
        id
    }

    pub fn update_conversation_entry(
        &self,
        entry_id: &str,
        context_text: String,
        nodes: Vec<SerializedConversationNode>,
        root_node_id: Option<String>,
        current_path: Vec<String>,
        images: Vec<ImagePayload>,
        model_id: Option<String>,
        reasoning_effort: Option<String>,
    ) -> Result<(), HistoryError> {
        let now = Self::now_timestamp();

        let input_summary = Self::build_input_summary(&nodes);
        let output_summary = Self::build_output_summary(&nodes);

        let tree_json = serde_json::to_string(&TreeJson {
            nodes,
            root_node_id,
            current_path,
            resolved_environment_section: None,
            model_id,
            reasoning_effort,
        })
        .unwrap_or_default();

        let conn = self.db.conn();
        let tx = conn.unchecked_transaction()?;

        let updated = tx.execute(
            "UPDATE conversations SET input_content = ?1, output_content = ?2, context_text = ?3, tree_json = ?4, updated_at = ?5 WHERE id = ?6",
            rusqlite::params![input_summary, output_summary, context_text, tree_json, now, entry_id],
        )?;

        if updated == 0 {
            return Err(HistoryError::EntryNotFound(entry_id.to_string()));
        }

        tx.execute(
            "DELETE FROM conversation_images WHERE conversation_id = ?1",
            [entry_id],
        )?;

        self.insert_images(&tx, entry_id, &images);

        tx.commit()?;
        Ok(())
    }

    pub fn get_conversations(&self, offset: u32, limit: u32) -> Vec<HistoryEntry> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, skill_id, skill_name, entry_type, input_content, output_content,
                        success, error, is_multi_turn, quick_action, created_at, updated_at
                 FROM conversations
                 WHERE quick_action = 0
                 ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .unwrap();

        stmt.query_map(rusqlite::params![limit, offset], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                skill_id: row.get(2)?,
                skill_name: row.get(3)?,
                entry_type: parse_entry_type(row.get::<_, String>(4)?.as_str()),
                input_content: row.get(5)?,
                output_content: row.get(6)?,
                success: row.get(7)?,
                error: row.get(8)?,
                is_multi_turn: row.get(9)?,
                quick_action: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                timestamp: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                conversation_data: None,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, skill_id, skill_name, entry_type, input_content, output_content,
                        success, error, is_multi_turn, quick_action, created_at, updated_at
                 FROM conversations
                 ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC",
            )
            .unwrap();

        stmt.query_map([], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                skill_id: row.get(2)?,
                skill_name: row.get(3)?,
                entry_type: parse_entry_type(row.get::<_, String>(4)?.as_str()),
                input_content: row.get(5)?,
                output_content: row.get(6)?,
                success: row.get(7)?,
                error: row.get(8)?,
                is_multi_turn: row.get(9)?,
                quick_action: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                timestamp: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                conversation_data: None,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn get_entry_by_id(&self, id: &str) -> Option<HistoryEntry> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, skill_id, skill_name, entry_type, input_content, output_content,
                        success, error, is_multi_turn, quick_action, context_text, tree_json,
                        created_at, updated_at
                 FROM conversations WHERE id = ?1",
            )
            .ok()?;

        let entry = stmt
            .query_row([id], |row| {
                let context_text: String = row.get::<_, Option<String>>(11)?.unwrap_or_default();
                let tree_json_str: Option<String> = row.get(12)?;

                let conversation_data = tree_json_str
                    .and_then(|json| serde_json::from_str::<TreeJson>(&json).ok())
                    .map(|tree| ConversationHistoryData {
                        context_text: context_text.clone(),
                        nodes: tree.nodes,
                        root_node_id: tree.root_node_id,
                        current_path: tree.current_path,
                        resolved_environment_section: tree.resolved_environment_section,
                        node_images: HashMap::new(),
                        context_images: vec![],
                        model_id: tree.model_id,
                        reasoning_effort: tree.reasoning_effort,
                    });

                Ok((
                    HistoryEntry {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        skill_id: row.get(2)?,
                        skill_name: row.get(3)?,
                        entry_type: parse_entry_type(row.get::<_, String>(4)?.as_str()),
                        input_content: row.get(5)?,
                        output_content: row.get(6)?,
                        success: row.get(7)?,
                        error: row.get(8)?,
                        is_multi_turn: row.get(9)?,
                        quick_action: row.get(10)?,
                        created_at: row.get(13)?,
                        updated_at: row.get(14)?,
                        timestamp: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                        conversation_data: None,
                    },
                    conversation_data,
                ))
            })
            .ok()?;

        let (mut history_entry, conversation_data) = entry;

        if let Some(mut conv_data) = conversation_data {
            let mut img_stmt = conn
                .prepare(
                    "SELECT node_id, data, mime_type FROM conversation_images
                     WHERE conversation_id = ?1 ORDER BY image_index",
                )
                .ok()?;

            let images: Vec<(Option<String>, Vec<u8>, String)> = img_stmt
                .query_map([id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .ok()?
                .filter_map(|r| r.ok())
                .collect();

            for (node_id, data, mime_type) in images {
                let image_data = ImageData {
                    data: BASE64.encode(&data),
                    media_type: mime_type,
                };
                match node_id {
                    Some(nid) => conv_data
                        .node_images
                        .entry(nid)
                        .or_default()
                        .push(image_data),
                    None => conv_data.context_images.push(image_data),
                }
            }

            history_entry.conversation_data = Some(conv_data);
        }

        Some(history_entry)
    }

    pub fn get_last_item_by_type(&self, entry_type: HistoryEntryType) -> Option<HistoryEntry> {
        let type_str = match entry_type {
            HistoryEntryType::Text => "text",
            HistoryEntryType::Speech => "speech",
        };
        self.query_single_entry(
            "SELECT id, title, skill_id, skill_name, entry_type, input_content, output_content,
                    success, error, is_multi_turn, quick_action, created_at, updated_at
             FROM conversations WHERE entry_type = ?1
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC LIMIT 1",
            [type_str],
        )
    }

    pub fn get_last_quick_action(&self, entry_type: HistoryEntryType) -> Option<HistoryEntry> {
        let type_str = match entry_type {
            HistoryEntryType::Text => "text",
            HistoryEntryType::Speech => "speech",
        };
        self.query_single_entry(
            "SELECT id, title, skill_id, skill_name, entry_type, input_content, output_content,
                    success, error, is_multi_turn, quick_action, created_at, updated_at
             FROM conversations WHERE entry_type = ?1 AND quick_action = 1
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC LIMIT 1",
            [type_str],
        )
    }

    pub fn get_conversation_data(&self, entry_id: &str) -> Option<ConversationHistoryData> {
        self.get_entry_by_id(entry_id)
            .and_then(|e| e.conversation_data)
    }

    pub fn update_entry_title(
        &self,
        entry_id: &str,
        title: String,
    ) -> Result<(), HistoryError> {
        let updated = self.db.conn().execute(
            "UPDATE conversations SET title = ?1 WHERE id = ?2",
            rusqlite::params![title, entry_id],
        )?;
        if updated == 0 {
            return Err(HistoryError::EntryNotFound(entry_id.to_string()));
        }
        Ok(())
    }

    pub fn clear(&self) {
        let _ = self
            .db
            .conn()
            .execute("DELETE FROM conversations", []);
    }

    pub fn entry_count(&self) -> usize {
        self.db
            .conn()
            .query_row("SELECT COUNT(*) FROM conversations", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as usize
    }

    fn insert_images(
        &self,
        tx: &rusqlite::Transaction,
        conversation_id: &str,
        images: &[ImagePayload],
    ) {
        for img in images {
            let blob = BASE64.decode(&img.data).unwrap_or_default();
            if blob.is_empty() {
                continue;
            }
            let img_id = Self::generate_id();
            let _ = tx.execute(
                "INSERT INTO conversation_images (id, conversation_id, node_id, image_index, data, mime_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    img_id,
                    conversation_id,
                    img.node_id,
                    img.image_index,
                    blob,
                    img.media_type,
                ],
            );
        }
    }

    fn query_single_entry<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Option<HistoryEntry> {
        self.db
            .conn()
            .query_row(sql, params, |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    skill_id: row.get(2)?,
                    skill_name: row.get(3)?,
                    entry_type: parse_entry_type(row.get::<_, String>(4)?.as_str()),
                    input_content: row.get(5)?,
                    output_content: row.get(6)?,
                    success: row.get(7)?,
                    error: row.get(8)?,
                    is_multi_turn: row.get(9)?,
                    quick_action: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    timestamp: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                    conversation_data: None,
                })
            })
            .ok()
    }

    fn enforce_max_entries(&self) {
        let _ = self.db.conn().execute(
            "DELETE FROM conversations WHERE id NOT IN (
                SELECT id FROM conversations ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC LIMIT ?1
            )",
            [self.max_entries as i64],
        );
    }

    fn generate_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn now_timestamp() -> String {
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    fn build_input_summary(nodes: &[SerializedConversationNode]) -> String {
        let last_user = nodes.iter().rev().find(|n| n.role == "user");
        match last_user {
            None => "(no input)".to_string(),
            Some(node) => {
                let text = &node.content;
                if text.is_empty() {
                    return "(empty)".to_string();
                }
                let user_count = nodes.iter().filter(|n| n.role == "user").count();
                if user_count > 1 {
                    let truncated: String = text.chars().take(100).collect();
                    format!("{}... (+{} more)", truncated, user_count - 1)
                } else if text.chars().count() > 200 {
                    text.chars().take(200).collect()
                } else {
                    text.to_string()
                }
            }
        }
    }

    fn build_output_summary(nodes: &[SerializedConversationNode]) -> Option<String> {
        let last_assistant = nodes.iter().rev().find(|n| n.role == "assistant");
        match last_assistant {
            None => None,
            Some(node) => {
                let text = &node.content;
                if text.is_empty() {
                    return Some("(no output yet)".to_string());
                }
                let asst_count = nodes.iter().filter(|n| n.role == "assistant").count();
                if asst_count > 1 {
                    let truncated: String = text.chars().take(100).collect();
                    Some(format!("{}... (+{} more)", truncated, asst_count - 1))
                } else if text.chars().count() > 200 {
                    Some(text.chars().take(200).collect())
                } else {
                    Some(text.to_string())
                }
            }
        }
    }
}

fn parse_entry_type(s: &str) -> HistoryEntryType {
    match s {
        "speech" => HistoryEntryType::Speech,
        _ => HistoryEntryType::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::Database;

    fn make_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn make_svc() -> SqliteHistoryService {
        SqliteHistoryService::new(make_db(), 1000)
    }

    fn make_nodes(user_text: &str, assistant_text: &str) -> Vec<SerializedConversationNode> {
        vec![
            SerializedConversationNode {
                node_id: "u1".into(),
                parent_id: None,
                role: "user".into(),
                content: user_text.into(),
                timestamp: "2026-01-01".into(),
                children: vec!["a1".into()],
                updates: vec![],
                prompt_tokens: None,
                completion_tokens: None,
                thinking: None,
                error: None,
                cancelled: false,
                tool_calls: vec![],
                text_attachments: vec![],
            },
            SerializedConversationNode {
                node_id: "a1".into(),
                parent_id: Some("u1".into()),
                role: "assistant".into(),
                content: assistant_text.into(),
                timestamp: "2026-01-01".into(),
                children: vec![],
                updates: vec![],
                prompt_tokens: None,
                completion_tokens: None,
                thinking: None,
                error: None,
                cancelled: false,
                tool_calls: vec![],
                text_attachments: vec![],
            },
        ]
    }

    #[test]
    fn add_and_get_history() {
        let svc = make_svc();
        svc.add_entry(
            "hello".into(),
            HistoryEntryType::Text,
            Some("world".into()),
            None,
            true,
            None,
            false,
            None,
            false,
        );
        let history = svc.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].input_content, "hello");
    }

    #[test]
    fn add_conversation_and_restore() {
        let svc = make_svc();
        let nodes = make_nodes("hi", "hello");
        let id = svc.add_conversation_entry(
            "context".into(),
            None,
            None,
            true,
            None,
            nodes,
            Some("u1".into()),
            vec!["u1".into(), "a1".into()],
            false,
            None,
            vec![],
            None,
            None,
        );

        let entry = svc.get_entry_by_id(&id).unwrap();
        assert!(entry.is_multi_turn);
        let conv = entry.conversation_data.unwrap();
        assert_eq!(conv.nodes.len(), 2);
        assert_eq!(conv.root_node_id, Some("u1".into()));
        assert_eq!(conv.current_path, vec!["u1", "a1"]);
        assert_eq!(conv.context_text, "context");
    }

    #[test]
    fn update_conversation_entry() {
        let svc = make_svc();
        let nodes = make_nodes("hi", "hello");
        let id = svc.add_conversation_entry(
            "ctx".into(),
            None,
            None,
            true,
            None,
            nodes,
            Some("u1".into()),
            vec!["u1".into(), "a1".into()],
            false,
            None,
            vec![],
            None,
            None,
        );

        let new_nodes = make_nodes("hi updated", "hello updated");
        svc.update_conversation_entry(
            &id,
            "new ctx".into(),
            new_nodes,
            Some("u1".into()),
            vec!["u1".into(), "a1".into()],
            vec![],
            None,
            None,
        )
        .unwrap();

        let entry = svc.get_entry_by_id(&id).unwrap();
        assert!(entry.updated_at.is_some());
        let conv = entry.conversation_data.unwrap();
        assert_eq!(conv.context_text, "new ctx");
        assert_eq!(conv.nodes[0].content, "hi updated");
    }

    #[test]
    fn update_nonexistent_entry_fails() {
        let svc = make_svc();
        let result = svc.update_conversation_entry(
            "nonexistent",
            "ctx".into(),
            vec![],
            None,
            vec![],
            vec![],
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn max_entries_enforcement() {
        let svc = SqliteHistoryService::new(make_db(), 3);
        for i in 0..5 {
            svc.add_entry(
                format!("entry-{}", i),
                HistoryEntryType::Text,
                None,
                None,
                true,
                None,
                false,
                None,
                false,
            );
        }
        assert_eq!(svc.entry_count(), 3);
    }

    #[test]
    fn get_entry_by_id_found_and_not_found() {
        let svc = make_svc();
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
    fn get_last_item_by_type() {
        let svc = make_svc();
        svc.add_entry("t1".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
        svc.add_entry("s1".into(), HistoryEntryType::Speech, None, None, true, None, false, None, false);
        svc.add_entry("t2".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);

        let last_text = svc.get_last_item_by_type(HistoryEntryType::Text).unwrap();
        assert_eq!(last_text.input_content, "t2");

        let last_speech = svc.get_last_item_by_type(HistoryEntryType::Speech).unwrap();
        assert_eq!(last_speech.input_content, "s1");
    }

    #[test]
    fn update_title() {
        let svc = make_svc();
        svc.add_entry("test".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
        let entry = svc.get_history().into_iter().next().unwrap();

        svc.update_entry_title(&entry.id, "My Title".into()).unwrap();
        let updated = svc.get_entry_by_id(&entry.id).unwrap();
        assert_eq!(updated.title, Some("My Title".into()));
    }

    #[test]
    fn clear_removes_all() {
        let svc = make_svc();
        svc.add_entry("test".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
        assert_eq!(svc.entry_count(), 1);
        svc.clear();
        assert_eq!(svc.entry_count(), 0);
    }

    #[test]
    fn image_round_trip() {
        let svc = make_svc();
        let nodes = make_nodes("hi", "hello");
        let image_data = BASE64.encode(b"fake png data");

        let id = svc.add_conversation_entry(
            "ctx".into(),
            None,
            None,
            true,
            None,
            nodes,
            Some("u1".into()),
            vec!["u1".into(), "a1".into()],
            false,
            None,
            vec![
                ImagePayload {
                    node_id: Some("u1".into()),
                    image_index: 0,
                    data: image_data.clone(),
                    media_type: "image/png".into(),
                },
                ImagePayload {
                    node_id: None,
                    image_index: 0,
                    data: image_data.clone(),
                    media_type: "image/jpeg".into(),
                },
            ],
            None,
            None,
        );

        let entry = svc.get_entry_by_id(&id).unwrap();
        let conv = entry.conversation_data.unwrap();

        let node_imgs = conv.node_images.get("u1").unwrap();
        assert_eq!(node_imgs.len(), 1);
        assert_eq!(node_imgs[0].data, image_data);
        assert_eq!(node_imgs[0].media_type, "image/png");

        assert_eq!(conv.context_images.len(), 1);
        assert_eq!(conv.context_images[0].data, image_data);
        assert_eq!(conv.context_images[0].media_type, "image/jpeg");
    }

    #[test]
    fn image_cascade_delete() {
        let svc = make_svc();
        let nodes = make_nodes("hi", "hello");
        let image_data = BASE64.encode(b"data");

        svc.add_conversation_entry(
            "ctx".into(),
            None,
            None,
            true,
            None,
            nodes,
            Some("u1".into()),
            vec!["u1".into(), "a1".into()],
            false,
            None,
            vec![ImagePayload {
                node_id: Some("u1".into()),
                image_index: 0,
                data: image_data,
                media_type: "image/png".into(),
            }],
            None,
            None,
        );

        svc.clear();

        let count: i64 = svc
            .db
            .conn()
            .query_row("SELECT COUNT(*) FROM conversation_images", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn build_input_summary_single() {
        let nodes = make_nodes("hello world", "reply");
        assert_eq!(
            SqliteHistoryService::build_input_summary(&nodes),
            "hello world"
        );
    }

    #[test]
    fn build_input_summary_multi_turn() {
        let mut nodes = make_nodes("first", "reply1");
        nodes.push(SerializedConversationNode {
            node_id: "u2".into(),
            parent_id: Some("a1".into()),
            role: "user".into(),
            content: "second".into(),
            timestamp: "2026-01-01".into(),
            children: vec![],
            updates: vec![],
            prompt_tokens: None,
            completion_tokens: None,
            thinking: None,
            error: None,
            cancelled: false,
            tool_calls: vec![],
            text_attachments: vec![],
        });
        let summary = SqliteHistoryService::build_input_summary(&nodes);
        assert!(summary.contains("second"));
        assert!(summary.contains("+1 more"));
    }

    #[test]
    fn build_input_summary_empty() {
        assert_eq!(
            SqliteHistoryService::build_input_summary(&[]),
            "(no input)"
        );
    }

    #[test]
    fn build_output_summary_none() {
        let nodes = vec![SerializedConversationNode {
            node_id: "u1".into(),
            parent_id: None,
            role: "user".into(),
            content: "hi".into(),
            timestamp: "2026-01-01".into(),
            children: vec![],
            updates: vec![],
            prompt_tokens: None,
            completion_tokens: None,
            thinking: None,
            error: None,
            cancelled: false,
            tool_calls: vec![],
            text_attachments: vec![],
        }];
        assert!(SqliteHistoryService::build_output_summary(&nodes).is_none());
    }

    #[test]
    fn quick_action_query() {
        let svc = make_svc();
        svc.add_entry("normal".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
        svc.add_entry("quick".into(), HistoryEntryType::Text, None, None, true, None, false, None, true);

        let last_quick = svc.get_last_quick_action(HistoryEntryType::Text).unwrap();
        assert_eq!(last_quick.input_content, "quick");
    }
}
