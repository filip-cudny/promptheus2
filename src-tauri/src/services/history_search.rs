use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::{Deserialize, Serialize};

use crate::models::history::{HistoryEntry, HistoryEntryType};
use crate::services::sqlite_history::SqliteHistoryService;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const FTS_MIN_QUERY_LEN: usize = 3;
const FTS_CANDIDATE_LIMIT: usize = 1000;
const RECENCY_TAU_DAYS: f64 = 30.0;
const RECENCY_FLOOR: f64 = 0.3;
const PROXIMITY_WINDOW_CHARS: u32 = 50;
const PROXIMITY_BONUS: f32 = 1.5;
const TITLE_EARLY_EXIT_SCORE: i32 = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryTypeFilter {
    All,
    Chat,
    QuickAction,
    Speech,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatusFilter {
    All,
    Success,
    Error,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub query: String,
    pub type_filter: HistoryTypeFilter,
    pub status_filter: HistoryStatusFilter,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub date_from: Option<i64>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    Title,
    SkillName,
    InputContent,
    OutputContent,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldMatch {
    pub field: SearchField,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub entry: HistoryEntry,
    pub matches: Vec<FieldMatch>,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
}

const FIELD_WEIGHTS: &[(SearchField, f32)] = &[
    (SearchField::Title, 1.00),
    (SearchField::SkillName, 0.75),
    (SearchField::InputContent, 0.50),
    (SearchField::OutputContent, 0.25),
];

const AVG_LEN_FALLBACK: &[(SearchField, f32)] = &[
    (SearchField::Title, 50.0),
    (SearchField::SkillName, 15.0),
    (SearchField::InputContent, 200.0),
    (SearchField::OutputContent, 2000.0),
];

fn is_transcription(entry: &HistoryEntry) -> bool {
    entry.quick_action
        && entry.entry_type == HistoryEntryType::Speech
        && entry.skill_name.is_none()
}

fn matches_type_filter(entry: &HistoryEntry, filter: HistoryTypeFilter) -> bool {
    match filter {
        HistoryTypeFilter::All => true,
        HistoryTypeFilter::Chat => !entry.quick_action,
        HistoryTypeFilter::Speech => is_transcription(entry),
        HistoryTypeFilter::QuickAction => entry.quick_action && !is_transcription(entry),
    }
}

fn matches_status_filter(entry: &HistoryEntry, filter: HistoryStatusFilter) -> bool {
    match filter {
        HistoryStatusFilter::All => true,
        HistoryStatusFilter::Success => entry.success,
        HistoryStatusFilter::Error => !entry.success,
    }
}

fn field_value<'a>(entry: &'a HistoryEntry, field: SearchField) -> Option<&'a str> {
    let value = match field {
        SearchField::Title => entry.title.as_deref(),
        SearchField::SkillName => entry.skill_name.as_deref(),
        SearchField::InputContent => entry
            .input_content_rendered
            .as_deref()
            .or(Some(entry.input_content.as_str())),
        SearchField::OutputContent => entry
            .output_content_rendered
            .as_deref()
            .or(entry.output_content.as_deref()),
    };
    value.filter(|s| !s.is_empty())
}

fn sort_key(entry: &HistoryEntry) -> &str {
    entry
        .updated_at
        .as_deref()
        .or(entry.created_at.as_deref())
        .unwrap_or(entry.timestamp.as_str())
}

fn entry_timestamp(entry: &HistoryEntry) -> Option<i64> {
    let raw = entry
        .updated_at
        .as_deref()
        .or(entry.created_at.as_deref())
        .unwrap_or(entry.timestamp.as_str());
    let naive = NaiveDateTime::parse_from_str(raw, TIMESTAMP_FORMAT).ok()?;
    let local_dt = Local.from_local_datetime(&naive).single()?;
    Some(local_dt.with_timezone(&Utc).timestamp())
}

fn recency_multiplier(entry: &HistoryEntry, now: i64) -> f32 {
    let Some(ts) = entry_timestamp(entry) else {
        return 1.0;
    };
    let age_secs = (now - ts).max(0) as f64;
    let age_days = age_secs / 86_400.0;
    let raw = (-age_days / RECENCY_TAU_DAYS).exp();
    raw.max(RECENCY_FLOOR) as f32
}

