use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::history::{
    HistoryEntry, HistoryEntryType, ImagePayload, SerializedConversationNode,
};

use super::settings::AppState;

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
pub async fn clear_history(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let state = state.lock().await;
    state.history.clear();
    emit_history_changed(&app)
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
