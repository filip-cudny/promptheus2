use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::Utc;

use crate::models::settings::Settings;
use crate::services::change_log::{
    card_backup_path, change_log_dir, new_id, secret_backup_dir, write_entry, ChangeLogEntry,
    ChangeLogError, EntryKind, SecretChange,
};
use crate::services::env_resolve::{
    env_ref_names, env_value, override_for, override_values, set_override,
};
use crate::services::missing_env::collect_missing;

const ENV_FILE_NAME: &str = ".env";
const BEFORE_PREFIX: &str = "before__";
const AFTER_PREFIX: &str = "after__";
pub(crate) const REDACTED: &str = "[redacted]";

static ENV_WRITE_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn lock_env_writes() -> MutexGuard<'static, ()> {
    ENV_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("invalid variable name '{0}'")]
    InvalidName(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ChangeLog(#[from] ChangeLogError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum UndoOutcome {
    Undone { entry_id: String },
    Skipped { held_by: String },
}

pub fn validate_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn env_path(config_dir: &Path) -> PathBuf {
    config_dir.join(ENV_FILE_NAME)
}

fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn line_defines(line: &str, name: &str) -> bool {
    let line = line.trim_start();
    let line = match line.strip_prefix("export") {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim_start(),
        _ => line,
    };
    line.strip_prefix(name)
        .is_some_and(|rest| rest.trim_start().starts_with('='))
}

fn write_private(path: &Path, content: impl AsRef<[u8]>) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_ref())
}

fn write_env_var(config_dir: &Path, name: &str, value: Option<&str>) -> std::io::Result<()> {
    let path = env_path(config_dir);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        if !line_defines(line, name) {
            lines.push(line.to_string());
        } else if let (Some(value), false) = (value, replaced) {
            lines.push(format!("{name}={}", quote(value)));
            replaced = true;
        }
    }
    if let (Some(value), false) = (value, replaced) {
        lines.push(format!("{name}={}", quote(value)));
    }
    let mut content = lines.join("\n");
    if !lines.is_empty() {
        content.push('\n');
    }
    write_private(&path, &content)
}

