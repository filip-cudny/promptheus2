use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::services::change_log::{
    entry_state, ChangeLogEntry, Effect, EntryState, LoggedOperation, Target,
};
use crate::services::env_resolve::env_ref_names;
use crate::services::secrets::REDACTED;

const DEFAULT_LIMIT: u64 = 20;
const MAX_LIMIT: u64 = 100;
const API_KEY_FIELD: &str = "api_key";

#[derive(Debug, Default, Deserialize)]
pub struct Filters {
    pub name: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub key: Option<String>,
    pub op: Option<String>,
    pub text: Option<String>,
    pub host: Option<String>,
    pub limit: Option<u64>,
}

struct Matcher<'a> {
    since: Option<DateTime<FixedOffset>>,
    until: Option<DateTime<FixedOffset>>,
    key: Option<&'a str>,
    op: Option<&'a str>,
    text: Option<String>,
    host: Option<&'a str>,
}

fn parse_time(field: &str, value: Option<&str>) -> Result<Option<DateTime<FixedOffset>>, String> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map_err(|e| format!("`{field}` is not an RFC 3339 time ('{value}'): {e}"))
        })
        .transpose()
}

impl<'a> Matcher<'a> {
    fn new(filters: &'a Filters) -> Result<Self, String> {
        Ok(Self {
            since: parse_time("since", filters.since.as_deref())?,
            until: parse_time("until", filters.until.as_deref())?,
            key: filters.key.as_deref(),
            op: filters.op.as_deref(),
            text: filters.text.as_deref().map(str::to_lowercase),
            host: filters.host.as_deref(),
        })
    }

    fn matches(&self, entry: &ChangeLogEntry) -> bool {
        self.in_time(entry)
            && self.host.is_none_or(|host| entry.host == host)
            && self.op.is_none_or(|op| entry.operations.iter().any(|o| o.op == op))
            && self.key.is_none_or(|key| entry.operations.iter().any(|o| touches(o, key)))
            && self.text.as_deref().is_none_or(|text| mentions(entry, text))
    }

    fn in_time(&self, entry: &ChangeLogEntry) -> bool {
        if self.since.is_none() && self.until.is_none() {
            return true;
        }
        let Ok(created) = DateTime::parse_from_rfc3339(&entry.created_at) else {
            return false;
        };
        self.since.is_none_or(|since| created >= since) && self.until.is_none_or(|until| created <= until)
    }
}

fn touches(operation: &LoggedOperation, key: &str) -> bool {
    operation
        .effects
        .iter()
        .any(|effect| matches!(&effect.target, Target::Setting { path } if path.starts_with(key)))
}

fn mentions(entry: &ChangeLogEntry, text: &str) -> bool {
    let contains = |field: &str| field.to_lowercase().contains(text);
    entry.request.as_deref().is_some_and(contains)
        || entry.reply.as_deref().is_some_and(contains)
        || entry.operations.iter().any(|operation| contains(&operation.label))
}

fn is_literal_key(value: &str) -> bool {
    let names = env_ref_names(value);
    let single_reference = names.len() == 1 && value == format!("${{{}}}", names[0]);
    !value.is_empty() && !single_reference
}

fn redact_api_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(name, child)| {
                    let literal = name == API_KEY_FIELD && child.as_str().is_some_and(is_literal_key);
                    let child = if literal { json!(REDACTED) } else { redact_api_keys(child) };
                    (name.clone(), child)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_api_keys).collect()),
        other => other.clone(),
    }
}

fn redacted_value(path: &str, value: &Option<Value>) -> Value {
    match value {
        Some(Value::String(text)) if path.ends_with(".api_key") && is_literal_key(text) => json!(REDACTED),
        Some(value) => redact_api_keys(value),
        None => Value::Null,
    }
}

fn effect_json(effect: &Effect, with_file_content: bool) -> Value {
    match &effect.target {
        Target::Setting { path } => json!({
            "target": "setting",
            "path": path,
            "before": redacted_value(path, &effect.before),
            "after": redacted_value(path, &effect.after),
        }),
        Target::File { path } if with_file_content => json!({
            "target": "file",
            "path": path,
            "before": redacted_value(path, &effect.before),
            "after": redacted_value(path, &effect.after),
        }),
        Target::File { path } => json!({"target": "file", "path": path}),
        Target::Secret { name } => json!({"target": "secret", "name": name}),
    }
}

fn item(entries: &[ChangeLogEntry], entry: &ChangeLogEntry, with_file_content: bool) -> Value {
    let state = match entry_state(entries, &entry.id) {
        EntryState::Applied => "applied",
        EntryState::Undone => "undone",
    };
    let operations: Vec<Value> = entry
        .operations
        .iter()
        .map(|operation| {
            json!({
                "label": operation.label,
                "op": operation.op,
                "effects": operation.effects.iter().map(|e| effect_json(e, with_file_content)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut item = json!({
        "id": entry.id,
        "created_at": entry.created_at,
        "host": entry.host,
        "kind": entry.kind,
        "state": state,
        "request": entry.request,
        "reply": entry.reply,
        "undoes": entry.undoes,
        "operations": operations,
    });
    if let Some(secret) = &entry.secret {
        item["secret"] = json!({"name": secret.name});
    }
    item
}

/// `entries` is `load_all` order: oldest first.
pub fn read(entries: &[ChangeLogEntry], filters: &Filters) -> Result<Value, String> {
    if let Some(id) = &filters.name {
        let entry = entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| format!("change log entry '{id}' not found"))?;
        return Ok(item(entries, entry, true));
    }
    let matcher = Matcher::new(filters)?;
    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
    let matching: Vec<&ChangeLogEntry> = entries.iter().rev().filter(|entry| matcher.matches(entry)).collect();
    let returned = &matching[..matching.len().min(limit)];
    let more = matching.len() - returned.len();
    let mut result = json!({
        "entries": returned.iter().map(|entry| item(entries, entry, false)).collect::<Vec<_>>(),
        "more": more,
    });
    if let Some(oldest) = returned.last().filter(|_| more > 0) {
        result["hint"] = json!(format!(
            "{more} more matching entries are older. Narrow by `until` (inclusive; the oldest returned is {}), `key` or `since`.",
            oldest.created_at
        ));
    }
    Ok(result)
}
