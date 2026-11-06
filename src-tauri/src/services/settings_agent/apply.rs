use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::models::settings::Settings;
use crate::services::change_log::{
    entry_state, load_all, new_id, write_entry, ChangeLogEntry, ChangeLogError, Effect, EntryKind,
    EntryState, LoggedOperation, Target, TestResult,
};
use crate::services::config::ConfigService;
use crate::services::secrets::{self, SecretError};

use super::plan::{mismatches, parse_path, put_at, stale_rows, Current, Plan, PlannedRow};

const SETTINGS_FILE: &str = "settings.json";
const SKILLS_DIR: &str = "skills";

pub struct Context<'a> {
    pub config: &'a mut ConfigService,
    pub config_dir: &'a Path,
    pub log_dir: &'a Path,
    pub host: &'a str,
    pub request: Option<String>,
    pub reply: Option<String>,
    pub tests: BTreeMap<usize, TestResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("rows changed since the proposal: {}", row_numbers(.0))]
    Stale(Vec<usize>),
    #[error("row {} failed: {message}", .row + 1)]
    WriteFailed { row: usize, message: String },
    #[error("change log entry '{0}' not found")]
    NotFound(String),
    #[error(transparent)]
    ChangeLog(#[from] ChangeLogError),
    #[error(transparent)]
    Secret(#[from] SecretError),
}

fn row_numbers(rows: &[usize]) -> String {
    rows.iter().map(|row| (row + 1).to_string()).collect::<Vec<_>>().join(", ")
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conflict {
    pub row: usize,
    pub target: Target,
    pub expected_after: Option<Value>,
    pub current: Option<Value>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UndoOutcome {
    Undone { entry_id: String },
    Conflicts { conflicts: Vec<Conflict> },
    Skipped { held_by: String },
}

#[derive(Default)]
struct Snapshots {
    settings: Option<Vec<u8>>,
    files: Vec<(String, Option<Vec<u8>>)>,
}

impl Snapshots {
    fn settings(&mut self, config_dir: &Path) -> Result<(), String> {
        if self.settings.is_none() {
            let bytes = std::fs::read(config_dir.join(SETTINGS_FILE))
                .map_err(|e| format!("cannot read settings.json: {e}"))?;
            self.settings = Some(bytes);
        }
        Ok(())
    }

    fn file(&mut self, config_dir: &Path, path: &str) -> Result<(), String> {
        if self.files.iter().any(|(known, _)| known == path) {
            return Ok(());
        }
        let bytes = match std::fs::read(config_dir.join(path)) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("cannot read {path}: {e}")),
        };
        self.files.push((path.to_string(), bytes));
        Ok(())
    }
}

fn write_file(config_dir: &Path, path: &str, content: Option<&[u8]>) -> Result<(), String> {
    let full = config_dir.join(path);
    match content {
        Some(bytes) => {
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {path}: {e}"))?;
            }
            std::fs::write(&full, bytes).map_err(|e| format!("cannot write {path}: {e}"))
        }
        None => {
            match std::fs::remove_file(&full) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("cannot delete {path}: {e}")),
            }
            remove_empty_skill_dirs(config_dir, &full);
            Ok(())
        }
    }
}

fn remove_empty_skill_dirs(config_dir: &Path, removed: &Path) {
    let skills = config_dir.join(SKILLS_DIR);
    let mut dir = removed.parent();
    while let Some(current) = dir.filter(|d| d.starts_with(&skills) && *d != skills) {
        if std::fs::remove_dir(current).is_err() {
            break;
        }
        dir = current.parent();
    }
}