fn env_file_pairs(path: &Path) -> Vec<(String, String)> {
    match dotenvy::from_path_iter(path) {
        Ok(iter) => iter.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

fn env_file_value(config_dir: &Path, name: &str) -> Option<String> {
    env_file_pairs(&env_path(config_dir))
        .into_iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

pub fn current_value(config_dir: &Path, name: &str) -> Option<String> {
    match override_for(name) {
        Some(value) => value,
        None => env_file_value(config_dir, name).or_else(|| std::env::var(name).ok()),
    }
}

fn non_empty_value(config_dir: &Path, name: &str) -> Option<String> {
    current_value(config_dir, name).filter(|value| !value.is_empty())
}

pub(crate) fn copy_renamed_values(config_dir: &Path, settings: &Settings) -> Vec<String> {
    let _guard = lock_env_writes();
    let mut filled = Vec::new();
    for var in &settings.env_vars {
        if non_empty_value(config_dir, &var.name).is_some() {
            continue;
        }
        let Some((old, value)) = var
            .renamed_from
            .iter()
            .find_map(|old| non_empty_value(config_dir, old).map(|value| (old, value)))
        else {
            continue;
        };
        if let Err(e) = write_env_var(config_dir, &var.name, Some(&value)) {
            log::warn!("env var '{}' not copied from '{old}': {e}", var.name);
            continue;
        }
        set_override(&var.name, Some(value));
        log::info!("env var '{}' copied from '{old}'", var.name);
        filled.push(var.name.clone());
    }
    filled
}

fn write_backup_file(config_dir: &Path, entry_id: &str, content: &str) -> std::io::Result<String> {
    let dir = secret_backup_dir(config_dir);
    std::fs::create_dir_all(&dir)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    write_private(&card_backup_path(&dir, entry_id), content)?;
    Ok(format!("{entry_id}.env"))
}

fn write_backup(config_dir: &Path, entry_id: &str, name: &str, value: &str) -> std::io::Result<String> {
    write_backup_file(config_dir, entry_id, &format!("{name}={}\n", quote(value)))
}

fn read_backup(config_dir: &Path, backup: &str, name: &str) -> Option<String> {
    let file_name = Path::new(backup).file_name()?;
    let path = secret_backup_dir(config_dir).join(file_name);
    env_file_pairs(&path)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

pub(crate) type NameValues = BTreeMap<String, Option<String>>;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CardBackup {
    pub before: BTreeMap<String, String>,
    pub after: BTreeMap<String, String>,
}

pub(crate) fn set_env_value(config_dir: &Path, name: &str, value: Option<&str>) -> std::io::Result<()> {
    write_env_var(config_dir, name, value)?;
    set_override(name, value.map(str::to_string));
    Ok(())
}

pub(crate) fn read_env_bytes(config_dir: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(env_path(config_dir)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn restore_env_bytes(config_dir: &Path, bytes: Option<&[u8]>) -> std::io::Result<()> {
    match bytes {
        Some(bytes) => write_private(&env_path(config_dir), bytes),
        None => match std::fs::remove_file(env_path(config_dir)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

fn backup_lines<'a>(prefix: &'a str, values: &'a NameValues) -> impl Iterator<Item = String> + 'a {
    values
        .iter()
        .filter_map(move |(name, value)| value.as_ref().map(|value| format!("{prefix}{name}={}\n", quote(value))))
}

pub(crate) fn write_card_backup(
    config_dir: &Path,
    entry_id: &str,
    before: &NameValues,
    after: &NameValues,
) -> std::io::Result<()> {
    let content: String = backup_lines(BEFORE_PREFIX, before)
        .chain(backup_lines(AFTER_PREFIX, after))
        .collect();
    write_backup_file(config_dir, entry_id, &content).map(|_| ())
}

pub(crate) fn read_card_backup(config_dir: &Path, entry_id: &str) -> Option<CardBackup> {
    let path = card_backup_path(&secret_backup_dir(config_dir), entry_id);
    if !path.is_file() {
        return None;
    }
    let mut backup = CardBackup::default();
    for (key, value) in env_file_pairs(&path) {
        if let Some(name) = key.strip_prefix(BEFORE_PREFIX) {
            backup.before.insert(name.to_string(), value);
        } else if let Some(name) = key.strip_prefix(AFTER_PREFIX) {
            backup.after.insert(name.to_string(), value);
        }
    }
    Some(backup)
}

pub(crate) fn remove_card_backup(config_dir: &Path, entry_id: &str) {
    let path = card_backup_path(&secret_backup_dir(config_dir), entry_id);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("cannot delete {}: {e}", path.display()),
    }
}

fn backup_values(config_dir: &Path) -> Vec<String> {
    let dir = secret_backup_dir(config_dir);
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(read_dir) => read_dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", dir.display());
            return Vec::new();
        }
    };
    read_dir
        .flatten()
        .flat_map(|item| env_file_pairs(&item.path()))
        .map(|(_, value)| value)
        .collect()
}

fn apply_secret(
    config_dir: &Path,
    host: &str,
    name: &str,
    value: Option<&str>,
    kind: EntryKind,
    undoes: Option<String>,
) -> Result<String, SecretError> {
    if !validate_name(name) {
        return Err(SecretError::InvalidName(name.to_string()));
    }
    let _guard = lock_env_writes();
    let now = Utc::now();
    let id = new_id(now, host);
    let previous = current_value(config_dir, name);
    let backup = previous
        .as_deref()
        .map(|previous| write_backup(config_dir, &id, name, previous))
        .transpose()?;
    write_env_var(config_dir, name, value)?;
    set_override(name, value.map(str::to_string));
    let entry = ChangeLogEntry {
        id: id.clone(),
        created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        host: host.to_string(),
        kind,
        request: None,
        reply: None,
        operations: Vec::new(),
        undoes,
        secret: Some(SecretChange {
            name: name.to_string(),
            backup,
            had_previous: previous.is_some(),
        }),
    };
    write_entry(&change_log_dir(config_dir), &entry)?;
    log::info!("secret '{name}' {} (entry {id})", if value.is_some() { "saved" } else { "removed" });
    Ok(id)
}

pub fn save_secret(
    config_dir: &Path,
    host: &str,
    name: &str,
    value: &str,
) -> Result<String, SecretError> {
    apply_secret(config_dir, host, name, Some(value), EntryKind::Secret, None)
}

pub fn undo_secret(
    config_dir: &Path,
    local_host: &str,
    entry: &ChangeLogEntry,
) -> Result<UndoOutcome, SecretError> {
    let Some(secret) = &entry.secret else {
        return Ok(UndoOutcome::Skipped { held_by: entry.host.clone() });
    };
    let restore = if secret.had_previous {
        let previous = secret
            .backup
            .as_deref()
            .and_then(|backup| read_backup(config_dir, backup, &secret.name));
        match previous {
            Some(previous) => Some(previous),
            None => {
                log::info!("secret '{}' undo skipped: backup is held by {}", secret.name, entry.host);
                return Ok(UndoOutcome::Skipped { held_by: entry.host.clone() });
            }
        }
    } else {
        None
    };
    let entry_id = apply_secret(
        config_dir,
        local_host,
        &secret.name,
        restore.as_deref(),
        EntryKind::Undo,
        Some(entry.id.clone()),
    )?;
    Ok(UndoOutcome::Undone { entry_id })
}

pub struct Redactor {
    values: Vec<String>,
}

fn longest_first(mut values: Vec<String>) -> Vec<String> {
    values.retain(|value| !value.is_empty());
    values.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    values.dedup();
    values
}

#[derive(Default)]
struct ValueSources(BTreeMap<String, Vec<Option<String>>>);

impl ValueSources {
    fn add(&mut self, value: String, name: Option<&str>) {
        self.0.entry(value).or_default().push(name.map(str::to_string));
    }

    fn secret_values(self, plain_names: &BTreeSet<&str>) -> Vec<String> {
        self.0
            .into_iter()
            .filter(|(_, sources)| {
                !sources
                    .iter()
                    .all(|source| source.as_deref().is_some_and(|name| plain_names.contains(name)))
            })
            .map(|(value, _)| value)
            .collect()
    }
}

fn candidate_names(settings: &Settings, env_pairs: &[(String, String)], referenced: &[String]) -> BTreeSet<String> {
    env_pairs
        .iter()
        .map(|(name, _)| name.clone())
        .chain(settings.env_vars.iter().map(|var| var.name.clone()))
        .chain(referenced.iter().cloned())
        .collect()
}

impl Redactor {
    /// A value is exempt from redaction only when every source of it is a variable listed in
    /// `settings.env_vars` with `secret: false`.
    pub fn new(config_dir: &Path, settings: &Settings) -> Self {
        let env_pairs = env_file_pairs(&env_path(config_dir));
        let referenced: Vec<String> =
            collect_missing(settings, |_| false).into_iter().map(|reference| reference.name).collect();
        let mut sources = ValueSources::default();
        for (name, value) in &env_pairs {
            sources.add(value.clone(), Some(name));
        }
        let mut unattributed = override_values();
        for name in candidate_names(settings, &env_pairs, &referenced) {
            if let Some(Some(value)) = override_for(&name) {
                if let Some(at) = unattributed.iter().position(|held| *held == value) {
                    unattributed.swap_remove(at);
                }
                sources.add(value, Some(&name));
            }
        }
        for value in unattributed.into_iter().chain(backup_values(config_dir)) {
            sources.add(value, None);
        }
        for name in &referenced {
            if let Some(value) = env_value(name) {
                sources.add(value, Some(name));
            }
        }
        for key in settings
            .models
            .iter()
            .filter_map(|model| model.api_key.clone())
            .filter(|key| env_ref_names(key).is_empty())
        {
            sources.add(key, None);
        }
        let plain_names: BTreeSet<&str> = settings
            .env_vars
            .iter()
            .filter(|var| !var.secret)
            .map(|var| var.name.as_str())
            .collect();
        Self { values: longest_first(sources.secret_values(&plain_names)) }
    }

    pub fn with_values(self, extra: impl IntoIterator<Item = String>) -> Self {
        Self { values: longest_first(self.values.into_iter().chain(extra).collect()) }
    }

    pub fn redact(&self, text: &str) -> String {
        self.values
            .iter()
            .fold(text.to_string(), |text, value| text.replace(value.as_str(), REDACTED))
    }

    pub fn redact_settings(&self, settings: &Settings) -> serde_json::Value {
        let mut json = serde_json::to_value(settings).unwrap_or(serde_json::Value::Null);
        if let Some(models) = json.get_mut("models").and_then(|m| m.as_array_mut()) {
            for model in models {
                let literal = model
                    .get("api_key")
                    .and_then(|k| k.as_str())
                    .is_some_and(|k| env_ref_names(k).is_empty());
                if literal {
                    model["api_key"] = serde_json::Value::String(REDACTED.to_string());
                }
            }
        }
        self.redact_strings(&mut json);
        json
    }

    fn redact_strings(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => *s = self.redact(s),
            serde_json::Value::Array(items) => items.iter_mut().for_each(|v| self.redact_strings(v)),
            serde_json::Value::Object(map) => map.values_mut().for_each(|v| self.redact_strings(v)),
            _ => {}
        }
    }
}
