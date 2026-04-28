use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::{Deserialize, Serialize};

use crate::models::history::{HistoryEntry, HistoryEntryType};

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

pub struct HistorySearch {
    matcher: Matcher,
}

impl HistorySearch {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    pub fn run(&mut self, entries: &[HistoryEntry], query: &SearchQuery) -> SearchResponse {
        let filtered: Vec<&HistoryEntry> = entries
            .iter()
            .filter(|e| matches_type_filter(e, query.type_filter))
            .filter(|e| matches_status_filter(e, query.status_filter))
            .collect();

        let trimmed_query = query.query.trim();

        if trimmed_query.is_empty() {
            return self.run_empty_query(filtered, query.offset, query.limit);
        }

        self.run_with_query(filtered, trimmed_query, query.offset, query.limit)
    }

    fn run_empty_query(
        &self,
        mut filtered: Vec<&HistoryEntry>,
        offset: usize,
        limit: usize,
    ) -> SearchResponse {
        filtered.sort_by(|a, b| sort_key(b).cmp(sort_key(a)));
        let total = filtered.len();
        let results = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|entry| SearchResult {
                entry: entry.clone(),
                matches: Vec::new(),
                score: 0,
            })
            .collect();
        SearchResponse { results, total }
    }

    fn run_with_query(
        &mut self,
        filtered: Vec<&HistoryEntry>,
        query_str: &str,
        offset: usize,
        limit: usize,
    ) -> SearchResponse {
        let pattern = Pattern::parse(query_str, CaseMatching::Smart, Normalization::Smart);

        let mut scored: Vec<(i32, Vec<FieldMatch>, &HistoryEntry)> = Vec::new();
        let mut buf: Vec<char> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for entry in filtered {
            let mut entry_score: f32 = 0.0;
            let mut entry_matches: Vec<FieldMatch> = Vec::new();

            for (field, weight) in FIELD_WEIGHTS {
                let Some(text) = field_value(entry, *field) else {
                    continue;
                };
                indices.clear();
                let haystack = Utf32Str::new(text, &mut buf);
                let Some(score) = pattern.indices(haystack, &mut self.matcher, &mut indices) else {
                    continue;
                };
                let weighted = score as f32 * weight;
                if weighted > entry_score {
                    entry_score = weighted;
                }
                indices.sort_unstable();
                indices.dedup();
                entry_matches.push(FieldMatch {
                    field: *field,
                    indices: indices.clone(),
                });
            }

            if entry_matches.is_empty() {
                continue;
            }

            scored.push((entry_score as i32, entry_matches, entry));
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let total = scored.len();

        let results = scored
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(score, matches, entry)| SearchResult {
                entry: entry.clone(),
                matches,
                score,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::{HistoryEntry, HistoryEntryType};

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
            limit: 50,
            offset: 0,
        }
    }

    #[test]
    fn empty_query_returns_filtered_chat_entries_sorted_by_date() {
        let mut search = HistorySearch::new();

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

        let entries = vec![chat.clone(), quick, chat_old.clone()];

        let mut q = empty_query();
        q.type_filter = HistoryTypeFilter::Chat;

        let response = search.run(&entries, &q);
        assert_eq!(response.total, 2);
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].entry.id, "chat-1");
        assert_eq!(response.results[1].entry.id, "chat-2");
        assert!(response.results[0].matches.is_empty());
        assert_eq!(response.results[0].score, 0);
    }

    #[test]
    fn query_matches_title_with_highest_score() {
        let mut search = HistorySearch::new();

        let mut a = make_entry("a", "hello world");
        a.title = Some("hello world".into());

        let mut b = make_entry("b", "unrelated body");
        b.title = Some("nothing here".into());

        let mut c = make_entry("c", "hello somewhere in body");
        c.title = Some("title".into());

        let entries = vec![a, b, c];

        let mut q = empty_query();
        q.query = "hello".into();

        let response = search.run(&entries, &q);
        assert!(response.total >= 2);
        assert_eq!(response.results[0].entry.id, "a");
        let title_match = response.results[0]
            .matches
            .iter()
            .find(|m| m.field == SearchField::Title)
            .expect("title match present");
        assert!(!title_match.indices.is_empty());
    }

    #[test]
    fn query_matches_skill_name_with_indices() {
        let mut search = HistorySearch::new();

        let mut entry = make_entry("e", "irrelevant body");
        entry.skill_name = Some("translate".into());

        let mut q = empty_query();
        q.query = "tran".into();

        let response = search.run(&[entry], &q);
        assert_eq!(response.total, 1);
        let m = response.results[0]
            .matches
            .iter()
            .find(|m| m.field == SearchField::SkillName)
            .expect("skill match present");
        assert_eq!(m.indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn status_filter_error_excludes_successes() {
        let mut search = HistorySearch::new();

        let mut ok = make_entry("ok", "ok body");
        ok.success = true;
        let mut bad = make_entry("bad", "bad body");
        bad.success = false;

        let mut q = empty_query();
        q.status_filter = HistoryStatusFilter::Error;

        let response = search.run(&[ok, bad], &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "bad");
    }

    #[test]
    fn pagination_limit_and_offset() {
        let mut search = HistorySearch::new();

        let entries: Vec<HistoryEntry> = (0..5)
            .map(|i| {
                let mut e = make_entry(&format!("e-{i}"), "body");
                e.created_at = Some(format!("2026-01-0{} 00:00:00", i + 1));
                e
            })
            .collect();

        let mut q = empty_query();
        q.limit = 2;
        q.offset = 1;

        let response = search.run(&entries, &q);
        assert_eq!(response.total, 5);
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].entry.id, "e-3");
        assert_eq!(response.results[1].entry.id, "e-2");
    }

    #[test]
    fn fallback_to_raw_input_content_when_rendered_is_none() {
        let mut search = HistorySearch::new();

        let mut entry = make_entry("e", "raw input contains banana");
        entry.input_content_rendered = None;

        let mut q = empty_query();
        q.query = "banana".into();

        let response = search.run(&[entry], &q);
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
        let mut search = HistorySearch::new();
        let q = empty_query();
        let response = search.run(&[], &q);
        assert_eq!(response.total, 0);
        assert!(response.results.is_empty());
    }

    #[test]
    fn type_filter_speech_returns_only_transcriptions() {
        let mut search = HistorySearch::new();

        let mut transcription = make_entry("trans", "transcription text");
        transcription.entry_type = HistoryEntryType::Speech;
        transcription.quick_action = true;
        transcription.skill_name = None;

        let mut speech_with_skill = make_entry("speech-skill", "speech with skill");
        speech_with_skill.entry_type = HistoryEntryType::Speech;
        speech_with_skill.quick_action = true;
        speech_with_skill.skill_name = Some("summarize".into());

        let chat = make_entry("chat", "chat body");

        let entries = vec![transcription, speech_with_skill, chat];

        let mut q = empty_query();
        q.type_filter = HistoryTypeFilter::Speech;

        let response = search.run(&entries, &q);
        assert_eq!(response.total, 1);
        assert_eq!(response.results[0].entry.id, "trans");
    }
}
