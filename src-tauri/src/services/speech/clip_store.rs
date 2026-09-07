use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, thiserror::Error)]
pub enum ClipStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Audio clip not found")]
    NotFound,
    #[error("Audio clip file is missing from disk")]
    FileMissing,
}

#[derive(Debug, Clone)]
pub struct AudioClip {
    pub id: String,
    pub history_entry_id: Option<String>,
    pub path: PathBuf,
    pub sample_rate: u32,
    pub duration_secs: f64,
    pub bytes: u64,
    pub created_at: String,
    pub expires_at: String,
}

/// Short-lived WAV storage backing transcription retries.
///
/// Files live on disk under `audio_clips/`; the `audio_clips` table is the index.
/// SQL is passed a borrowed connection (owned by `SqliteHistoryService`) so both
/// stay on a single database handle.
pub struct AudioClipStore {
    dir: PathBuf,
}

impl AudioClipStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            dir: app_data_dir.join("audio_clips"),
        }
    }

    pub fn initialize(&self) -> Result<(), ClipStoreError> {
        fs::create_dir_all(&self.dir)?;
        Ok(())
    }

    pub fn store(
        &self,
        conn: &Connection,
        wav_bytes: &[u8],
        sample_rate: u32,
        duration_secs: f64,
        retention_hours: u32,
    ) -> Result<String, ClipStoreError> {
        fs::create_dir_all(&self.dir)?;

        let id = uuid::Uuid::new_v4().to_string();
        let path = self.dir.join(format!("{id}.wav"));
        fs::write(&path, wav_bytes)?;

        let now = chrono::Local::now();
        let expires = now + chrono::Duration::hours(retention_hours.max(1) as i64);

        conn.execute(
            "INSERT INTO audio_clips (id, history_entry_id, path, sample_rate, duration_secs, bytes, created_at, expires_at)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                path.to_string_lossy(),
                sample_rate,
                duration_secs,
                wav_bytes.len() as i64,
                format_ts(now),
                format_ts(expires),
            ],
        )?;

        log::debug!(
            "audio clip stored id={id} bytes={} duration={duration_secs:.1}s expires_at={}",
            wav_bytes.len(),
            format_ts(expires)
        );

        Ok(id)
    }

    pub fn attach_to_entry(
        &self,
        conn: &Connection,
        clip_id: &str,
        entry_id: &str,
    ) -> Result<(), ClipStoreError> {
        let updated = conn.execute(
            "UPDATE audio_clips SET history_entry_id = ?1 WHERE id = ?2",
            rusqlite::params![entry_id, clip_id],
        )?;
        if updated == 0 {
            return Err(ClipStoreError::NotFound);
        }
        Ok(())
    }

    pub fn find_by_entry(&self, conn: &Connection, entry_id: &str) -> Option<AudioClip> {
        conn.query_row(
            "SELECT id, history_entry_id, path, sample_rate, duration_secs, bytes, created_at, expires_at
             FROM audio_clips WHERE history_entry_id = ?1
             ORDER BY created_at DESC LIMIT 1",
            [entry_id],
            row_to_clip,
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn read_bytes(&self, clip: &AudioClip) -> Result<Vec<u8>, ClipStoreError> {
        if !clip.path.exists() {
            return Err(ClipStoreError::FileMissing);
        }
        Ok(fs::read(&clip.path)?)
    }

    pub fn copy_to(&self, clip: &AudioClip, destination: &Path) -> Result<(), ClipStoreError> {
        if !clip.path.exists() {
            return Err(ClipStoreError::FileMissing);
        }
        fs::copy(&clip.path, destination)?;
        Ok(())
    }

    pub fn delete(&self, conn: &Connection, clip_id: &str) -> Result<(), ClipStoreError> {
        let path: Option<String> = conn
            .query_row(
                "SELECT path FROM audio_clips WHERE id = ?1",
                [clip_id],
                |row| row.get(0),
            )
            .optional()?;

        conn.execute("DELETE FROM audio_clips WHERE id = ?1", [clip_id])?;

        if let Some(path) = path {
            let _ = fs::remove_file(path);
        }
        Ok(())
    }

    pub fn delete_by_entry(&self, conn: &Connection, entry_id: &str) -> Result<(), ClipStoreError> {
        let ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM audio_clips WHERE history_entry_id = ?1")?;
            let iter = stmt.query_map([entry_id], |row| row.get::<_, String>(0))?;
            iter.filter_map(|r| r.ok()).collect()
        };
        for id in ids {
            self.delete(conn, &id)?;
        }
        Ok(())
    }

    /// Drops expired rows with their files, then removes files no row points at.
    /// The orphan pass is what reclaims clips lost to `ON DELETE CASCADE`,
    /// history retention pruning, and `clear()`.
    pub fn sweep(&self, conn: &Connection) -> usize {
        let expired: Vec<(String, String)> = match conn
            .prepare("SELECT id, path FROM audio_clips WHERE expires_at < datetime('now', 'localtime')")
        {
            Ok(mut stmt) => match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?))) {
                Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    log::warn!("audio clip sweep query failed: {e}");
                    return 0;
                }
            },
            Err(e) => {
                log::warn!("audio clip sweep prepare failed: {e}");
                return 0;
            }
        };

        let mut removed = 0usize;
        for (id, path) in expired {
            let _ = fs::remove_file(&path);
            if conn
                .execute("DELETE FROM audio_clips WHERE id = ?1", [&id])
                .is_ok()
            {
                removed += 1;
            }
        }

        removed += self.sweep_orphan_files(conn);

        if removed > 0 {
            log::info!("audio clip sweep removed {removed} clips");
        }
        removed
    }

    fn sweep_orphan_files(&self, conn: &Connection) -> usize {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return 0;
        };

        let known: HashSet<String> = match conn.prepare("SELECT id FROM audio_clips") {
            Ok(mut stmt) => match stmt.query_map([], |row| row.get::<_, String>(0)) {
                Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
                Err(_) => return 0,
            },
            Err(_) => return 0,
        };

        let mut removed = 0usize;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wav") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if known.contains(stem) {
                continue;
            }
            if fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        removed
    }
}

