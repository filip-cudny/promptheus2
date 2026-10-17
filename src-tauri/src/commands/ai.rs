use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::models::capabilities::ModelCapabilities;
use crate::services::config::ConfigService;
use crate::services::ai::capabilities::resolve;

#[tauri::command]
pub async fn get_model_capabilities(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    model_id: String,
) -> crate::Result<ModelCapabilities> {
    let config = config.lock().await;
    let model = config
        .settings()
        .models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| crate::services::ai::AiError::ModelNotFound(model_id.clone()))?;
    Ok(resolve(model))
}
