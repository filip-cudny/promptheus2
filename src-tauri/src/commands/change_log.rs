use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::commands::settings::apply_settings_reload;
use crate::services::change_log::{
    change_log_dir, entry_state, load_all, ChangeLogEntry, EntryState, TestResult,
};
use crate::services::config::ConfigService;
use crate::services::settings_agent::apply::{self, Context, UndoOutcome};
use crate::services::settings_agent::plan::{Current, Plan};
use crate::services::settings_sync::SettingsSync;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggedState {
    Applied,
    Undone,
}

#[derive(Serialize)]
pub struct ChangeLogItem {
    entry: ChangeLogEntry,
    state: LoggedState,
}

#[tauri::command]
pub async fn list_change_log(app: AppHandle) -> crate::Result<Vec<ChangeLogItem>> {
    let config_dir = app.path().app_config_dir()?;
    let items = tokio::task::spawn_blocking(move || {
        let entries = load_all(&change_log_dir(&config_dir));
        let mut items: Vec<ChangeLogItem> = entries
            .iter()
            .map(|entry| ChangeLogItem {
                state: match entry_state(&entries, &entry.id) {
                    EntryState::Applied => LoggedState::Applied,
                    EntryState::Undone => LoggedState::Undone,
                },
                entry: entry.clone(),
            })
            .collect();
        items.reverse();
        items
    })
    .await
    .map_err(|e| crate::Error::Other(e.to_string()))?;
    Ok(items)
}

pub(crate) async fn apply_plan(
    app: &AppHandle,
    plan: Plan,
    request: Option<String>,
    reply: Option<String>,
    tests: BTreeMap<usize, TestResult>,
) -> crate::Result<String> {
    let config_dir = app.path().app_config_dir()?;
    let host = app.state::<Arc<SettingsSync>>().host().await;
    let mut config = Arc::clone(&*app.state::<Arc<Mutex<ConfigService>>>()).lock_owned().await;
    let id = tokio::task::spawn_blocking(move || {
        let log_dir = change_log_dir(&config_dir);
        let settings = config.settings().clone();
        let mut ctx = Context {
            config: &mut config,
            config_dir: &config_dir,
            log_dir: &log_dir,
            host: &host,
            request,
            reply,
            tests,
        };
        apply::apply(&plan, &Current { settings: &settings, config_dir: &config_dir }, &mut ctx)
    })
    .await
    .map_err(|e| crate::Error::Other(e.to_string()))?
    .map_err(|e| crate::Error::Other(e.to_string()))?;
    finish_write(app).await?;
    Ok(id)
}

pub(crate) async fn undo_entry(app: &AppHandle, id: String, force: bool) -> crate::Result<UndoOutcome> {
    let config_dir = app.path().app_config_dir()?;
    let host = app.state::<Arc<SettingsSync>>().host().await;
    let mut config = Arc::clone(&*app.state::<Arc<Mutex<ConfigService>>>()).lock_owned().await;
    let outcome = tokio::task::spawn_blocking(move || {
        let log_dir = change_log_dir(&config_dir);
        let mut ctx = Context {
            config: &mut config,
            config_dir: &config_dir,
            log_dir: &log_dir,
            host: &host,
            request: None,
            reply: None,
            tests: BTreeMap::new(),
        };
        apply::undo(&id, force, &mut ctx)
    })
    .await
    .map_err(|e| crate::Error::Other(e.to_string()))?
    .map_err(|e| crate::Error::Other(e.to_string()))?;
    if matches!(outcome, UndoOutcome::Undone { .. }) {
        finish_write(app).await?;
    }
    Ok(outcome)
}

async fn finish_write(app: &AppHandle) -> crate::Result<()> {
    apply_settings_reload(app, "agent").await?;
    app.emit("change-log-changed", ())?;
    Ok(())
}

#[tauri::command]
pub async fn undo_change(app: AppHandle, id: String, force: bool) -> crate::Result<UndoOutcome> {
    log::info!("undo_change {id} force={force}");
    undo_entry(&app, id, force).await
}
