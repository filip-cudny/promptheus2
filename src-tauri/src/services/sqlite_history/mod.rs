mod codec;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::models::history::{
    ConversationHistoryData, HistoryEntry, HistoryEntryType, ImagePayload,
    SerializedConversationNode,
};
use crate::models::message::ImageData;
use crate::services::database::Database;
use crate::services::history_search::{HistoryStatusFilter, HistoryTypeFilter};

use codec::{
    build_applied_skill_names, build_input_summary, build_output_summary, row_to_entry,
    TreeJson, ENTRY_COLUMNS,
};

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
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
    ) -> Option<String> {
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
            return None;
        }

        self.enforce_max_entries();
        Some(id)
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

        let input_summary = build_input_summary(&nodes);
        let output_summary = build_output_summary(&nodes);
        let applied_skill_names = build_applied_skill_names(&nodes);

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
            "INSERT INTO conversations (id, entry_type, input_content, output_content, skill_id, skill_name, success, error, is_multi_turn, quick_action, context_text, tree_json, created_at, applied_skill_names)
             VALUES (?1, 'text', ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11, ?12)",
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
                applied_skill_names,
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

        let input_summary = build_input_summary(&nodes);
        let output_summary = build_output_summary(&nodes);
        let applied_skill_names = build_applied_skill_names(&nodes);

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
            "UPDATE conversations SET input_content = ?1, output_content = ?2, context_text = ?3, tree_json = ?4, updated_at = ?5, applied_skill_names = ?6 WHERE id = ?7",
            rusqlite::params![input_summary, output_summary, context_text, tree_json, now, applied_skill_names, entry_id],
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
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM conversations
             WHERE quick_action = 0
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC
             LIMIT ?1 OFFSET ?2"
        );
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(rusqlite::params![limit, offset], row_to_entry)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM conversations
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC"
        );
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], row_to_entry)
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
                        entry_type: codec::parse_entry_type(row.get::<_, String>(4)?.as_str()),
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
                .query_map([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
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
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM conversations WHERE entry_type = ?1
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC LIMIT 1"
        );
        self.db.conn().query_row(&sql, [type_str], row_to_entry).ok()
    }

    pub fn get_last_quick_action(&self, entry_type: HistoryEntryType) -> Option<HistoryEntry> {
        let type_str = match entry_type {
            HistoryEntryType::Text => "text",
            HistoryEntryType::Speech => "speech",
        };
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM conversations WHERE entry_type = ?1 AND quick_action = 1
             ORDER BY COALESCE(updated_at, created_at) DESC, rowid DESC LIMIT 1"
        );
        self.db.conn().query_row(&sql, [type_str], row_to_entry).ok()
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
        let now = Self::now_timestamp();
        let updated = self.db.conn().execute(
            "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, entry_id],
        )?;
        if updated == 0 {
            return Err(HistoryError::EntryNotFound(entry_id.to_string()));
        }
        Ok(())
    }

    pub fn delete_entry(&self, entry_id: &str) -> Result<(), HistoryError> {
        self.db.conn().execute(
            "DELETE FROM conversation_images WHERE conversation_id = ?1",
            rusqlite::params![entry_id],
        )?;
        let deleted = self.db.conn().execute(
            "DELETE FROM conversations WHERE id = ?1",
            rusqlite::params![entry_id],
        )?;
        if deleted == 0 {
            return Err(HistoryError::EntryNotFound(entry_id.to_string()));
        }
        Ok(())
    }

    pub fn clear(&self) {
        let _ = self.db.conn().execute("DELETE FROM conversations", []);
    }

    pub fn conn(&self) -> &rusqlite::Connection {
        self.db.conn()
    }

    pub fn entry_count(&self) -> usize {
        self.db
            .conn()
            .query_row("SELECT COUNT(*) FROM conversations", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as usize
    }

    pub fn search_fts(
        &self,
        fts_query: &str,
        type_filter: HistoryTypeFilter,
        status_filter: HistoryStatusFilter,
        skill_ids: &[String],
        limit: usize,
    ) -> Result<Vec<(HistoryEntry, f64)>, HistoryError> {
        let mut sql = String::from(
            "SELECT c.id, c.title, c.skill_id, c.skill_name, c.entry_type, c.input_content, c.output_content,
                    c.success, c.error, c.is_multi_turn, c.quick_action, c.created_at, c.updated_at,
                    bm25(conversations_fts, 10.0, 5.0, 2.0, 1.0) AS rank
             FROM conversations_fts
             JOIN conversations c ON c.rowid = conversations_fts.rowid
             WHERE conversations_fts MATCH ?1",
        );

        match type_filter {
            HistoryTypeFilter::All => {}
            HistoryTypeFilter::Chat => sql.push_str(" AND c.quick_action = 0"),
            HistoryTypeFilter::Speech => sql.push_str(
                " AND c.quick_action = 1 AND c.entry_type = 'speech' AND c.skill_name IS NULL",
            ),
            HistoryTypeFilter::QuickAction => sql.push_str(
                " AND c.quick_action = 1 AND NOT (c.entry_type = 'speech' AND c.skill_name IS NULL)",
            ),
        }

        match status_filter {
            HistoryStatusFilter::All => {}
            HistoryStatusFilter::Success => sql.push_str(" AND c.success = 1"),
            HistoryStatusFilter::Error => sql.push_str(" AND c.success = 0"),
        }

        if !skill_ids.is_empty() {
            sql.push_str(" AND c.skill_id IN (");
            for i in 0..skill_ids.len() {
                if i > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", i + 2));
            }
            sql.push(')');
        }

        sql.push_str(&format!(" ORDER BY rank LIMIT {}", limit));

        let conn = self.db.conn();
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&fts_query];
        for s in skill_ids {
            params.push(s);
        }

        let rows = stmt.query_map(params.as_slice(), |row| {
            let entry = row_to_entry(row)?;
            let rank: f64 = row.get(13)?;
            Ok((entry, rank))
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
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
}
