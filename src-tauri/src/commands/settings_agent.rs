use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::commands::change_log::apply_plan;
use crate::models::ai::StreamEvent;
use crate::models::settings::Settings;
use crate::services::ai::AiService;
use crate::services::change_log::TestResult;
use crate::services::config::ConfigService;
use crate::services::execution::PromptExecutionService;
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::recent_apps::RecentAppsState;
use crate::services::settings_agent::dispatch::{Proposal, SecretAnswer};
use crate::services::settings_agent::engine::{
    self, describe, lock, proposal_payload, EngineContext, Emit, SharedSession,
};
use crate::services::settings_agent::plan::{plan, stale_rows, Current};
use crate::services::settings_agent::testing::{run_row_test, RowTest, RowTestRequest};

struct TauriEmit(AppHandle);

impl Emit for TauriEmit {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Err(e) = self.0.emit(event, payload) {
            log::warn!("settings agent: cannot emit {event}: {e}");
        }
    }
}

async fn fresh_settings(app: &AppHandle) -> Settings {
    let config = app.state::<Arc<Mutex<ConfigService>>>();
    let mut config = config.lock().await;
    let _ = config.reload_if_changed();
    config.settings().clone()
}

async fn placeholders(app: &AppHandle) -> PlaceholderContext {
    let recent_apps = app.state::<Arc<RecentAppsState>>();
    PlaceholderContext::with_apps(recent_apps.active().await, recent_apps.display().await)
}

fn proposal_of(session: &SharedSession, proposal_id: &str) -> Result<Proposal, String> {
    lock(session)
        .proposal
        .clone()
        .filter(|p| p.id == proposal_id)
        .ok_or_else(|| "this proposal is no longer available".to_string())
}

fn drop_proposal(session: &SharedSession, proposal_id: &str) {
    let mut s = lock(session);
    if s.proposal.as_ref().is_some_and(|p| p.id == proposal_id) {
        s.proposal = None;
    }
}

#[tauri::command]
pub async fn settings_agent_send(
    app: AppHandle,
    session: State<'_, SharedSession>,
    message: String,
    on_event: Channel<StreamEvent>,
) -> crate::Result<()> {
    let settings = fresh_settings(&app).await;
    let config_dir = app.path().app_config_dir()?;
    let ai = Arc::clone(&*app.state::<Arc<Mutex<AiService>>>()).lock().await.clone();
    let ctx = EngineContext {
        skills_dir: config_dir.join("skills"),
        config_dir,
        settings: Arc::new(move || settings.clone()),
        ai,
        ai_factory: Arc::new(|settings| AiService::new(&settings.models)),
        placeholders: placeholders(&app).await,
        emit: Arc::new(TauriEmit(app.clone())),
        transcript_dir: app.path().app_data_dir()?.join("settings-agent-turns"),
    };
    let (live, _cancel) = PromptExecutionService::new().start_live(
        &uuid::Uuid::new_v4().to_string(),
        message.clone(),
        on_event,
    );
    engine::run_turn(&session, &message, ctx, live).await
}

#[tauri::command]
pub async fn settings_agent_cancel(session: State<'_, SharedSession>) -> crate::Result<()> {
    lock(&session).cancel();
    Ok(())
}

