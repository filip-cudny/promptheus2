use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::models::history::InvocationSource;
use crate::services::ai_webview;
use crate::services::dialog::{self, focus_host_window, DialogConfig};
use crate::Error;

#[derive(Serialize, Clone)]
pub struct NewConversationEvent {
    pub reason: String,
}

#[derive(Serialize, Clone)]
struct RestoreHistoryEvent {
    entry_id: String,
    last_interaction_only: bool,
}

#[derive(Serialize, Clone)]
struct VoiceInputEvent {
    text: String,
    skill_id: String,
    skill_model: Option<String>,
    auto_send: bool,
    invocation_source: Option<InvocationSource>,
    copy_token: Option<String>,
}

#[derive(Serialize, Clone)]
struct OpenForSkillEvent {
    skill_id: String,
    skill_name: String,
    skill_model: Option<String>,
}

struct PendingDialogParams {
    skill_id: String,
    skill_name: String,
    skill_model: Option<String>,
    history_entry_id: Option<String>,
    last_interaction_only: bool,
    initial_input: Option<String>,
    auto_send_input: bool,
    new_chat: bool,
    invocation_source: Option<InvocationSource>,
    copy_token: Option<String>,
}

static PENDING: Mutex<Option<PendingDialogParams>> = Mutex::new(None);

#[derive(Serialize)]
pub struct DialogInitParams {
    skill_id: String,
    skill_name: String,
    skill_model: Option<String>,
    history_entry_id: Option<String>,
    last_interaction_only: bool,
    initial_input: Option<String>,
    auto_send_input: bool,
    new_chat: bool,
    invocation_source: Option<InvocationSource>,
    copy_token: Option<String>,
}

pub fn conversation_dialog_config() -> DialogConfig {
    DialogConfig {
        label: "conversation-dialog".into(),
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat".into(),
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "conversation-dialog".into(),
    }
}

#[tauri::command]
pub async fn open_conversation_dialog(
    app: tauri::AppHandle,
    skill_id: String,
    skill_name: String,
    skill_model: Option<String>,
    history_entry_id: Option<String>,
    last_interaction_only: Option<bool>,
    initial_input: Option<String>,
    auto_send_input: Option<bool>,
    new_chat: Option<bool>,
    invocation_source: Option<InvocationSource>,
    copy_token: Option<String>,
) -> crate::Result<()> {
    let label = "conversation-dialog";
    let last_interaction_only = last_interaction_only.unwrap_or(false);
    let auto_send_input = auto_send_input.unwrap_or(false);
    let new_chat = new_chat.unwrap_or(false);

    let config = conversation_dialog_config();

    if app.get_window(label).is_some() {
        surface_conversation_dialog(&app, label);
        emit_reuse_event(
            &app,
            label,
            history_entry_id,
            last_interaction_only,
            initial_input,
            auto_send_input,
            &skill_id,
            &skill_name,
            skill_model.as_deref(),
            new_chat,
            invocation_source,
            copy_token,
        )?;
        return Ok(());
    }

    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingDialogParams {
        skill_id: skill_id.clone(),
        skill_name: skill_name.clone(),
        skill_model: skill_model.clone(),
        history_entry_id: history_entry_id.clone(),
        last_interaction_only,
        initial_input: initial_input.clone(),
        auto_send_input,
        new_chat,
        invocation_source,
        copy_token,
    });

    let (_, _created) = dialog::open_or_focus(&app, &config)
        .await
        .map_err(Error::Other)?;
    Ok(())
}

fn surface_conversation_dialog(app: &tauri::AppHandle, label: &str) {
    if let Err(e) = ai_webview::show_native_view(app) {
        log::warn!("show_native_view failed: {e}");
    }
    let _ = focus_host_window(app, label);
}

#[tauri::command]
pub async fn focus_or_open_chat(app: tauri::AppHandle) -> crate::Result<()> {
    let label = "conversation-dialog";

    if app.get_window(label).is_some() {
        log::debug!(
            target: "app_lib::commands::conversation_dialog",
            "focus_or_open_chat: focusing existing host {label}",
        );
        return focus_host_window(&app, label).map_err(Error::Other);
    }

    let config = DialogConfig {
        label: label.into(),
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat".into(),
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "conversation-dialog".into(),
    };

    let (_, _) = dialog::open_or_focus(&app, &config)
        .await
        .map_err(Error::Other)?;
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
            skill_model: p.skill_model,
            history_entry_id: p.history_entry_id,
            last_interaction_only: p.last_interaction_only,
            initial_input: p.initial_input,
            auto_send_input: p.auto_send_input,
            new_chat: p.new_chat,
            invocation_source: p.invocation_source,
            copy_token: p.copy_token,
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
    skill_model: Option<&str>,
    new_chat: bool,
    invocation_source: Option<InvocationSource>,
    copy_token: Option<String>,
) -> crate::Result<()> {
    if app.get_webview_window(label).is_none() {
        return Err(Error::Other(format!(
            "conversation dialog window '{label}' missing"
        )));
    }

    if new_chat {
        app.emit_to(
            label,
            "new-conversation",
            NewConversationEvent {
                reason: "reopen".into(),
            },
        )?;
    } else if let Some(entry_id) = history_entry_id {
        app.emit_to(
            label,
            "restore-history",
            RestoreHistoryEvent {
                entry_id,
                last_interaction_only,
            },
        )?;
    } else if let Some(text) = initial_input {
        app.emit_to(
            label,
            "voice-input",
            VoiceInputEvent {
                text,
                skill_id: skill_id.to_string(),
                skill_model: skill_model.map(|s| s.to_string()),
                auto_send: auto_send_input,
                invocation_source,
                copy_token,
            },
        )?;
    } else if !skill_id.is_empty() {
        app.emit_to(
            label,
            "open-for-skill",
            OpenForSkillEvent {
                skill_id: skill_id.to_string(),
                skill_name: skill_name.to_string(),
                skill_model: skill_model.map(|s| s.to_string()),
            },
        )?;
    }
    Ok(())
}
