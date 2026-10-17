use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::services::settings_sync::{run_sync, SettingsSync, SyncResult, SyncStatus, Trigger};
use crate::Error;

#[tauri::command]
pub async fn get_settings_sync_status(
    engine: State<'_, Arc<SettingsSync>>,
) -> crate::Result<SyncStatus> {
    Ok(engine.status())
}

#[tauri::command]
pub async fn sync_settings_now(
    app: AppHandle,
    engine: State<'_, Arc<SettingsSync>>,
) -> crate::Result<SyncStatus> {
    match run_sync(&app, Trigger::Manual).await {
        SyncResult::AlreadySyncing => Err(Error::Other("already syncing".to_string())),
        SyncResult::Paused | SyncResult::Completed(_) => Ok(engine.status()),
    }
}