fn length_norm(field_len: usize, avg_len: f32) -> f32 {
    let ratio = (field_len.max(1) as f32) / avg_len.max(1.0);
    1.0 / (1.0 + ratio.ln().max(0.0))
}

fn proximity_bonus(indices: &[u32]) -> f32 {
    if indices.len() < 2 {
        return 1.0;
    }
    let span = indices.last().copied().unwrap_or(0) - indices.first().copied().unwrap_or(0);
    if span <= PROXIMITY_WINDOW_CHARS {
        PROXIMITY_BONUS
    } else {
        1.0
    }
}

fn split_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

fn escape_fts_token(token: &str) -> String {
    let escaped = token.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn build_fts_query(tokens: &[String]) -> Option<String> {
    let parts: Vec<String> = tokens
        .iter()
        .filter(|t| t.chars().count() >= 2)
        .map(|t| escape_fts_token(t))
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts.join(" "))
}

fn collect_token_indices(text: &str, token: &str) -> Vec<u32> {
    let token_lower: String = token.to_lowercase();
    let token_chars: Vec<char> = token_lower.chars().collect();
    if token_chars.is_empty() {
        return Vec::new();
    }
    let text_lower: Vec<char> = text.chars().flat_map(|c| c.to_lowercase()).collect();
    if text_lower.len() < token_chars.len() {
        return Vec::new();
    }

    let mut indices: Vec<u32> = Vec::new();
    let mut i = 0;
    while i + token_chars.len() <= text_lower.len() {
        if text_lower[i..i + token_chars.len()] == token_chars[..] {
            for k in 0..token_chars.len() {
                indices.push((i + k) as u32);
            }
            i += token_chars.len();
        } else {
            i += 1;
        }
    }
    indices
}

pub struct HistorySearch {
    matcher: Matcher,
}

impl HistorySearch {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    pub fn run(&mut self, history: &SqliteHistoryService, query: &SearchQuery) -> SearchResponse {
        let trimmed_query = query.query.trim();

        if trimmed_query.is_empty() {
            return self.run_empty_query(history, query);
        }

        let tokens = split_tokens(trimmed_query);
        let max_token_len = tokens.iter().map(|t| t.chars().count()).max().unwrap_or(0);

        if max_token_len >= FTS_MIN_QUERY_LEN {
            if let Some(response) = self.run_fts(history, query, &tokens) {
                if !response.results.is_empty() {
                    return response;
                }
            }
        }

        self.run_with_query_nucleo(history, query, trimmed_query, &tokens)
    }

