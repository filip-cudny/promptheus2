use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::models::ai::{StreamEvent, ToolCall};
use crate::services::config::ConfigService;

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Execution already in progress")]
    AlreadyExecuting,

    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

#[derive(Clone, Serialize)]
pub struct ExecutionSnapshot {
    pub execution_id: String,
    pub user_message: String,
    pub accumulated_text: String,
    pub accumulated_thinking: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub is_thinking: bool,
    pub finished: bool,
    pub error: Option<String>,
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
}

pub struct LiveExecution {
    pub snapshot: ExecutionSnapshot,
    pub channel: Option<Channel<StreamEvent>>,
}

pub struct PromptExecutionService {
    is_executing: bool,
    current_execution_id: Option<String>,
    pub live: Option<Arc<TokioMutex<LiveExecution>>>,
}

impl PromptExecutionService {
    pub fn new() -> Self {
        Self {
            is_executing: false,
            current_execution_id: None,
            live: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_executing
    }

    pub fn current_execution_id(&self) -> Option<&str> {
        self.current_execution_id.as_deref()
    }

    pub fn start_execution(&mut self) -> Result<String, ExecutionError> {
        if self.is_executing {
            return Err(ExecutionError::AlreadyExecuting);
        }
        let execution_id = Uuid::new_v4().to_string();
        self.is_executing = true;
        self.current_execution_id = Some(execution_id.clone());
        Ok(execution_id)
    }

    pub fn start_live(
        &mut self,
        execution_id: &str,
        user_message: String,
        channel: Channel<StreamEvent>,
    ) -> Arc<TokioMutex<LiveExecution>> {
        let live = Arc::new(TokioMutex::new(LiveExecution {
            snapshot: ExecutionSnapshot {
                execution_id: execution_id.to_string(),
                user_message,
                accumulated_text: String::new(),
                accumulated_thinking: None,
                tool_calls: Vec::new(),
                is_thinking: false,
                finished: false,
                error: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
            channel: Some(channel),
        }));
        self.live = Some(Arc::clone(&live));
        live
    }

    pub fn finish_execution(&mut self) {
        self.is_executing = false;
        self.current_execution_id = None;
        self.live = None;
    }

    pub fn resolve_model(
        config: &ConfigService,
        model_id: Option<&str>,
    ) -> Result<String, ExecutionError> {
        match model_id {
            Some(id) => {
                let exists = config.settings().models.iter().any(|m| m.id == id);
                if exists {
                    Ok(id.to_string())
                } else {
                    Err(ExecutionError::ModelNotFound(id.to_string()))
                }
            }
            None => config
                .settings()
                .default_model
                .clone()
                .ok_or_else(|| ExecutionError::ModelNotFound("no default model configured".to_string())),
        }
    }

    pub fn resolve_quick_action_model(
        config: &ConfigService,
        model_id: Option<&str>,
    ) -> Result<String, ExecutionError> {
        match model_id {
            Some(id) => {
                let exists = config.settings().models.iter().any(|m| m.id == id);
                if exists {
                    Ok(id.to_string())
                } else {
                    Err(ExecutionError::ModelNotFound(id.to_string()))
                }
            }
            None => config
                .settings()
                .quick_action_default_model
                .clone()
                .or_else(|| config.settings().default_model.clone())
                .ok_or_else(|| ExecutionError::ModelNotFound("no default model configured".to_string())),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_config() -> (TempDir, ConfigService) {
        let dir = TempDir::new().unwrap();
        let settings = crate::models::settings::Settings {
            default_model: Some("model-1".to_string()),
            models: vec![crate::models::settings::ModelConfig {
                id: "model-1".to_string(),
                model: "gpt-4".to_string(),
                display_name: "GPT-4".to_string(),
                api_key_source: crate::models::settings::ApiKeySource::Direct,
                provider: Default::default(),
                api_key_env: None,
                api_key: Some("test-key".to_string()),
                base_url: None,
                parameters: None,
                context_window_size: None,
                api_mode: None,
                store: true,
                enabled_tools: vec![],
            }],
            ..Default::default()
        };
        let settings_path = dir.path().join("settings.json");
        std::fs::write(&settings_path, serde_json::to_string(&settings).unwrap()).unwrap();
        let config = ConfigService::load(dir.path(), None).unwrap();
        (dir, config)
    }

    #[test]
    fn start_execution_succeeds_when_idle() {
        let mut svc = PromptExecutionService::new();
        assert!(!svc.is_busy());
        let id = svc.start_execution().unwrap();
        assert!(!id.is_empty());
        assert!(svc.is_busy());
        assert_eq!(svc.current_execution_id(), Some(id.as_str()));
    }

    #[test]
    fn start_execution_fails_when_busy() {
        let mut svc = PromptExecutionService::new();
        svc.start_execution().unwrap();
        let result = svc.start_execution();
        assert!(matches!(result, Err(ExecutionError::AlreadyExecuting)));
    }

    #[test]
    fn finish_execution_resets_state() {
        let mut svc = PromptExecutionService::new();
        svc.start_execution().unwrap();
        assert!(svc.is_busy());
        svc.finish_execution();
        assert!(!svc.is_busy());
        assert!(svc.current_execution_id().is_none());
    }

    #[test]
    fn resolve_model_falls_back_to_default() {
        let (_dir, config) = setup_config();
        let result = PromptExecutionService::resolve_model(&config, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "model-1");
    }

    #[test]
    fn resolve_model_validates_explicit_id() {
        let (_dir, config) = setup_config();
        let result = PromptExecutionService::resolve_model(&config, Some("model-1"));
        assert!(result.is_ok());

        let result = PromptExecutionService::resolve_model(&config, Some("nonexistent"));
        assert!(matches!(result, Err(ExecutionError::ModelNotFound(_))));
    }

    #[test]
    fn finish_allows_restart() {
        let mut svc = PromptExecutionService::new();
        let id1 = svc.start_execution().unwrap();
        svc.finish_execution();
        let id2 = svc.start_execution().unwrap();
        assert_ne!(id1, id2);
    }
}
