use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, OwnedMutexGuard};

use crate::commands::settings::{apply_reloaded_settings, emit_changed as emit_settings_changed, rebuild_ai};
use crate::commands::skills::emit_changed as emit_skills_changed;
use crate::services::ai::AiService;
use crate::services::config::ConfigService;
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::settings_sync::{
    reload_changed, CycleReport, Reloaded, SettingsSync, SyncHooks, SyncResult, SyncState,
    SyncStatus, Trigger,
};
use crate::services::skill::SkillService;
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

const NOTIFICATION_EVENT: &str = "settings_sync";

pub struct LocalGuard {
    config: OwnedMutexGuard<ConfigService>,
    skills: OwnedMutexGuard<SkillService>,
}

struct AppHooks {
    app: AppHandle,
    reloaded: StdMutex<Reloaded>,
}

#[async_trait]
impl SyncHooks for AppHooks {
    type Guard = LocalGuard;

    async fn lock_local(&self) -> LocalGuard {
        let config = self.app.state::<Arc<Mutex<ConfigService>>>().inner().clone();
        let skills = self.app.state::<Arc<Mutex<SkillService>>>().inner().clone();
        let config = config.lock_owned().await;
        let skills = skills.lock_owned().await;
        LocalGuard { config, skills }
    }

    async fn reload(&self, guard: &mut LocalGuard, changed: &[PathBuf]) {
        let config_dir = match self.app.path().app_config_dir() {
            Ok(dir) => dir,
            Err(e) => {
                log::warn!("settings sync: config dir unavailable, skipping reload: {e}");
                return;
            }
        };
        let history = self.app.state::<Arc<Mutex<SqliteHistoryService>>>().inner().clone();
        let history = history.lock().await;
        let result = reload_changed(
            changed,
            &config_dir.join("settings.json"),
            &config_dir.join("skills"),
            &mut guard.config,
            &mut guard.skills,
            history.conn(),
        );
        match result {
            Ok(reloaded) => {
                if reloaded.settings {
                    let ai = self.app.state::<Arc<Mutex<AiService>>>().inner().clone();
                    rebuild_ai(&guard.config, &mut *ai.lock().await);
                }
                *self.reloaded.lock().unwrap() = reloaded;
            }
            Err(e) => log::warn!("settings sync: reload after pull failed: {e}"),
        }
    }

    fn status_changed(&self, status: &SyncStatus) {
        if let Err(e) = self.app.emit("settings-sync-status-changed", status) {
            log::warn!("settings sync: failed to emit status: {e}");
        }
    }
}

pub async fn run_sync(app: &AppHandle, trigger: Trigger) -> SyncResult {
    let engine = app.state::<Arc<SettingsSync>>().inner().clone();
    let hooks = AppHooks {
        app: app.clone(),
        reloaded: StdMutex::new(Reloaded::default()),
    };
    let result = engine.sync(trigger, &hooks).await;
    let reloaded = *hooks.reloaded.lock().unwrap();
    apply_reloaded(app, reloaded).await;
    if let SyncResult::Completed(report) = &result {
        notify_state_entered(app, report).await;
    }
    result
}

async fn apply_reloaded(app: &AppHandle, reloaded: Reloaded) {
    if reloaded.settings {
        let config = app.state::<Arc<Mutex<ConfigService>>>().inner().clone();
        let settings = config.lock().await.settings().clone();
        apply_reloaded_settings(app, &settings);
        if let Err(e) = emit_settings_changed(app) {
            log::warn!("settings sync: failed to emit settings-changed: {e}");
        }
    }
    if reloaded.skills {
        if let Err(e) = emit_skills_changed(app, "sync") {
            log::warn!("settings sync: failed to emit skills-changed: {e}");
        }
    }
}

async fn notify_state_entered(app: &AppHandle, report: &CycleReport) {
    if !report.entered_new_state {
        return;
    }
    let status = &report.status;
    let (title, message) = match status.state {
        SyncState::Conflict => (
            "Settings sync paused",
            format!(
                "Rebase conflict in {}. Resolve it manually, then press \"Sync now\" in Settings to retry.",
                status.repo_path.as_deref().unwrap_or("the settings repo")
            ),
        ),
        SyncState::Error => (
            "Settings sync failed",
            status.message.clone().unwrap_or_default(),
        ),
        _ => return,
    };
    let config = app.state::<Arc<Mutex<ConfigService>>>().inner().clone();
    let notifications = config.lock().await.settings().notifications.clone();
    log::info!("settings sync: showing '{title}' toast");
    let notifier = app.state::<NotificationService>();
    if let Err(e) = notifier.notify(
        NOTIFICATION_EVENT,
        NotificationLevel::Error,
        title,
        Some(message),
        &notifications,
    ) {
        log::warn!("settings sync: toast failed: {e}");
    }
}

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
