use std::collections::HashMap;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::history::{
    HistoryEntry, HistoryEntryType, ImagePayload, SerializedConversationNode,
};
use crate::services::history_search::{SearchQuery, SearchResponse};

use super::settings::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct SkillCount {
    pub skill_id: String,
    pub skill_name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastInteractionData {
    pub last_text: Option<HistoryEntry>,
    pub last_speech: Option<HistoryEntry>,
}

fn emit_history_changed(app: &AppHandle) -> Result<(), String> {
    app.emit("history-changed", ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_history(state: State<'_, Mutex<AppState>>) -> Result<Vec<HistoryEntry>, String> {
    let state = state.lock().await;
    Ok(state.history.get_history())
}

#[tauri::command]
pub async fn get_conversations(
    state: State<'_, Mutex<AppState>>,
    offset: u32,
    limit: u32,
) -> Result<Vec<HistoryEntry>, String> {
    let state = state.lock().await;
    Ok(state.history.get_conversations(offset, limit))
}

#[tauri::command]
pub async fn get_history_entry(
    state: State<'_, Mutex<AppState>>,
    entry_id: String,
) -> Result<Option<HistoryEntry>, String> {
    let state = state.lock().await;
    Ok(state.history.get_entry_by_id(&entry_id))
}

#[tauri::command]
pub async fn add_history_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    input_content: String,
    entry_type: HistoryEntryType,
    output_content: Option<String>,
    skill_id: Option<String>,
    success: bool,
    error: Option<String>,
    is_multi_turn: bool,
    skill_name: Option<String>,
) -> Result<(), String> {
    let state = state.lock().await;
    state.history.add_entry(
        input_content,
        entry_type,
        output_content,
        skill_id,
        success,
        error,
        is_multi_turn,
        skill_name,
        false,
    );
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn add_conversation_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    context_text: String,
    skill_id: Option<String>,
    skill_name: Option<String>,
    success: bool,
    error: Option<String>,
    nodes: Vec<SerializedConversationNode>,
    root_node_id: Option<String>,
    current_path: Vec<String>,
    tab_id: Option<String>,
    #[allow(unused_variables)] images: Vec<ImagePayload>,
    model_id: Option<String>,
    reasoning_effort: Option<String>,
) -> Result<String, String> {
    let state = state.lock().await;
    let resolved_environment_section = tab_id
        .as_deref()
        .and_then(|id| state.conversation_context.get(id))
        .map(|s| s.to_string());
    let id = state.history.add_conversation_entry(
        context_text,
        skill_id,
        skill_name,
        success,
        error,
        nodes,
        root_node_id,
        current_path,
        false,
        resolved_environment_section,
        images,
        model_id,
        reasoning_effort,
    );
    emit_history_changed(&app)?;
    Ok(id)
}

#[tauri::command]
pub async fn update_conversation_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    entry_id: String,
    context_text: String,
    nodes: Vec<SerializedConversationNode>,
    root_node_id: Option<String>,
    current_path: Vec<String>,
    #[allow(unused_variables)] images: Vec<ImagePayload>,
    model_id: Option<String>,
    reasoning_effort: Option<String>,
) -> Result<(), String> {
    let state = state.lock().await;
    state
        .history
        .update_conversation_entry(
            &entry_id,
            context_text,
            nodes,
            root_node_id,
            current_path,
            images,
            model_id,
            reasoning_effort,
        )
        .map_err(|e| e.to_string())?;
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn get_last_interaction(
    state: State<'_, Mutex<AppState>>,
) -> Result<LastInteractionData, String> {
    let state = state.lock().await;
    Ok(LastInteractionData {
        last_text: state.history.get_last_quick_action(HistoryEntryType::Text),
        last_speech: state
            .history
            .get_last_quick_action(HistoryEntryType::Speech),
    })
}

#[tauri::command]
pub async fn update_history_entry_title(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    entry_id: String,
    title: String,
) -> Result<(), String> {
    let state = state.lock().await;
    state
        .history
        .update_entry_title(&entry_id, title)
        .map_err(|e| e.to_string())?;
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn delete_history_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    entry_id: String,
) -> Result<(), String> {
    let state = state.lock().await;
    state
        .history
        .delete_entry(&entry_id)
        .map_err(|e| e.to_string())?;
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn clear_history(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let state = state.lock().await;
    state.history.clear();
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn search_history(
    state: State<'_, Mutex<AppState>>,
    query: SearchQuery,
) -> Result<SearchResponse, String> {
    let mut state = state.lock().await;
    let state = &mut *state;
    let response = state.history_search.run(&state.history, &query);
    log::debug!(
        target: "app_lib::history_search",
        "search_history: query='{}' type={:?} status={:?} skills={} date_from={:?} total={} returned={}",
        query.query,
        query.type_filter,
        query.status_filter,
        query.skill_ids.len(),
        query.date_from,
        response.total,
        response.results.len(),
    );
    Ok(response)
}

fn collect_skill_counts(entries: &[HistoryEntry]) -> Vec<SkillCount> {
    let mut map: HashMap<String, SkillCount> = HashMap::new();
    for e in entries.iter() {
        let id = e.skill_id.as_deref().filter(|s| !s.is_empty());
        let name = e.skill_name.as_deref().filter(|s| !s.is_empty());
        if let (Some(id), Some(name)) = (id, name) {
            map.entry(id.to_string())
                .and_modify(|c| c.count += 1)
                .or_insert(SkillCount {
                    skill_id: id.to_string(),
                    skill_name: name.to_string(),
                    count: 1,
                });
        }
    }
    let mut list: Vec<SkillCount> = map.into_values().collect();
    list.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.skill_name.cmp(&b.skill_name)));
    list
}

#[tauri::command]
pub async fn list_history_skills(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<SkillCount>, String> {
    let state = state.lock().await;
    let entries = state.history.get_history();
    let list = collect_skill_counts(&entries);
    log::debug!(
        target: "app_lib::history_search",
        "list_history_skills: returned {} unique skills",
        list.len(),
    );
    Ok(list)
}

#[tauri::command]
pub async fn copy_history_content(
    state: State<'_, Mutex<AppState>>,
    content: String,
) -> Result<(), String> {
    let state = state.lock().await;
    state
        .clipboard
        .set_text(&content)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::HistoryEntryType;

    fn make_entry(id: &str, skill_id: Option<&str>, skill_name: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            timestamp: "2026-01-01 00:00:00".into(),
            input_content: "input".into(),
            entry_type: HistoryEntryType::Text,
            output_content: None,
            skill_id: skill_id.map(|s| s.to_string()),
            success: true,
            error: None,
            is_multi_turn: false,
            skill_name: skill_name.map(|s| s.to_string()),
            conversation_data: None,
            created_at: Some("2026-01-01 00:00:00".into()),
            updated_at: None,
            quick_action: false,
            title: None,
        }
    }

    #[test]
    fn collect_skill_counts_aggregates_and_sorts_by_count_desc_then_name_asc() {
        let entries = vec![
            make_entry("a", Some("translate"), Some("Translate")),
            make_entry("b", Some("translate"), Some("Translate")),
            make_entry("c", Some("translate"), Some("Translate")),
            make_entry("d", Some("rewrite"), Some("Rewrite")),
            make_entry("e", Some("rewrite"), Some("Rewrite")),
            make_entry("f", Some("summarize"), Some("Summarize")),
            make_entry("g", Some("expand"), Some("Expand")),
            make_entry("h", None, None),
            make_entry("i", Some("partial"), None),
            make_entry("j", None, Some("orphan")),
        ];

        let counts = collect_skill_counts(&entries);
        assert_eq!(counts.len(), 4);
        assert_eq!(counts[0].skill_id, "translate");
        assert_eq!(counts[0].count, 3);
        assert_eq!(counts[1].skill_id, "rewrite");
        assert_eq!(counts[1].count, 2);
        assert_eq!(counts[2].skill_name, "Expand");
        assert_eq!(counts[2].count, 1);
        assert_eq!(counts[3].skill_name, "Summarize");
        assert_eq!(counts[3].count, 1);
    }

    #[test]
    fn collect_skill_counts_empty_input_returns_empty_list() {
        let counts = collect_skill_counts(&[]);
        assert!(counts.is_empty());
    }

    #[test]
    fn collect_skill_counts_skips_entries_with_empty_skill_id_or_name() {
        let entries = vec![
            make_entry("legacy-chat", Some(""), Some("Chat")),
            make_entry("legacy-empty-name", Some("translate"), Some("")),
            make_entry("legacy-both-empty", Some(""), Some("")),
            make_entry("real", Some("translate"), Some("Translate")),
        ];

        let counts = collect_skill_counts(&entries);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].skill_id, "translate");
        assert_eq!(counts[0].skill_name, "Translate");
        assert_eq!(counts[0].count, 1);
    }
}
