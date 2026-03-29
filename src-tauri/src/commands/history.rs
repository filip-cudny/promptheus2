use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::history::{
    HistoryEntry, HistoryEntryType, SerializedConversationNode, SerializedConversationTurn,
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
    prompt_id: Option<String>,
    success: bool,
    error: Option<String>,
    is_conversation: bool,
    prompt_name: Option<String>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.history.add_entry(
        input_content,
        entry_type,
        output_content,
        prompt_id,
        success,
        error,
        is_conversation,
        prompt_name,
    );
    emit_history_changed(&app)
}

#[tauri::command]
pub async fn add_conversation_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    turns: Vec<SerializedConversationTurn>,
    context_text: String,
    context_image_paths: Vec<String>,
    prompt_id: Option<String>,
    prompt_name: Option<String>,
    success: bool,
    error: Option<String>,
    nodes: Vec<SerializedConversationNode>,
    root_node_id: Option<String>,
    current_path: Vec<String>,
) -> Result<String, String> {
    let mut state = state.lock().await;
    let id = state.history.add_conversation_entry(
        &turns,
        context_text,
        context_image_paths,
        prompt_id,
        prompt_name,
        success,
        error,
        nodes,
        root_node_id,
        current_path,
    );
    emit_history_changed(&app)?;
    Ok(id)
}

#[tauri::command]
pub async fn update_conversation_entry(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    entry_id: String,
    turns: Vec<SerializedConversationTurn>,
    context_text: String,
    context_image_paths: Vec<String>,
    nodes: Vec<SerializedConversationNode>,
    root_node_id: Option<String>,
    current_path: Vec<String>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state
        .history
        .update_conversation_entry(
            &entry_id,
            &turns,
            context_text,
            context_image_paths,
            nodes,
            root_node_id,
            current_path,
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
        last_text: state.history.get_last_item_by_type(HistoryEntryType::Text),
        last_speech: state
            .history
            .get_last_item_by_type(HistoryEntryType::Speech),
    })
}

#[tauri::command]
pub async fn clear_history(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
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
