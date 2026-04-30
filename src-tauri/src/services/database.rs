use std::path::Path;

use rusqlite::Connection;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(app_data_dir: &Path) -> Result<Self, DatabaseError> {
        std::fs::create_dir_all(app_data_dir)?;
        let db_path = app_data_dir.join("history.db");
        let conn = Connection::open(&db_path)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        run_migrations(&conn)?;

        log::info!("database opened at {}", db_path.display());
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn get_schema_version(conn: &Connection) -> i32 {
    conn.query_row(
        "SELECT version FROM schema_version LIMIT 1",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<(), DatabaseError> {
    conn.execute("DELETE FROM schema_version", [])?;
    conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    Ok(())
}

fn run_migrations(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        [],
    )?;

    let version = get_schema_version(conn);

    if version < 1 {
        migrate_to_v1(conn)?;
    }

    if version < 2 {
        migrate_to_v2(conn)?;
    }

    if version < 3 {
        migrate_to_v3(conn)?;
    }

    if version < 4 {
        migrate_to_v4(conn)?;
    }

    Ok(())
}

fn migrate_to_v1(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        "CREATE TABLE conversations (
            id TEXT PRIMARY KEY,
            title TEXT,
            skill_id TEXT,
            skill_name TEXT,
            entry_type TEXT NOT NULL,
            input_content TEXT NOT NULL,
            output_content TEXT,
            success INTEGER NOT NULL DEFAULT 1,
            error TEXT,
            is_multi_turn INTEGER NOT NULL DEFAULT 0,
            quick_action INTEGER NOT NULL DEFAULT 0,
            context_text TEXT DEFAULT '',
            tree_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT
        );

        CREATE TABLE conversation_images (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            node_id TEXT,
            image_index INTEGER NOT NULL DEFAULT 0,
            data BLOB NOT NULL,
            mime_type TEXT NOT NULL
        );

        CREATE INDEX idx_conversations_updated ON conversations(COALESCE(updated_at, created_at) DESC);
        CREATE INDEX idx_conv_images_conv_id ON conversation_images(conversation_id);",
    )?;

    set_schema_version(conn, 1)?;
    log::info!("database migrated to schema version 1");
    Ok(())
}

fn migrate_to_v2(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute(
        "ALTER TABLE conversations ADD COLUMN input_content_rendered TEXT",
        [],
    )?;
    conn.execute(
        "ALTER TABLE conversations ADD COLUMN output_content_rendered TEXT",
        [],
    )?;
    set_schema_version(conn, 2)?;
    log::info!("database migrated to schema version 2");
    Ok(())
}

fn parse_xml_skill_blocks(content: &str) -> Option<Vec<(String, String, String)>> {
    if !content.starts_with("<skill name=\"") {
        return None;
    }

    let mut blocks = Vec::new();
    let mut cursor = 0usize;

    loop {
        let rest = &content[cursor..];
        let Some(rel) = rest.find("<skill name=\"") else {
            break;
        };
        let open_pos = cursor + rel;
        let name_start = open_pos + "<skill name=\"".len();
        let Some(name_end_rel) = content[name_start..].find('"') else {
            break;
        };
        let name = content[name_start..name_start + name_end_rel].to_string();
        let after_open_quote = name_start + name_end_rel + 1;
        if !content[after_open_quote..].starts_with('>') {
            break;
        }
        let body_start = after_open_quote + 1;
        let Some(body_end_rel) = content[body_start..].find("</skill>") else {
            break;
        };
        let body = content[body_start..body_start + body_end_rel]
            .trim_matches('\n')
            .to_string();
        let after_close = body_start + body_end_rel + "</skill>".len();

        let Some(input_marker_rel) = content[after_close..].find("<input>") else {
            break;
        };
        let input_start = after_close + input_marker_rel + "<input>".len();
        let Some(input_end_rel) = content[input_start..].find("</input>") else {
            break;
        };
        let input = content[input_start..input_start + input_end_rel]
            .trim_matches('\n')
            .to_string();
        let after_input_close = input_start + input_end_rel + "</input>".len();

        blocks.push((name, body, input));
        cursor = after_input_close;
    }

    if blocks.is_empty() {
        None
    } else {
        Some(blocks)
    }
}