    fn run_empty_query(
        &self,
        history: &SqliteHistoryService,
        query: &SearchQuery,
    ) -> SearchResponse {
        let entries = history.get_history();
        let mut filtered: Vec<&HistoryEntry> = entries
            .iter()
            .filter(|e| matches_type_filter(e, query.type_filter))
            .filter(|e| matches_status_filter(e, query.status_filter))
            .collect();

        if !query.skill_ids.is_empty() {
            filtered.retain(|e| {
                e.skill_id
                    .as_ref()
                    .map_or(false, |id| query.skill_ids.iter().any(|q| q == id))
            });
        }

        if let Some(from) = query.date_from {
            filtered.retain(|e| entry_timestamp(e).map_or(false, |ts| ts >= from));
        }

        filtered.sort_by(|a, b| sort_key(b).cmp(sort_key(a)));
        let total = filtered.len();
        let results = filtered
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|entry| SearchResult {
                entry: entry.clone(),
                matches: Vec::new(),
                score: 0,
            })
            .collect();
        SearchResponse { results, total }
    }

    fn run_fts(
        &self,
        history: &SqliteHistoryService,
        query: &SearchQuery,
        tokens: &[String],
    ) -> Option<SearchResponse> {
        let fts_query = build_fts_query(tokens)?;
        let raw_hits = history
            .search_fts(
                &fts_query,
                query.type_filter,
                query.status_filter,
                &query.skill_ids,
                FTS_CANDIDATE_LIMIT,
            )
            .map_err(|e| {
                log::warn!(target: "app_lib::history_search", "fts search failed: {}", e);
                e
            })
            .ok()?;

        let now = Utc::now().timestamp();
        let mut scored: Vec<(f32, Vec<FieldMatch>, HistoryEntry)> = Vec::new();

        for (entry, raw_score) in raw_hits {
            if let Some(from) = query.date_from {
                if entry_timestamp(&entry).map_or(true, |ts| ts < from) {
                    continue;
                }
            }

            let positive_score = (-raw_score) as f32;
            let recency = recency_multiplier(&entry, now);
            let mut entry_matches: Vec<FieldMatch> = Vec::new();
            let mut max_proximity: f32 = 1.0;

            for (field, _weight) in FIELD_WEIGHTS {
                let Some(text) = field_value(&entry, *field) else {
                    continue;
                };
                let mut indices: Vec<u32> = Vec::new();
                for token in tokens {
                    if token.chars().count() == 0 {
                        continue;
                    }
                    indices.extend(collect_token_indices(text, token));
                }
                if indices.is_empty() {
                    continue;
                }
                indices.sort_unstable();
                indices.dedup();
                let prox = proximity_bonus(&indices);
                if prox > max_proximity {
                    max_proximity = prox;
                }
                entry_matches.push(FieldMatch {
                    field: *field,
                    indices,
                });
            }

            let final_score = positive_score * recency * max_proximity;
            scored.push((final_score, entry_matches, entry));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let total = scored.len();
        let results = scored
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|(score, matches, entry)| SearchResult {
                entry,
                matches,
                score: (score * 100.0) as i32,
            })
            .collect();

        Some(SearchResponse { results, total })
    }

    fn run_with_query_nucleo(
        &mut self,
        history: &SqliteHistoryService,
        query: &SearchQuery,
        query_str: &str,
        tokens: &[String],
    ) -> SearchResponse {
        let entries = history.get_history();
        let mut filtered: Vec<&HistoryEntry> = entries
            .iter()
            .filter(|e| matches_type_filter(e, query.type_filter))
            .filter(|e| matches_status_filter(e, query.status_filter))
            .collect();

        if !query.skill_ids.is_empty() {
            filtered.retain(|e| {
                e.skill_id
                    .as_ref()
                    .map_or(false, |id| query.skill_ids.iter().any(|q| q == id))
            });
        }

        if let Some(from) = query.date_from {
            filtered.retain(|e| entry_timestamp(e).map_or(false, |ts| ts >= from));
        }

        let avg_lens = compute_avg_lens(&filtered);
        let now = Utc::now().timestamp();
        let token_patterns: Vec<Pattern> = if tokens.len() > 1 {
            tokens
                .iter()
                .map(|t| Pattern::parse(t, CaseMatching::Smart, Normalization::Smart))
                .collect()
        } else {
            vec![Pattern::parse(
                query_str,
                CaseMatching::Smart,
                Normalization::Smart,
            )]
        };

        let mut scored: Vec<(f32, Vec<FieldMatch>, &HistoryEntry)> = Vec::new();
        let mut buf: Vec<char> = Vec::new();

        'outer: for entry in filtered {
            let mut entry_score: f32 = 0.0;
            let mut entry_matches: Vec<FieldMatch> = Vec::new();
            let mut token_matched = vec![false; token_patterns.len()];
            let mut title_strong_hit = false;

            for (field, weight) in FIELD_WEIGHTS {
                if title_strong_hit && *field == SearchField::OutputContent {
                    continue;
                }
                let Some(text) = field_value(entry, *field) else {
                    continue;
                };
                let avg = avg_lens.get(field).copied().unwrap_or(100.0);
                let len_norm = length_norm(text.chars().count(), avg);

                let haystack = Utf32Str::new(text, &mut buf);
                let mut field_indices: Vec<u32> = Vec::new();
                let mut field_score: f32 = 0.0;

                for (idx, pattern) in token_patterns.iter().enumerate() {
                    let mut local_idx: Vec<u32> = Vec::new();
                    let Some(score) = pattern.indices(haystack, &mut self.matcher, &mut local_idx)
                    else {
                        continue;
                    };
                    token_matched[idx] = true;
                    let weighted = score as f32 * weight * len_norm;
                    field_score += weighted;
                    field_indices.extend(local_idx);
                }

                if field_indices.is_empty() {
                    continue;
                }

                field_indices.sort_unstable();
                field_indices.dedup();
                let prox = proximity_bonus(&field_indices);
                let field_total = field_score * prox;
                if field_total > entry_score {
                    entry_score = field_total;
                }

                if *field == SearchField::Title {
                    let raw_title = field_score / weight.max(0.001) / len_norm.max(0.001);
                    if raw_title as i32 >= TITLE_EARLY_EXIT_SCORE {
                        title_strong_hit = true;
                    }
                }

                entry_matches.push(FieldMatch {
                    field: *field,
                    indices: field_indices,
                });
            }

            if entry_matches.is_empty() {
                continue;
            }

            if token_patterns.len() > 1 && !token_matched.iter().all(|m| *m) {
                continue 'outer;
            }

            let recency = recency_multiplier(entry, now);
            let final_score = entry_score * recency;
            scored.push((final_score, entry_matches, entry));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let total = scored.len();

        let results = scored
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|(score, matches, entry)| SearchResult {
                entry: entry.clone(),
                matches,
                score: score as i32,
            })
            .collect();

        SearchResponse { results, total }
    }
}

