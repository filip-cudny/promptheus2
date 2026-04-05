pub mod openai;
pub mod openai_responses;
pub mod provider;
pub mod sse;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;

use crate::models::message::ProcessedMessage;
use crate::models::settings::{ApiMode, ModelConfig, ModelParameters, Provider};

use self::openai::OpenAiProvider;
use self::openai_responses::OpenAiResponsesProvider;
use self::provider::{AiProvider, CompletionRequest, StreamChunk};

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("model unavailable: {0}")]
    ModelUnavailable(String),

    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("rate limit exceeded")]
    RateLimit,

    #[error("API error (status {status}): {message}")]
    ApiStatus { status: u16, message: String },

    #[error("stream error: {0}")]
    Stream(String),

    #[error("request failed: {0}")]
    Request(String),
}

struct ProviderEntry {
    provider: Box<dyn AiProvider>,
    model_name: String,
    parameters: ModelParameters,
}

struct AiServiceInner {
    providers: HashMap<String, ProviderEntry>,
    unavailable_models: HashMap<String, String>,
}

#[derive(Clone)]
pub struct AiService {
    inner: Arc<AiServiceInner>,
}

impl AiService {
    pub fn new(models: &[ModelConfig]) -> Self {
        let mut providers = HashMap::new();
        let mut unavailable_models = HashMap::new();

        for model in models {
            match model.provider {
                Provider::Openai => {
                    let api_mode = model.api_mode.clone().unwrap_or_default();
                    let result: Result<Box<dyn AiProvider>, AiError> = match api_mode {
                        ApiMode::Responses => OpenAiResponsesProvider::new(model).map(|p| Box::new(p) as Box<dyn AiProvider>),
                        ApiMode::Completions => OpenAiProvider::new(model).map(|p| Box::new(p) as Box<dyn AiProvider>),
                    };
                    match result {
                        Ok(provider) => {
                            providers.insert(
                                model.id.clone(),
                                ProviderEntry {
                                    provider,
                                    model_name: model.model.clone(),
                                    parameters: model.parameters.clone().unwrap_or_default(),
                                },
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "model '{}' unavailable: {}",
                                model.display_name,
                                e
                            );
                            unavailable_models.insert(model.id.clone(), e.to_string());
                        }
                    }
                }
                Provider::Anthropic => {
                    unavailable_models
                        .insert(model.id.clone(), "Anthropic provider not yet supported".into());
                }
                Provider::Gemini => {
                    unavailable_models
                        .insert(model.id.clone(), "Gemini provider not yet supported".into());
                }
            }
        }

        Self {
            inner: Arc::new(AiServiceInner {
                providers,
                unavailable_models,
            }),
        }
    }

    pub async fn complete(
        &self,
        model_id: &str,
        messages: Vec<ProcessedMessage>,
    ) -> Result<String, AiError> {
        let entry = self.get_provider(model_id)?;
        let request = CompletionRequest {
            model: entry.model_name.clone(),
            messages,
            parameters: entry.parameters.clone(),
        };
        entry.provider.complete(request).await
    }

    pub async fn complete_stream(
        &self,
        model_id: &str,
        messages: Vec<ProcessedMessage>,
        parameter_overrides: Option<ModelParameters>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError> {
        let entry = self.get_provider(model_id)?;
        let parameters = match parameter_overrides {
            Some(overrides) => merge_parameters(&entry.parameters, &overrides),
            None => entry.parameters.clone(),
        };
        let request = CompletionRequest {
            model: entry.model_name.clone(),
            messages,
            parameters,
        };
        entry.provider.complete_stream(request).await
    }

    pub fn has_model(&self, model_id: &str) -> bool {
        self.inner.providers.contains_key(model_id)
    }

    pub fn get_available_models(&self) -> Vec<String> {
        self.inner.providers.keys().cloned().collect()
    }

    pub fn get_unavailable_models(&self) -> &HashMap<String, String> {
        &self.inner.unavailable_models
    }

    fn get_provider(&self, model_id: &str) -> Result<&ProviderEntry, AiError> {
        if let Some(reason) = self.inner.unavailable_models.get(model_id) {
            return Err(AiError::ModelUnavailable(reason.clone()));
        }
        self.inner
            .providers
            .get(model_id)
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))
    }
}

fn merge_parameters(base: &ModelParameters, overrides: &ModelParameters) -> ModelParameters {
    ModelParameters {
        temperature: overrides.temperature.or(base.temperature),
        max_tokens: overrides.max_tokens.or(base.max_tokens),
        top_p: overrides.top_p.or(base.top_p),
        frequency_penalty: overrides.frequency_penalty.or(base.frequency_penalty),
        presence_penalty: overrides.presence_penalty.or(base.presence_penalty),
        reasoning_effort: overrides.reasoning_effort.clone().or(base.reasoning_effort.clone()),
    }
}
