use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::skill::{Skill, SkillSummary};
use crate::services::config::ConfigService;
use crate::services::skill::SkillService;
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

#[tauri::command]
pub async fn list_skills(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
) -> crate::Result<Vec<SkillSummary>> {
    Ok(skill_service
        .lock()
        .await
        .list_skills()
        .iter()
        .map(SkillSummary::from)
        .collect())
}

#[tauri::command]
pub async fn get_skill(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    name: String,
) -> crate::Result<Skill> {
    skill_service
        .lock()
        .await
        .get_skill(&name)
        .cloned()
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn get_skill_body(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    name: String,
) -> crate::Result<String> {
    skill_service
        .lock()
        .await
        .get_skill(&name)
        .map(|s| s.body.clone())
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn reload_skills(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
) -> crate::Result<()> {
    let order = config.lock().await.settings().skills_order.clone();
    let mut skill_service = skill_service.lock().await;
    skill_service.reload(&order)?;
    let history = history.lock().await;
    skill_service.sync_versions(history.conn())?;
    Ok(())
}
