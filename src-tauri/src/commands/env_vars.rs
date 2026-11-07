use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::commands::change_log::apply_plan;
use crate::models::settings::Settings;
use crate::services::config::ConfigService;
use crate::services::missing_env::{env_is_set, env_var_overview, EnvVarStatus};
use crate::services::secrets;
use crate::services::settings_agent::operations::Operation;
use crate::services::settings_agent::plan::{plan_single, Current};

async fn fresh_settings(app: &AppHandle) -> Settings {
    let config = app.state::<Arc<Mutex<ConfigService>>>();
    let mut config = config.lock().await;
    let _ = config.reload_if_changed();
    config.settings().clone()
}

#[tauri::command]
pub async fn get_env_vars(app: AppHandle) -> crate::Result<Vec<EnvVarStatus>> {
    let settings = fresh_settings(&app).await;
    let config_dir = app.path().app_config_dir()?;
    Ok(env_var_overview(&settings, env_is_set, |name| secrets::current_value(&config_dir, name)))
}

#[tauri::command]
pub async fn apply_env_var_operation(app: AppHandle, operation: Operation) -> crate::Result<String> {
    let variable = match &operation {
        Operation::AddEnvVar { name, .. }
        | Operation::UpdateEnvVar { name, .. }
        | Operation::RenameEnvVar { name, .. }
        | Operation::DeleteEnvVar { name } => name.clone(),
        other => {
            return Err(crate::Error::Other(format!(
                "{} is not an environment variable operation",
                other.name()
            )))
        }
    };
    log::info!("env var operation {} on {variable}", operation.name());
    let settings = fresh_settings(&app).await;
    let config_dir = app.path().app_config_dir()?;
    let current = Current { settings: &settings, config_dir: &config_dir };
    let planned = plan_single(operation, &current).map_err(crate::Error::Other)?;
    apply_plan(&app, planned, None, None, BTreeMap::new(), BTreeMap::new()).await
}