impl Default for HistorySearch {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_avg_lens(
    entries: &[&HistoryEntry],
) -> std::collections::HashMap<SearchField, f32> {
    use std::collections::HashMap;
    let mut sums: HashMap<SearchField, (usize, usize)> = HashMap::new();
    for entry in entries {
        for (field, _w) in FIELD_WEIGHTS {
            if let Some(text) = field_value(entry, *field) {
                let len = text.chars().count();
                let s = sums.entry(*field).or_insert((0, 0));
                s.0 += len;
                s.1 += 1;
            }
        }
    }
    let mut out: HashMap<SearchField, f32> = HashMap::new();
    for (field, fallback) in AVG_LEN_FALLBACK {
        let avg = sums
            .get(field)
            .map(|(sum, count)| if *count == 0 { *fallback } else { *sum as f32 / *count as f32 })
            .unwrap_or(*fallback);
        out.insert(*field, avg.max(1.0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::{HistoryEntry, HistoryEntryType};
    use crate::services::database::Database;

    fn make_entry(id: &str, input: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            timestamp: "2026-01-01 00:00:00".into(),
            input_content: input.into(),
            entry_type: HistoryEntryType::Text,
            output_content: None,
            skill_id: None,
            success: true,
            error: None,
            is_multi_turn: false,
            skill_name: None,
            conversation_data: None,
            created_at: Some("2026-01-01 00:00:00".into()),
            updated_at: None,
            quick_action: false,
            title: None,
            input_content_rendered: None,
            output_content_rendered: None,
        }
    }

    fn empty_query() -> SearchQuery {
        SearchQuery {
            query: String::new(),
            type_filter: HistoryTypeFilter::All,
            status_filter: HistoryStatusFilter::All,
            skill_ids: Vec::new(),
            date_from: None,
            limit: 50,
            offset: 0,
        }
    }

    fn local_format(ts: chrono::DateTime<Local>) -> String {
        ts.format(TIMESTAMP_FORMAT).to_string()
    }

    fn make_history(entries: Vec<HistoryEntry>) -> SqliteHistoryService {
        let db = Database::open_in_memory().unwrap();
        let svc = SqliteHistoryService::new(db, 10_000);
        for e in entries {
            insert_raw(&svc, e);
        }
        svc
    }

    fn insert_raw(svc: &SqliteHistoryService, e: HistoryEntry) {
        let entry_type_str = match e.entry_type {
            HistoryEntryType::Text => "text",
            HistoryEntryType::Speech => "speech",
        };
        svc.conn().execute(
            "INSERT INTO conversations (id, title, skill_id, skill_name, entry_type, input_content, output_content,
                success, error, is_multi_turn, quick_action, created_at, updated_at,
                input_content_rendered, output_content_rendered)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                e.id, e.title, e.skill_id, e.skill_name, entry_type_str,
                e.input_content, e.output_content, e.success, e.error,
                e.is_multi_turn, e.quick_action, e.created_at, e.updated_at,
                e.input_content_rendered, e.output_content_rendered,
            ],
        ).unwrap();
    }

    #[test]
    fn empty_query_returns_filtered_chat_entries_sorted_by_date() {
        let mut chat = make_entry("chat-1", "hello chat");
        chat.created_at = Some("2026-01-02 00:00:00".into());
        chat.quick_action = false;

        let mut quick = make_entry("quick-1", "hello quick");
        quick.created_at = Some("2026-01-03 00:00:00".into());
        quick.quick_action = true;
        quick.skill_name = Some("translate".into());

        let mut chat_old = make_entry("chat-2", "older chat");
        chat_old.created_at = Some("2026-01-01 00:00:00".into());
        chat_old.quick_action = false;

        let history = make_history(vec![chat, quick, chat_old]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.type_filter = HistoryTypeFilter::Chat;

        let response = search.run(&history, &q);
        assert_eq!(response.total, 2);
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].entry.id, "chat-1");
        assert_eq!(response.results[1].entry.id, "chat-2");
        assert!(response.results[0].matches.is_empty());
        assert_eq!(response.results[0].score, 0);
    }