fn slash_form_for_blocks(blocks: &[(String, String, String)]) -> String {
    blocks
        .iter()
        .map(|(name, _, input)| {
            if input.is_empty() {
                format!("/{}", name)
            } else {
                format!("/{} {}", name, input)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn migrate_tree_json_user_nodes(tree_json: &str) -> Option<(String, Option<String>, Option<String>)> {
    let mut value: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    let nodes = value.get_mut("nodes")?.as_array_mut()?;

    let mut changed = false;
    for node in nodes.iter_mut() {
        let role = node.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "user" {
            continue;
        }
        let already_has = node
            .get("applied_skills")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if already_has {
            continue;
        }
        let content = node
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let Some(blocks) = parse_xml_skill_blocks(&content) else {
            continue;
        };
        let slash = slash_form_for_blocks(&blocks);
        let applied: Vec<serde_json::Value> = blocks
            .into_iter()
            .map(|(name, body, input)| {
                serde_json::json!({
                    "name": name,
                    "body_snapshot": body,
                    "input": input,
                })
            })
            .collect();
        if let Some(map) = node.as_object_mut() {
            map.insert(
                "content".to_string(),
                serde_json::Value::String(slash),
            );
            map.insert(
                "applied_skills".to_string(),
                serde_json::Value::Array(applied),
            );
        }
        changed = true;
    }

    let mut last_user_content: Option<String> = None;
    let mut last_assistant_content: Option<String> = None;
    if let Some(nodes_arr) = value.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes_arr.iter().rev() {
            let role = node.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content = node
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            if role == "user" && last_user_content.is_none() {
                last_user_content = Some(content);
            } else if role == "assistant" && last_assistant_content.is_none() {
                last_assistant_content = Some(content);
            }
            if last_user_content.is_some() && last_assistant_content.is_some() {
                break;
            }
        }
    }

    if !changed {
        return None;
    }

    let new_json = serde_json::to_string(&value).ok()?;
    Some((new_json, last_user_content, last_assistant_content))
}

fn migrate_to_v4(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS conversations_fts_ai;
         DROP TRIGGER IF EXISTS conversations_fts_ad;
         DROP TRIGGER IF EXISTS conversations_fts_au;
         DROP TABLE IF EXISTS conversations_fts;",
    )?;

    let _ = conn.execute(
        "ALTER TABLE conversations DROP COLUMN input_content_rendered",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE conversations DROP COLUMN output_content_rendered",
        [],
    );

    conn.execute(
        "ALTER TABLE conversations ADD COLUMN applied_skill_names TEXT",
        [],
    )?;

    let rows: Vec<(String, Option<String>)> = {
        let mut stmt = conn.prepare("SELECT id, tree_json FROM conversations WHERE tree_json IS NOT NULL")?;
        let iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let tree: Option<String> = row.get(1)?;
            Ok((id, tree))
        })?;
        iter.filter_map(|r| r.ok()).collect()
    };

    let mut migrated_count = 0usize;
    for (id, tree_opt) in rows {
        let Some(tree) = tree_opt else { continue };
        let Some((new_tree, last_user, last_assistant)) = migrate_tree_json_user_nodes(&tree) else {
            continue;
        };

        let mut applied_names: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&new_tree) {
            if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
                for node in nodes {
                    if let Some(arr) = node.get("applied_skills").and_then(|v| v.as_array()) {
                        for s in arr {
                            if let Some(n) = s.get("name").and_then(|v| v.as_str()) {
                                applied_names.insert(n.to_string());
                            }
                        }
                    }
                }
            }
        }
        let applied_csv: Option<String> = if applied_names.is_empty() {
            None
        } else {
            Some(applied_names.into_iter().collect::<Vec<_>>().join(","))
        };

        let user_summary: Option<String> = last_user.map(|c| {
            if c.chars().count() > 200 {
                c.chars().take(200).collect()
            } else {
                c
            }
        });
        let asst_summary: Option<String> = last_assistant.map(|c| {
            if c.chars().count() > 200 {
                c.chars().take(200).collect()
            } else {
                c
            }
        });

        match (user_summary, asst_summary) {
            (Some(u), Some(a)) => {
                conn.execute(
                    "UPDATE conversations SET tree_json = ?1, input_content = ?2, output_content = ?3, applied_skill_names = ?4 WHERE id = ?5",
                    rusqlite::params![new_tree, u, a, applied_csv, id],
                )?;
            }
            (Some(u), None) => {
                conn.execute(
                    "UPDATE conversations SET tree_json = ?1, input_content = ?2, applied_skill_names = ?3 WHERE id = ?4",
                    rusqlite::params![new_tree, u, applied_csv, id],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE conversations SET tree_json = ?1, applied_skill_names = ?2 WHERE id = ?3",
                    rusqlite::params![new_tree, applied_csv, id],
                )?;
            }
        }
        migrated_count += 1;
    }

    if migrated_count > 0 {
        log::info!("v4 migration: rewrote {} entries from XML to slash form", migrated_count);
    }

    conn.execute_batch(
        "CREATE VIRTUAL TABLE conversations_fts USING fts5(
            title, skill_name, input_content, output_content,
            tokenize='unicode61 remove_diacritics 2'
        );

        INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
        SELECT rowid,
               COALESCE(title, ''),
               COALESCE(skill_name, ''),
               COALESCE(input_content, ''),
               COALESCE(output_content, '')
        FROM conversations;

        CREATE TRIGGER conversations_fts_ai AFTER INSERT ON conversations BEGIN
          INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
          VALUES (new.rowid,
                  COALESCE(new.title, ''),
                  COALESCE(new.skill_name, ''),
                  COALESCE(new.input_content, ''),
                  COALESCE(new.output_content, ''));
        END;

        CREATE TRIGGER conversations_fts_ad AFTER DELETE ON conversations BEGIN
          DELETE FROM conversations_fts WHERE rowid = old.rowid;
        END;

        CREATE TRIGGER conversations_fts_au AFTER UPDATE ON conversations BEGIN
          DELETE FROM conversations_fts WHERE rowid = old.rowid;
          INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
          VALUES (new.rowid,
                  COALESCE(new.title, ''),
                  COALESCE(new.skill_name, ''),
                  COALESCE(new.input_content, ''),
                  COALESCE(new.output_content, ''));
        END;",
    )?;

    set_schema_version(conn, 4)?;
    log::info!("database migrated to schema version 4");
    Ok(())
}