fn format_ts(value: chrono::DateTime<chrono::Local>) -> String {
    value.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn row_to_clip(row: &rusqlite::Row<'_>) -> rusqlite::Result<AudioClip> {
    Ok(AudioClip {
        id: row.get(0)?,
        history_entry_id: row.get(1)?,
        path: PathBuf::from(row.get::<_, String>(2)?),
        sample_rate: row.get(3)?,
        duration_secs: row.get(4)?,
        bytes: row.get::<_, i64>(5)? as u64,
        created_at: row.get(6)?,
        expires_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::Database;

    fn setup() -> (tempfile::TempDir, AudioClipStore, Database) {
        let dir = tempfile::tempdir().unwrap();
        let store = AudioClipStore::new(dir.path());
        store.initialize().unwrap();
        let db = Database::open_in_memory().unwrap();
        (dir, store, db)
    }

    #[test]
    fn store_writes_file_and_row() {
        let (_dir, store, db) = setup();
        let id = store.store(db.conn(), &[1, 2, 3, 4], 16000, 1.5, 3).unwrap();

        let clip = db
            .conn()
            .query_row(
                "SELECT id, history_entry_id, path, sample_rate, duration_secs, bytes, created_at, expires_at FROM audio_clips WHERE id = ?1",
                [&id],
                row_to_clip,
            )
            .unwrap();

        assert_eq!(clip.bytes, 4);
        assert_eq!(clip.sample_rate, 16000);
        assert!(clip.path.exists());
        assert!(clip.history_entry_id.is_none());
    }

    #[test]
    fn attach_and_find_by_entry() {
        let (_dir, store, db) = setup();
        db.conn()
            .execute(
                "INSERT INTO conversations (id, entry_type, input_content, created_at) VALUES ('e1', 'speech', 'x', datetime('now'))",
                [],
            )
            .unwrap();
        let id = store.store(db.conn(), &[0; 8], 16000, 0.5, 3).unwrap();
        store.attach_to_entry(db.conn(), &id, "e1").unwrap();

        let found = store.find_by_entry(db.conn(), "e1").unwrap();
        assert_eq!(found.id, id);
        assert_eq!(store.read_bytes(&found).unwrap().len(), 8);
    }

    #[test]
    fn delete_by_entry_removes_file() {
        let (_dir, store, db) = setup();
        db.conn()
            .execute(
                "INSERT INTO conversations (id, entry_type, input_content, created_at) VALUES ('e1', 'speech', 'x', datetime('now'))",
                [],
            )
            .unwrap();
        let id = store.store(db.conn(), &[0; 8], 16000, 0.5, 3).unwrap();
        store.attach_to_entry(db.conn(), &id, "e1").unwrap();
        let path = store.find_by_entry(db.conn(), "e1").unwrap().path;

        store.delete_by_entry(db.conn(), "e1").unwrap();

        assert!(!path.exists());
        assert!(store.find_by_entry(db.conn(), "e1").is_none());
    }

    #[test]
    fn sweep_removes_expired_clips() {
        let (_dir, store, db) = setup();
        let id = store.store(db.conn(), &[0; 8], 16000, 0.5, 3).unwrap();
        db.conn()
            .execute(
                "UPDATE audio_clips SET expires_at = datetime('now', 'localtime', '-1 hour') WHERE id = ?1",
                [&id],
            )
            .unwrap();

        assert_eq!(store.sweep(db.conn()), 1);
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM audio_clips", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn sweep_removes_orphan_files_left_by_cascade() {
        let (_dir, store, db) = setup();
        db.conn()
            .execute(
                "INSERT INTO conversations (id, entry_type, input_content, created_at) VALUES ('e1', 'speech', 'x', datetime('now'))",
                [],
            )
            .unwrap();
        let id = store.store(db.conn(), &[0; 8], 16000, 0.5, 3).unwrap();
        store.attach_to_entry(db.conn(), &id, "e1").unwrap();
        let path = store.find_by_entry(db.conn(), "e1").unwrap().path;

        db.conn()
            .execute("DELETE FROM conversations WHERE id = 'e1'", [])
            .unwrap();
        assert!(path.exists());

        assert_eq!(store.sweep(db.conn()), 1);
        assert!(!path.exists());
    }

    #[test]
    fn read_bytes_reports_missing_file() {
        let (_dir, store, db) = setup();
        let id = store.store(db.conn(), &[0; 8], 16000, 0.5, 3).unwrap();
        let clip = db
            .conn()
            .query_row(
                "SELECT id, history_entry_id, path, sample_rate, duration_secs, bytes, created_at, expires_at FROM audio_clips WHERE id = ?1",
                [&id],
                row_to_clip,
            )
            .unwrap();
        fs::remove_file(&clip.path).unwrap();

        assert!(matches!(
            store.read_bytes(&clip),
            Err(ClipStoreError::FileMissing)
        ));
    }
}