fn write_setting_effects(
    ctx: &mut Context,
    effects: &[&Effect],
    snapshots: &mut Snapshots,
) -> Result<(), String> {
    snapshots.settings(ctx.config_dir)?;
    let _ = ctx.config.reload_if_changed();
    let mut json = serde_json::to_value(ctx.config.settings()).map_err(|e| e.to_string())?;
    for effect in effects {
        let Target::Setting { path } = &effect.target else {
            continue;
        };
        put_at(&mut json, &parse_path(path), effect.after.clone())?;
    }
    let settings: Settings = serde_json::from_value(json).map_err(|e| e.to_string())?;
    ctx.config
        .try_mutate(|config| {
            config.replace_settings(settings);
            Ok(())
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn write_row(ctx: &mut Context, operation: &LoggedOperation, snapshots: &mut Snapshots) -> Result<(), String> {
    let settings: Vec<&Effect> = operation
        .effects
        .iter()
        .filter(|effect| matches!(effect.target, Target::Setting { .. }))
        .collect();
    if !settings.is_empty() {
        write_setting_effects(ctx, &settings, snapshots)?;
    }
    for effect in &operation.effects {
        let Target::File { path } = &effect.target else {
            continue;
        };
        snapshots.file(ctx.config_dir, path)?;
        let content = match &effect.after {
            None => None,
            Some(Value::String(text)) => Some(text.as_bytes()),
            Some(_) => return Err(format!("{path} content is not text")),
        };
        write_file(ctx.config_dir, path, content)?;
    }
    Ok(())
}

fn restore(ctx: &mut Context, snapshots: Snapshots) {
    for (path, bytes) in snapshots.files.iter().rev() {
        if let Err(e) = write_file(ctx.config_dir, path, bytes.as_deref()) {
            log::error!("restore of {path} failed: {e}");
        }
    }
    if let Some(bytes) = snapshots.settings {
        if let Err(e) = std::fs::write(ctx.config_dir.join(SETTINGS_FILE), bytes) {
            log::error!("restore of settings.json failed: {e}");
        }
        if let Err(e) = ctx.config.reload() {
            log::error!("reload after restore failed: {e}");
        }
    }
}

fn write_rows(ctx: &mut Context, rows: &[PlannedRow]) -> Result<(), ApplyError> {
    let mut snapshots = Snapshots::default();
    for planned in rows {
        if let Err(message) = write_row(ctx, &planned.operation, &mut snapshots) {
            log::warn!("row {} failed, restoring written targets: {message}", planned.row + 1);
            restore(ctx, snapshots);
            return Err(ApplyError::WriteFailed { row: planned.row, message });
        }
    }
    Ok(())
}

fn write_log_entry(
    ctx: &Context,
    kind: EntryKind,
    undoes: Option<String>,
    operations: Vec<LoggedOperation>,
) -> Result<String, ApplyError> {
    let now = Utc::now();
    let id = new_id(now, ctx.host);
    let entry = ChangeLogEntry {
        id: id.clone(),
        created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        host: ctx.host.to_string(),
        kind,
        request: ctx.request.clone(),
        reply: ctx.reply.clone(),
        operations,
        undoes,
        secret: None,
    };
    write_entry(ctx.log_dir, &entry)?;
    Ok(id)
}

pub fn apply(plan: &Plan, current: &Current, ctx: &mut Context) -> Result<String, ApplyError> {
    let stale = stale_rows(plan, current);
    if !stale.is_empty() {
        return Err(ApplyError::Stale(stale));
    }
    write_rows(ctx, &plan.rows)?;
    let operations = plan
        .rows
        .iter()
        .map(|planned| LoggedOperation {
            test: ctx.tests.get(&planned.row).cloned(),
            ..planned.operation.clone()
        })
        .collect();
    let id = write_log_entry(ctx, EntryKind::Card, None, operations)?;
    log::info!("applied {} rows as entry {id}", plan.rows.len());
    Ok(id)
}

fn invert(operations: &[LoggedOperation]) -> Vec<PlannedRow> {
    operations
        .iter()
        .enumerate()
        .rev()
        .map(|(row, operation)| PlannedRow {
            row,
            operation: LoggedOperation {
                label: operation.label.clone(),
                op: operation.op.clone(),
                effects: operation
                    .effects
                    .iter()
                    .rev()
                    .map(|effect| Effect {
                        target: effect.target.clone(),
                        before: effect.after.clone(),
                        after: effect.before.clone(),
                    })
                    .collect(),
                test: None,
            },
        })
        .collect()
}

pub fn undo(entry_id: &str, force: bool, ctx: &mut Context) -> Result<UndoOutcome, ApplyError> {
    let entries = load_all(ctx.log_dir);
    let entry = entries
        .iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| ApplyError::NotFound(entry_id.to_string()))?;
    if entry.kind == EntryKind::Secret || entry.secret.is_some() {
        return Ok(match secrets::undo_secret(ctx.config_dir, ctx.host, entry)? {
            secrets::UndoOutcome::Undone { entry_id } => UndoOutcome::Undone { entry_id },
            secrets::UndoOutcome::Skipped { held_by } => UndoOutcome::Skipped { held_by },
        });
    }
    if entry_state(&entries, entry_id) == EntryState::Undone {
        log::debug!("undo of entry {entry_id} that is already undone");
    }
    let rows = invert(&entry.operations);
    let _ = ctx.config.reload_if_changed();
    let settings = ctx.config.settings().clone();
    let current = Current { settings: &settings, config_dir: ctx.config_dir };
    let conflicts: Vec<Conflict> = mismatches(&rows, &current)
        .into_iter()
        .map(|m| Conflict {
            row: m.row,
            target: m.effect.target.clone(),
            expected_after: m.effect.before.clone(),
            current: m.current,
        })
        .collect();
    if !conflicts.is_empty() && !force {
        log::info!("undo of {entry_id} blocked by {} conflicts", conflicts.len());
        return Ok(UndoOutcome::Conflicts { conflicts });
    }
    write_rows(ctx, &rows)?;
    let operations = rows.into_iter().map(|planned| planned.operation).collect();
    let id = write_log_entry(ctx, EntryKind::Undo, Some(entry_id.to_string()), operations)?;
    log::info!("undid entry {entry_id} as {id}");
    Ok(UndoOutcome::Undone { entry_id: id })
}