fn migrate_to_v3(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE conversations_fts USING fts5(
            title, skill_name, input_content, output_content,
            tokenize='unicode61 remove_diacritics 2'
        );

        INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
        SELECT rowid,
               COALESCE(title, ''),
               COALESCE(skill_name, ''),
               COALESCE(input_content_rendered, input_content, ''),
               COALESCE(output_content_rendered, output_content, '')
        FROM conversations;

        CREATE TRIGGER conversations_fts_ai AFTER INSERT ON conversations BEGIN
          INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
          VALUES (new.rowid,
                  COALESCE(new.title, ''),
                  COALESCE(new.skill_name, ''),
                  COALESCE(new.input_content_rendered, new.input_content, ''),
                  COALESCE(new.output_content_rendered, new.output_content, ''));
        END;

        CREATE TRIGGER conversations_fts_ad AFTER DELETE ON conversations BEGIN
          DELETE FROM conversations_fts WHERE rowid = old.rowid;
        END;

        CREATE TRIGGER conversations_fts_au AFTER UPDATE ON conversations BEGIN
          DELETE FROM conversations_fts WHERE rowid = old.rowid;
          INSERT INTO conversations_fts(rowid, title, skill_name, input_content, output_content)
          VALUES (new.rowid,
                  COALESCE(new.title, ''),
                  COALESCE(new.skill_name, ''),
                  COALESCE(new.input_content_rendered, new.input_content, ''),
                  COALESCE(new.output_content_rendered, new.output_content, ''));
        END;",
    )?;
    set_schema_version(conn, 3)?;
    log::info!("database migrated to schema version 3 (FTS5)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_tables() {
        let db = Database::open_in_memory().unwrap();
        let count: i32 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn schema_version_is_set() {
        let db = Database::open_in_memory().unwrap();
        let version = get_schema_version(db.conn());
        assert_eq!(version, 4);
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        run_migrations(db.conn()).unwrap();
        let version = get_schema_version(db.conn());
        assert_eq!(version, 4);
    }

    #[test]
    fn parse_xml_blocks_single_skill() {
        let content = "<skill name=\"translate\">\nTranslate to English.\n</skill>\n\n<input>\nCześć!\n</input>";
        let blocks = parse_xml_skill_blocks(content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, "translate");
        assert_eq!(blocks[0].1, "Translate to English.");
        assert_eq!(blocks[0].2, "Cześć!");
    }

    #[test]
    fn parse_xml_blocks_multi_skill() {
        let content = "<skill name=\"translate\">\nTranslate.\n</skill>\n\n<input>\nhello\n</input>\n\n<skill-end name=\"translate\" />\n<skill name=\"formal\">\nMake formal.\n</skill>\n\n<input>\nworld\n</input>";
        let blocks = parse_xml_skill_blocks(content).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, "translate");
        assert_eq!(blocks[1].0, "formal");
        assert_eq!(blocks[1].2, "world");
    }

    #[test]
    fn parse_xml_blocks_returns_none_for_plain_text() {
        assert!(parse_xml_skill_blocks("just plain text").is_none());
        assert!(parse_xml_skill_blocks("/translate hello").is_none());
    }

    #[test]
    fn migrate_tree_json_rewrites_xml_to_slash() {
        let original = serde_json::json!({
            "nodes": [
                {
                    "node_id": "u1",
                    "role": "user",
                    "content": "<skill name=\"translate\">\nBody\n</skill>\n\n<input>\nhi\n</input>",
                    "children": []
                },
                {
                    "node_id": "a1",
                    "role": "assistant",
                    "content": "Cześć!",
                    "children": []
                }
            ],
            "root_node_id": "u1",
            "current_path": ["u1", "a1"]
        });
        let json = serde_json::to_string(&original).unwrap();
        let (new_json, last_user, last_assistant) = migrate_tree_json_user_nodes(&json).unwrap();
        let value: serde_json::Value = serde_json::from_str(&new_json).unwrap();
        let nodes = value.get("nodes").unwrap().as_array().unwrap();
        assert_eq!(nodes[0].get("content").unwrap().as_str().unwrap(), "/translate hi");
        let applied = nodes[0].get("applied_skills").unwrap().as_array().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].get("name").unwrap().as_str().unwrap(), "translate");
        assert_eq!(applied[0].get("body_snapshot").unwrap().as_str().unwrap(), "Body");
        assert_eq!(applied[0].get("input").unwrap().as_str().unwrap(), "hi");
        assert_eq!(last_user.as_deref(), Some("/translate hi"));
        assert_eq!(last_assistant.as_deref(), Some("Cześć!"));
    }

    #[test]
    fn migrate_tree_json_skips_already_migrated() {
        let original = serde_json::json!({
            "nodes": [
                {
                    "node_id": "u1",
                    "role": "user",
                    "content": "/translate hi",
                    "applied_skills": [{"name": "translate", "body_snapshot": "B", "input": "hi"}],
                    "children": []
                }
            ],
            "root_node_id": "u1",
            "current_path": ["u1"]
        });
        let json = serde_json::to_string(&original).unwrap();
        assert!(migrate_tree_json_user_nodes(&json).is_none());
    }

    #[test]
    fn fts_table_is_created() {
        let db = Database::open_in_memory().unwrap();
        let count: i32 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversations_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn fts_triggers_keep_index_in_sync() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute(
            "INSERT INTO conversations (id, entry_type, input_content, success, is_multi_turn, quick_action, created_at, title, skill_name)
             VALUES ('a', 'text', 'fox jumps', 1, 0, 0, '2026-01-01 00:00:00', 'react hooks', 'translate')",
            [],
        ).unwrap();
        let cnt: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM conversations_fts WHERE conversations_fts MATCH 'react'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(cnt, 1);

        db.conn().execute(
            "UPDATE conversations SET title = 'svelte stores' WHERE id = 'a'",
            [],
        ).unwrap();
        let cnt2: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM conversations_fts WHERE conversations_fts MATCH 'react'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(cnt2, 0);

        let cnt3: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM conversations_fts WHERE conversations_fts MATCH 'svelte'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(cnt3, 1);

        db.conn().execute("DELETE FROM conversations WHERE id = 'a'", []).unwrap();
        let cnt4: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM conversations_fts",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(cnt4, 0);
    }

    #[test]
    fn open_on_disk_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let _db = Database::open(dir.path()).unwrap();
        assert!(dir.path().join("history.db").exists());
    }
}
