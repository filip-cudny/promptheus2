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
        assert_eq!(version, 3);
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        run_migrations(db.conn()).unwrap();
        let version = get_schema_version(db.conn());
        assert_eq!(version, 3);
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
