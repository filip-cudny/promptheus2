use tauri::Emitter;

use crate::services::dialog::{self, DialogConfig};

#[tauri::command]
pub async fn open_conversation_dialog(
    app: tauri::AppHandle,
    skill_id: String,
    skill_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
) -> Result<(), String> {
    let label = "conversation-dialog";

    let config = DialogConfig {
        label,
        url: build_url(&skill_id, &skill_name, &history_entry_id, last_interaction_only, &initial_input, auto_send_input),
        title: "Promptheus — chat",
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "conversation-dialog",
    };

    let (_, created) = dialog::open_or_focus(&app, &config).await?;

    if !created {
        emit_reuse_event(&app, label, history_entry_id, last_interaction_only, initial_input, auto_send_input, &skill_id, &skill_name)?;
    }

    Ok(())
}

fn build_url(
    skill_id: &str,
    skill_name: &str,
    history_entry_id: &Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: &Option<String>,
    auto_send_input: Option<bool>,
) -> String {
    let mut url = format!(
        "conversation-dialog.html?skillId={}&skillName={}",
        skill_id,
        urlencoding::encode(skill_name),
    );
    if let Some(entry_id) = history_entry_id {
        url.push_str(&format!("&historyEntryId={}", entry_id));
    }
    if last_interaction_only.unwrap_or(false) {
        url.push_str("&lastInteractionOnly=true");
    }
    if let Some(input) = initial_input {
        url.push_str(&format!("&initialInput={}", urlencoding::encode(input)));
        if auto_send_input.unwrap_or(false) {
            url.push_str("&autoSendInput=true");
        }
    }
    url
}

fn emit_reuse_event(
    app: &tauri::AppHandle,
    label: &str,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
    skill_id: &str,
    skill_name: &str,
) -> Result<(), String> {
    if let Some(entry_id) = history_entry_id {
        app.emit_to(
            label,
            "restore-history",
            serde_json::json!({
                "entry_id": entry_id,
                "last_interaction_only": last_interaction_only.unwrap_or(false),
            }),
        )
        .map_err(|e| e.to_string())
    } else if let Some(input) = &initial_input {
        app.emit_to(
            label,
            "voice-input",
            serde_json::json!({
                "text": input,
                "auto_send": auto_send_input.unwrap_or(false),
            }),
        )
        .map_err(|e| e.to_string())
    } else {
        app.emit_to(
            label,
            "open-for-skill",
            serde_json::json!({
                "skill_id": skill_id,
                "skill_name": skill_name,
            }),
        )
        .map_err(|e| e.to_string())
    }
}