#[tauri::command]
pub async fn settings_agent_reset(session: State<'_, SharedSession>) -> crate::Result<()> {
    lock(&session).reset();
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretOutcome {
    Saved,
    Cancelled,
}

#[tauri::command]
pub async fn settings_agent_secret_result(
    session: State<'_, SharedSession>,
    request_id: String,
    outcome: SecretOutcome,
) -> crate::Result<()> {
    let answer = match outcome {
        SecretOutcome::Saved => SecretAnswer::Saved,
        SecretOutcome::Cancelled => SecretAnswer::Cancelled,
    };
    if !lock(&session).resolve_secret(&request_id, answer) {
        log::debug!("settings agent: no pending secret request {request_id}");
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ApplyResult {
    Applied { entry_id: String },
    Stale { stale_rows: Vec<usize> },
    Failed { error: String },
}

async fn apply(
    app: &AppHandle,
    session: &SharedSession,
    proposal_id: &str,
    selected: Vec<bool>,
    values: BTreeMap<String, String>,
) -> Result<ApplyResult, String> {
    let proposal = proposal_of(session, proposal_id)?;
    if !selected.iter().any(|s| *s) {
        return Err("no rows selected".to_string());
    }
    let settings = fresh_settings(app).await;
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let current = Current { settings: &settings, config_dir: &config_dir };
    let planned = plan(&proposal.operations, &selected, &current).map_err(|e| describe(&e))?;
    let selected_rows: Vec<usize> = planned.rows.iter().map(|r| r.row).collect();
    let stale: Vec<usize> = stale_rows(&proposal.plan, &current)
        .into_iter()
        .filter(|row| selected_rows.contains(row))
        .collect();
    if !stale.is_empty() {
        return Ok(ApplyResult::Stale { stale_rows: stale });
    }
    let (request, reply) = {
        let s = lock(session);
        (s.last_request.clone(), s.last_reply.clone())
    };
    let entry_id = apply_plan(app, planned, request, reply, proposal.tests, values)
        .await
        .map_err(|e| e.to_string())?;
    drop_proposal(session, proposal_id);
    Ok(ApplyResult::Applied { entry_id })
}

#[tauri::command]
pub async fn settings_agent_apply(
    app: AppHandle,
    session: State<'_, SharedSession>,
    proposal_id: String,
    selected: Vec<bool>,
    values: BTreeMap<String, String>,
) -> crate::Result<ApplyResult> {
    log::info!("settings agent apply of proposal {proposal_id}");
    Ok(apply(&app, &session, &proposal_id, selected, values)
        .await
        .unwrap_or_else(|error| ApplyResult::Failed { error }))
}

#[tauri::command]
pub async fn settings_agent_reject(
    session: State<'_, SharedSession>,
    proposal_id: String,
) -> crate::Result<()> {
    log::info!("settings agent rejected proposal {proposal_id}");
    drop_proposal(&session, &proposal_id);
    Ok(())
}

#[tauri::command]
pub async fn settings_agent_refresh(
    app: AppHandle,
    session: State<'_, SharedSession>,
    proposal_id: String,
) -> crate::Result<()> {
    let proposal = proposal_of(&session, &proposal_id).map_err(crate::Error::Other)?;
    let settings = fresh_settings(&app).await;
    let config_dir = app.path().app_config_dir()?;
    let current = Current { settings: &settings, config_dir: &config_dir };
    let refreshed =
        engine::refresh(&proposal, &current).map_err(|e| crate::Error::Other(describe(&e)))?;
    let payload = proposal_payload(&refreshed, &current);
    lock(&session).proposal = Some(refreshed);
    TauriEmit(app).emit(engine::PROPOSAL_EVENT, payload);
    Ok(())
}

#[tauri::command]
pub async fn settings_agent_test_row(
    app: AppHandle,
    session: State<'_, SharedSession>,
    proposal_id: String,
    row: usize,
    values: BTreeMap<String, String>,
) -> crate::Result<TestResult> {
    let proposal = proposal_of(&session, &proposal_id).map_err(crate::Error::Other)?;
    if row >= proposal.operations.len() {
        return Err(crate::Error::Other(format!("row {row} does not exist")));
    }
    let before = fresh_settings(&app).await;
    let config_dir = app.path().app_config_dir()?;
    let request = RowTestRequest {
        row,
        operations: &proposal.operations,
        plan: &proposal.plan,
        before: &before,
        values: &values,
        ai_factory: &|settings: &Settings| AiService::new(&settings.models),
        placeholders: &placeholders(&app).await,
        config_dir: &config_dir,
        allow_new_destinations: true,
    };
    let result = match run_row_test(&request).await {
        RowTest::Tested(result) => result,
        RowTest::NotTestable => return Err(crate::Error::Other(format!("row {row} has no test"))),
        RowTest::Waits(names) => {
            return Err(crate::Error::Other(format!(
                "row {row} waits for a value of {}",
                names.join(", ")
            )))
        }
        RowTest::NewDestination(text) => return Err(crate::Error::Other(text)),
    };
    let mut s = lock(&session);
    if let Some(held) = s.proposal.as_mut().filter(|p| p.id == proposal_id) {
        held.tests.insert(row, result.clone());
        held.waits.remove(&row);
        held.new_destinations.remove(&row);
    }
    Ok(result)
}

#[tauri::command]
pub async fn settings_agent_panel(app: AppHandle, open: bool) -> crate::Result<&'static str> {
    crate::services::settings_agent::panel::set_open(&app, open).await
}
