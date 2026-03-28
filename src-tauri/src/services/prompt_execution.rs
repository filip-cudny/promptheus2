use uuid::Uuid;

use crate::models::message::ProcessedMessage;
use crate::models::settings::PromptData;
use crate::services::clipboard::ClipboardService;
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::placeholder::{PlaceholderError, PlaceholderService};

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Execution already in progress")]
    AlreadyExecuting,

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Clipboard unavailable: {0}")]
    ClipboardError(String),
}

pub struct PromptExecutionService {
    is_executing: bool,
    current_execution_id: Option<String>,
}

impl PromptExecutionService {
    pub fn new() -> Self {
        Self {
            is_executing: false,
            current_execution_id: None,
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

    pub fn finish_execution(&mut self) {
        self.is_executing = false;
        self.current_execution_id = None;
    }

    pub fn resolve_prompt(
        config: &ConfigService,
        prompt_id: &str,
    ) -> Result<PromptData, ExecutionError> {
        config
            .settings()
            .prompts
            .iter()
            .find(|p| p.id == prompt_id)
            .cloned()
            .ok_or_else(|| ExecutionError::PromptNotFound(prompt_id.to_string()))
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

    pub fn prepare_messages(
        prompt: &PromptData,
        placeholder: &PlaceholderService,
        clipboard: &ClipboardService,
        context: &ContextManagerService,
        input_override: Option<&str>,
    ) -> Result<Vec<ProcessedMessage>, ExecutionError> {
        let messages: Vec<(&str, &str)> = prompt
            .messages
            .iter()
            .map(|m| (m.role.as_str(), m.content.as_str()))
            .collect();

        placeholder
            .process_messages(&messages, input_override, clipboard, context)
            .map_err(|e| match e {
                PlaceholderError::ClipboardUnavailable(msg) => ExecutionError::ClipboardError(msg),
                other => ExecutionError::ClipboardError(other.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{PromptData, PromptMessage};
    use std::path::Path;
    use tempfile::TempDir;

    fn setup_config_with_prompts(prompts: Vec<PromptData>) -> (TempDir, ConfigService) {
        let dir = TempDir::new().unwrap();
        let settings = crate::models::settings::Settings {
            prompts,
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
    fn resolve_prompt_finds_existing() {
        let prompts = vec![PromptData {
            id: "p1".to_string(),
            name: "Test Prompt".to_string(),
            description: None,
            messages: vec![],
        }];
        let (_dir, config) = setup_config_with_prompts(prompts);
        let result = PromptExecutionService::resolve_prompt(&config, "p1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Prompt");
    }

    #[test]
    fn resolve_prompt_error_for_missing() {
        let (_dir, config) = setup_config_with_prompts(vec![]);
        let result = PromptExecutionService::resolve_prompt(&config, "nonexistent");
        assert!(matches!(result, Err(ExecutionError::PromptNotFound(_))));
    }

    #[test]
    fn resolve_model_falls_back_to_default() {
        let (_dir, config) = setup_config_with_prompts(vec![]);
        let result = PromptExecutionService::resolve_model(&config, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "model-1");
    }

    #[test]
    fn resolve_model_validates_explicit_id() {
        let (_dir, config) = setup_config_with_prompts(vec![]);
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
