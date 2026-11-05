#[cfg(test)]
mod tests;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

pub const CHANGE_LOG_DIR_NAME: &str = "change-log";
const SECRET_BACKUP_DIR_NAME: &str = "secret-backups";
const SHORT_ID_LEN: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum ChangeLogError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Card,
    Secret,
    Undo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    Setting { path: String },
    File { path: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub target: Target,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub ok: bool,
    pub millis: Option<u64>,
    pub error: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedOperation {
    pub label: String,
    pub op: String,
    pub effects: Vec<Effect>,
    pub test: Option<TestResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretChange {
    pub name: String,
    pub backup: Option<String>,
    pub had_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeLogEntry {
    pub id: String,
    pub created_at: String,
    pub host: String,
    pub kind: EntryKind,
    pub request: Option<String>,
    pub reply: Option<String>,
    pub operations: Vec<LoggedOperation>,
    pub undoes: Option<String>,
    pub secret: Option<SecretChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    Applied,
    Undone,
}

pub fn change_log_dir(config_dir: &Path) -> PathBuf {
    let settings = config_dir.join("settings.json");
    let is_symlink = std::fs::symlink_metadata(&settings)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let target_parent = is_symlink
        .then(|| std::fs::canonicalize(&settings).ok())
        .flatten()
        .and_then(|target| target.parent().map(Path::to_path_buf));
    target_parent
        .unwrap_or_else(|| config_dir.to_path_buf())
        .join(CHANGE_LOG_DIR_NAME)
}

pub fn secret_backup_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(SECRET_BACKUP_DIR_NAME)
}

pub fn new_id(now: DateTime<Utc>, host: &str) -> String {
    let host: String = host
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let uuid = uuid::Uuid::new_v4().simple().to_string();
    format!(
        "{}-{}-{}",
        now.format("%Y%m%dT%H%M%SZ"),
        host,
        &uuid[..SHORT_ID_LEN]
    )
}

pub fn write_entry(dir: &Path, entry: &ChangeLogEntry) -> Result<PathBuf, ChangeLogError> {
    let json = serde_json::to_string_pretty(entry)?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", entry.id));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
    file.write_all(json.as_bytes())?;
    log::debug!("change log: wrote entry {} ({:?})", entry.id, entry.kind);
    Ok(path)
}

pub fn load_all(dir: &Path) -> Vec<ChangeLogEntry> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            log::warn!("change log: cannot read {}: {}", dir.display(), e);
            return Vec::new();
        }
    };
    let mut entries: Vec<ChangeLogEntry> = read_dir
        .flatten()
        .map(|item| item.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| match read_entry(&path) {
            Ok(entry) => Some(entry),
            Err(e) => {
                log::warn!("change log: skipping {}: {}", path.display(), e);
                None
            }
        })
        .collect();
    entries.sort_by(|a, b| (&a.created_at, &a.id).cmp(&(&b.created_at, &b.id)));
    entries
}

fn read_entry(path: &Path) -> Result<ChangeLogEntry, ChangeLogError> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

pub fn entry_state(entries: &[ChangeLogEntry], id: &str) -> EntryState {
    state_of(entries, id, &mut Vec::new())
}

fn state_of<'a>(entries: &'a [ChangeLogEntry], id: &'a str, visiting: &mut Vec<&'a str>) -> EntryState {
    if visiting.contains(&id) {
        return EntryState::Applied;
    }
    visiting.push(id);
    let undone = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Undo && e.undoes.as_deref() == Some(id))
        .any(|undo| state_of(entries, &undo.id, visiting) == EntryState::Applied);
    visiting.pop();
    if undone {
        EntryState::Undone
    } else {
        EntryState::Applied
    }
}

pub fn prune(dir: &Path, backup_dir: &Path, retention_days: u32, now: DateTime<Utc>) -> usize {
    if retention_days == 0 {
        return 0;
    }
    let cutoff = now - Duration::days(i64::from(retention_days));
    let mut pruned = 0;
    for entry in load_all(dir) {
        let created = match DateTime::parse_from_rfc3339(&entry.created_at) {
            Ok(created) => created.with_timezone(&Utc),
            Err(e) => {
                log::warn!("change log: entry {} has an unreadable created_at: {}", entry.id, e);
                continue;
            }
        };
        if created >= cutoff {
            continue;
        }
        if let Some(backup) = entry.secret.as_ref().and_then(|s| s.backup.as_deref()) {
            remove_file(&backup_path(backup_dir, backup));
        }
        remove_file(&dir.join(format!("{}.json", entry.id)));
        pruned += 1;
    }
    pruned
}

fn backup_path(backup_dir: &Path, backup: &str) -> PathBuf {
    let name = Path::new(backup).file_name().unwrap_or_default();
    backup_dir.join(name)
}

fn remove_file(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("change log: cannot delete {}: {}", path.display(), e),
    }
}
