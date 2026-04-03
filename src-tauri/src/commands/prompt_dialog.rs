use tauri::Emitter;

use crate::services::dialog::{self, DialogConfig};

#[tauri::command]
pub async fn open_prompt_dialog(
    app: tauri::AppHandle,
    prompt_id: String,
    prompt_name: String,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
) -> Result<(), String> {
    let label = "prompt-dialog";

    let config = DialogConfig {
        label,
        url: build_url(&prompt_id, &prompt_name, &history_entry_id, last_interaction_only, &initial_input, auto_send_input),
        title: "Promptheus — chat",
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "prompt-dialog",
    };

    let (_, created) = dialog::open_or_focus(&app, &config).await?;

    if !created {
        emit_reuse_event(&app, label, history_entry_id, last_interaction_only, initial_input, auto_send_input, &prompt_id, &prompt_name)?;
    }

    Ok(())
}

fn build_url(
    prompt_id: &str,
    prompt_name: &str,
    history_entry_id: &Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: &Option<String>,
    auto_send_input: Option<bool>,
) -> String {
    let mut url = format!(
        "prompt-dialog.html?promptId={}&promptName={}",
        prompt_id,
        urlencoding::encode(prompt_name),
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
    prompt_id: &str,
    prompt_name: &str,
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
                "prompt_id": prompt_id,
                "prompt_name": prompt_name,
            }),
        )
        .map_err(|e| e.to_string())
    }
}
