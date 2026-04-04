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
        assert_eq!(version, 1);
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        run_migrations(db.conn()).unwrap();
        let version = get_schema_version(db.conn());
        assert_eq!(version, 1);
    }

    #[test]
    fn open_on_disk_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let _db = Database::open(dir.path()).unwrap();
        assert!(dir.path().join("history.db").exists());
    }
}
