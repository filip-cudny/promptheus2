use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::Mutex;

use crate::models::ai::StreamEvent;
use crate::services::execution::lifecycle::{ExecutionSnapshot, PromptExecutionService};
use crate::services::tool_confirmation::ToolConfirmationService;
use crate::Error;

#[tauri::command]
pub async fn cancel_skill_execution(
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
) -> crate::Result<bool> {
    Ok(prompt_execution.lock().await.cancel_execution())
}

#[tauri::command]
pub async fn cancel_live_execution(
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
) -> crate::Result<bool> {
    Ok(prompt_execution.lock().await.cancel_live())
}

#[tauri::command]
pub async fn get_executing_skill_id(
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
) -> crate::Result<Option<String>> {
    Ok(prompt_execution
        .lock()
        .await
        .executing_skill_id()
        .map(|s| s.to_string()))
}

#[tauri::command]
pub async fn respond_to_tool_call(
    tool_confirmation: State<'_, Arc<Mutex<ToolConfirmationService>>>,
    tool_call_id: String,
    approved: bool,
) -> crate::Result<()> {
    tool_confirmation
        .lock()
        .await
        .respond(&tool_call_id, approved)
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn retry_tool_call(
    tool_confirmation: State<'_, Arc<Mutex<ToolConfirmationService>>>,
    tool_call_id: String,
) -> crate::Result<()> {
    tool_confirmation
        .lock()
        .await
        .respond(&tool_call_id, true)
        .map_err(Error::Other)
}

#[tauri::command]
pub async fn reconnect_to_execution(
    prompt_execution: State<'_, Arc<Mutex<PromptExecutionService>>>,
    on_event: Channel<StreamEvent>,
) -> crate::Result<Option<ExecutionSnapshot>> {
    let live_arc = prompt_execution.lock().await.live.clone();
    let Some(live_arc) = live_arc else {
        return Ok(None);
    };

    let mut live = live_arc.lock().await;
    let snapshot = live.snapshot.clone();

    if !snapshot.finished {
        live.channel = Some(on_event);
    }

    Ok(Some(snapshot))
}