    #[test]
    fn fts_query_matches_title_and_returns_indices() {
        let mut a = make_entry("a", "irrelevant body text here");
        a.title = Some("react hooks tutorial".into());

        let mut b = make_entry("b", "completely unrelated content");
        b.title = Some("svelte stores".into());

        let history = make_history(vec![a, b]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "react".into();

        let response = search.run(&history, &q);
        assert!(response.total >= 1);
        assert_eq!(response.results[0].entry.id, "a");
        let title_match = response.results[0]
            .matches
            .iter()
            .find(|m| m.field == SearchField::Title)
            .expect("title match present");
        assert_eq!(title_match.indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn fts_multi_token_requires_all_tokens() {
        let mut a = make_entry("a", "react hooks usage example".into());
        a.title = Some("react hooks tutorial".into());

        let mut b = make_entry("b", "react state management only");
        b.title = Some("redux".into());

        let history = make_history(vec![a, b]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "react hooks".into();

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "a");
    }

    #[test]
    fn short_query_falls_back_to_nucleo() {
        let mut a = make_entry("a", "ax");
        a.title = Some("ax".into());

        let history = make_history(vec![a]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "ax".into();

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "a");
    }

    #[test]
    fn fts_skill_filter_keeps_only_matching() {
        let mut a = make_entry("a", "translate body needs hello");
        a.skill_id = Some("translate".into());
        a.skill_name = Some("Translate".into());

        let mut b = make_entry("b", "summarize body needs hello");
        b.skill_id = Some("summarize".into());
        b.skill_name = Some("Summarize".into());

        let history = make_history(vec![a, b]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "hello".into();
        q.skill_ids = vec!["translate".into()];

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "a");
    }

    #[test]
    fn recency_boost_prefers_newer_entries_with_equal_match() {
        let now = Local::now();
        let recent_ts = local_format(now - chrono::Duration::hours(2));
        let old_ts = local_format(now - chrono::Duration::days(120));

        let mut recent = make_entry("recent", "react today".into());
        recent.title = Some("react today".into());
        recent.created_at = Some(recent_ts.clone());
        recent.timestamp = recent_ts;

        let mut old = make_entry("old", "react yesterday year".into());
        old.title = Some("react long ago".into());
        old.created_at = Some(old_ts.clone());
        old.timestamp = old_ts;

        let history = make_history(vec![old, recent]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "react".into();

        let response = search.run(&history, &q);
        assert!(response.total >= 2);
        assert_eq!(response.results[0].entry.id, "recent");
    }

    #[test]
    fn status_filter_error_excludes_successes() {
        let mut ok = make_entry("ok", "body has hello");
        ok.success = true;
        let mut bad = make_entry("bad", "body also has hello");
        bad.success = false;

        let history = make_history(vec![ok, bad]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "hello".into();
        q.status_filter = HistoryStatusFilter::Error;

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "bad");
    }

    #[test]
    fn pagination_limit_and_offset() {
        let entries: Vec<HistoryEntry> = (0..5)
            .map(|i| {
                let mut e = make_entry(&format!("e-{i}"), "body");
                e.created_at = Some(format!("2026-01-0{} 00:00:00", i + 1));
                e
            })
            .collect();

        let history = make_history(entries);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.limit = 2;
        q.offset = 1;

        let response = search.run(&history, &q);
        assert_eq!(response.total, 5);
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].entry.id, "e-3");
        assert_eq!(response.results[1].entry.id, "e-2");
    }

    #[test]
    fn fallback_to_raw_input_content_when_rendered_is_none() {
        let mut entry = make_entry("e", "raw input contains banana");
        entry.input_content_rendered = None;

        let history = make_history(vec![entry]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "banana".into();

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        let m = response.results[0]
            .matches
            .iter()
            .find(|m| m.field == SearchField::InputContent)
            .expect("input content match present");
        assert!(!m.indices.is_empty());
    }

    #[test]
    fn empty_entries_returns_empty_response() {
        let history = make_history(vec![]);
        let mut search = HistorySearch::new();
        let q = empty_query();
        let response = search.run(&history, &q);
        assert_eq!(response.total, 0);
        assert!(response.results.is_empty());
    }

    #[test]
    fn skill_filter_single_id_keeps_only_matching_entries() {
        let mut a = make_entry("a", "translate body");
        a.skill_id = Some("translate".into());
        a.skill_name = Some("Translate".into());

        let mut b = make_entry("b", "summarize body");
        b.skill_id = Some("summarize".into());
        b.skill_name = Some("Summarize".into());

        let c = make_entry("c", "no skill");

        let history = make_history(vec![a, b, c]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.skill_ids = vec!["translate".into()];

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "a");
    }

    #[test]
    fn skill_filter_empty_is_noop() {
        let mut a = make_entry("a", "translate body");
        a.skill_id = Some("translate".into());

        let b = make_entry("b", "no skill");

        let history = make_history(vec![a, b]);
        let mut search = HistorySearch::new();

        let q = empty_query();

        let response = search.run(&history, &q);
        assert_eq!(response.total, 2);
    }

    #[test]
    fn type_filter_speech_returns_only_transcriptions() {
        let mut transcription = make_entry("trans", "transcription text");
        transcription.entry_type = HistoryEntryType::Speech;
        transcription.quick_action = true;
        transcription.skill_name = None;

        let mut speech_with_skill = make_entry("speech-skill", "speech with skill");
        speech_with_skill.entry_type = HistoryEntryType::Speech;
        speech_with_skill.quick_action = true;
        speech_with_skill.skill_name = Some("summarize".into());

        let chat = make_entry("chat", "chat body");

        let history = make_history(vec![transcription, speech_with_skill, chat]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.type_filter = HistoryTypeFilter::Speech;

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "trans");
    }

    #[test]
    fn date_from_filter_keeps_only_recent_entries() {
        let now = Local::now();
        let recent_ts = local_format(now - chrono::Duration::hours(1));
        let old_ts = local_format(now - chrono::Duration::days(30));

        let mut recent = make_entry("recent", "recent body");
        recent.created_at = Some(recent_ts.clone());
        recent.timestamp = recent_ts;

        let mut old = make_entry("old", "old body");
        old.created_at = Some(old_ts.clone());
        old.timestamp = old_ts;

        let history = make_history(vec![recent, old]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.date_from = Some((now - chrono::Duration::hours(24)).timestamp());

        let response = search.run(&history, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "recent");
    }

    #[test]
    fn date_from_none_returns_all_entries() {
        let mut a = make_entry("a", "a body");
        a.created_at = Some("2020-01-01 00:00:00".into());
        let mut b = make_entry("b", "b body");
        b.created_at = Some("2026-01-01 00:00:00".into());

        let history = make_history(vec![a, b]);
        let mut search = HistorySearch::new();

        let q = empty_query();

        let response = search.run(&history, &q);
        assert_eq!(response.total, 2);
    }

    #[test]
    fn fts_query_with_special_chars_does_not_panic() {
        let mut a = make_entry("a", "use react and *redux* together");
        a.title = Some("react".into());

        let history = make_history(vec![a]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = r#"react "needs escaping" *"#.into();

        let _response = search.run(&history, &q);
    }

    #[test]
    fn proximity_bonus_clusters_beat_scattered() {
        let mut clustered = make_entry(
            "cluster",
            "react hooks together in tight match phrase here",
        );
        clustered.title = None;

        let mut scattered = make_entry(
            "scatter",
            "react some long unrelated content that fills space and then much later hooks appear",
        );
        scattered.title = None;

        let history = make_history(vec![clustered, scattered]);
        let mut search = HistorySearch::new();

        let mut q = empty_query();
        q.query = "react hooks".into();

        let response = search.run(&history, &q);
        assert!(response.total >= 1);
        assert_eq!(response.results[0].entry.id, "cluster");
    }
}
