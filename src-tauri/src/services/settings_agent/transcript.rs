use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::settings::ModelParameters;

const SHORT_ID_LEN: usize = 8;

#[derive(Debug, Serialize)]
pub struct Transcript {
    pub id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub model_id: Option<String>,
    pub parameters: ModelParameters,
    pub user_text: String,
    pub iterations: Vec<Iteration>,
    pub end: Option<End>,
}

#[derive(Debug, Serialize)]
pub struct Iteration {
    pub started_at: String,
    pub millis: u64,
    pub text: String,
    pub thinking: Option<String>,
    pub reasoning_items: usize,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub result: Option<String>,
    pub error: Option<String>,
    pub millis: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum End {
    Reply { reply: String },
    Error { error: String },
    Cancelled,
    LoopLimit { error: String },
}

pub fn timestamp(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl Transcript {
    pub fn start(user_text: &str, now: DateTime<Utc>) -> Self {
        let uuid = uuid::Uuid::new_v4().simple().to_string();
        Self {
            id: format!("{}-{}", now.format("%Y%m%dT%H%M%SZ"), &uuid[..SHORT_ID_LEN]),
            started_at: timestamp(now),
            finished_at: None,
            model_id: None,
            parameters: ModelParameters::default(),
            user_text: user_text.to_string(),
            iterations: Vec::new(),
            end: None,
        }
    }

    pub fn finish(&mut self, end: End, now: DateTime<Utc>, redact: impl Fn(&str) -> String) {
        self.finished_at = Some(timestamp(now));
        self.end = Some(match end {
            End::Error { error } => End::Error { error: redact(&error) },
            End::LoopLimit { error } => End::LoopLimit { error: redact(&error) },
            other => other,
        });
    }
}

pub fn write(dir: &Path, transcript: &Transcript) -> crate::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", transcript.id));
    let file = OpenOptions::new().write(true).create_new(true).open(&path)?;
    serde_json::to_writer_pretty(file, transcript)?;
    Ok(path)
}

#[derive(Deserialize)]
struct StartedAt {
    started_at: String,
}

fn started_at(path: &Path) -> Result<DateTime<Utc>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let parsed: StartedAt = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    DateTime::parse_from_rfc3339(&parsed.started_at)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| e.to_string())
}

pub fn prune(dir: &Path, retention_days: u32, now: DateTime<Utc>) -> usize {
    if retention_days == 0 {
        return 0;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(e) => {
            log::warn!("settings agent transcripts: cannot read {}: {}", dir.display(), e);
            return 0;
        }
    };
    let cutoff = now - Duration::days(i64::from(retention_days));
    let mut pruned = 0;
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let started = match started_at(&path) {
            Ok(started) => started,
            Err(e) => {
                log::warn!("settings agent transcripts: skipping {}: {}", path.display(), e);
                continue;
            }
        };
        if started >= cutoff {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => pruned += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("settings agent transcripts: cannot delete {}: {}", path.display(), e),
        }
    }
    pruned
}
