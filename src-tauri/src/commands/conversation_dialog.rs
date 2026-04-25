use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::services::ai_providers;
use crate::services::ai_webview;
use crate::services::dialog::{self, focus_host_window, is_shell_toolbar_label, DialogConfig};

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
        label: label.into(),
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat".into(),
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: "conversation-dialog".into(),
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
            new_chat,
        )?;
        return Ok(());
    }

    let (_, _created) = dialog::open_or_focus(&app, &config).await?;
    Ok(())
}

fn surface_conversation_dialog(app: &tauri::AppHandle, label: &str) {
    use crate::services::ai_webview;

    if let Some(host) = app.get_window(label) {
        for webview in host.webviews() {
            let wv_label = webview.label();
            if wv_label == label {
                let _ = webview.show();
                let _ = webview.set_focus();
            } else if is_shell_toolbar_label(wv_label) {
                let _ = webview.show();
            } else {
                let _ = webview.hide();
            }
        }
        if let Err(e) = host.set_title("Promptheus — chat") {
            log::warn!("set_title failed: {e}");
        }
    }
    ai_webview::mark_active_webview(app, label, label);
    ai_webview::emit_active_changed_for(app, label, None);
    let _ = focus_host_window(app, label);
}

#[tauri::command]
pub async fn open_conversation_dialog_new_window(
    app: tauri::AppHandle,
    source_label: Option<String>,
    provider_id: Option<String>,
) -> Result<(), String> {
    let label = next_conversation_dialog_label(&app);
    log::info!(
        target: "app_lib::commands::conversation_dialog",
        "open_conversation_dialog_new_window label={label} source={source_label:?} provider={provider_id:?}",
    );

    if let Some(src) = source_label.as_deref() {
        dialog::seed_geometry_from(&app, src, &label).await;
    }

    let config = DialogConfig {
        label: label.clone(),
        url: "conversation-dialog.html".into(),
        title: "Promptheus — chat".into(),
        default_width: 700.0,
        default_height: 600.0,
        geometry_key: label.clone(),
    };

    let (_, _) = dialog::open_or_focus(&app, &config).await?;

    if let Some(pid) = provider_id.as_deref() {
        if !pid.is_empty() && pid != "promptheus" {
            let provider = ai_providers::find(pid)
                .ok_or_else(|| format!("unknown provider: {pid}"))?;
            ai_webview::swap_to_provider(&app, provider, &label).await?;
        }
    }

    Ok(())
}

fn next_conversation_dialog_label(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    let base = "conversation-dialog";
    if app.get_window(base).is_none() {
        return base.to_string();
    }
    let mut i = 2u32;
    loop {
        let candidate = format!("{base}-{i}");
        if app.get_window(&candidate).is_none() {
            return candidate;
        }
        i += 1;
    }
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
