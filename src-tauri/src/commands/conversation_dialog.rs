use std::sync::Mutex;

use serde::Serialize;
use tauri::Emitter;

use crate::services::dialog::{self, DialogConfig};

struct PendingDialogParams {
    skill_id: String,
    skill_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: bool,
    initial_input: Option<String>,
    auto_send_input: bool,
    new_chat: bool,
}

static PENDING: Mutex<Option<PendingDialogParams>> = Mutex::new(None);

#[derive(Serialize)]
pub struct DialogInitParams {
    skill_id: String,
    skill_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: bool,
    initial_input: Option<String>,
    auto_send_input: bool,
    new_chat: bool,
}

#[tauri::command]
pub async fn open_conversation_dialog(
    app: tauri::AppHandle,
    skill_id: String,
    skill_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
    new_chat: Option<bool>,
) -> Result<(), String> {
    let label = "conversation-dialog";
    let last_interaction_only = last_interaction_only.unwrap_or(false);
    let auto_send_input = auto_send_input.unwrap_or(false);
    let new_chat = new_chat.unwrap_or(false);

    let config = DialogConfig {
        label,
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat",
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "conversation-dialog",
    };

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingDialogParams {
        skill_id: skill_id.clone(),
        skill_name: skill_name.clone(),
        history_entry_id: history_entry_id.clone(),
        last_interaction_only,
        initial_input: initial_input.clone(),
        auto_send_input,
        new_chat,
    });

    let (_, created) = dialog::open_or_focus(&app, &config).await?;

    if !created {
        emit_reuse_event(
            &app,
            label,
            history_entry_id,
            last_interaction_only,
            initial_input,
            auto_send_input,
            &skill_id,
            &skill_name,
            new_chat,
        )?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_dialog_init_params() -> Option<DialogInitParams> {
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .map(|p| DialogInitParams {
            skill_id: p.skill_id,
            skill_name: p.skill_name,
            history_entry_id: p.history_entry_id,
            last_interaction_only: p.last_interaction_only,
            initial_input: p.initial_input,
            auto_send_input: p.auto_send_input,
            new_chat: p.new_chat,
        })
}

fn emit_reuse_event(
    app: &tauri::AppHandle,
    label: &str,
    history_entry_id: Option<String>,
    last_interaction_only: bool,
    initial_input: Option<String>,
    auto_send_input: bool,
    skill_id: &str,
    skill_name: &str,
    new_chat: bool,
) -> Result<(), String> {
    if new_chat {
        app.emit_to(label, "new-conversation", serde_json::json!({}))
            .map_err(|e| e.to_string())
    } else if let Some(entry_id) = history_entry_id {
        app.emit_to(
            label,
            "restore-history",
            serde_json::json!({
                "entry_id": entry_id,
                "last_interaction_only": last_interaction_only,
            }),
        )
        .map_err(|e| e.to_string())
    } else if let Some(input) = &initial_input {
        app.emit_to(
            label,
            "voice-input",
            serde_json::json!({
                "text": input,
                "auto_send": auto_send_input,
            }),
        )
        .map_err(|e| e.to_string())
    } else if !skill_id.is_empty() {
        app.emit_to(
            label,
            "open-for-skill",
            serde_json::json!({
                "skill_id": skill_id,
                "skill_name": skill_name,
            }),
        )
        .map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}
