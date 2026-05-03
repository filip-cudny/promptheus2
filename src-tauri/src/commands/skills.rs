use tauri::State;
use tokio::sync::Mutex;

use crate::commands::settings::AppState;
use crate::models::skill::{Skill, SkillSummary};
use crate::Error;

#[tauri::command]
pub async fn list_skills(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<Vec<SkillSummary>> {
    let state = state.lock().await;
    Ok(state
        .skill_service
        .list_skills()
        .iter()
        .map(SkillSummary::from)
        .collect())
}

#[tauri::command]
pub async fn get_skill(
    state: State<'_, Mutex<AppState>>,
    name: String,
) -> crate::Result<Skill> {
    let state = state.lock().await;
    state
        .skill_service
        .get_skill(&name)
        .cloned()
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn get_skill_body(
    state: State<'_, Mutex<AppState>>,
    name: String,
) -> crate::Result<String> {
    let state = state.lock().await;
    state
        .skill_service
        .get_skill(&name)
        .map(|s| s.body.clone())
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn reload_skills(
    state: State<'_, Mutex<AppState>>,
) -> crate::Result<()> {
    let mut guard = state.lock().await;
    let s = &mut *guard;
    let order = s.config.settings().skills_order.clone();
    s.skill_service.reload(&order)?;
    s.skill_service.sync_versions(s.history.conn())?;
    Ok(())
}
